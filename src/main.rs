use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand, ValueEnum};
use std::time::SystemTime;
use tfgrid_graphql::{
    graphql::Client,
    period::Period,
    uptime::{calculate_node_state_changes, NodeState},
};

/// Amount of time to wait after a period for possible uptime events for minting purposes.
const POST_PERIOD_UPTIME_FETCH: i64 = 3 * 60 * 60;

/// Amount of seconds in an hour.
const SECONDS_IN_HOUR: i64 = 3_600;

const UNITS_PER_TFT: u64 = 10_000_000;

/// Emoji for node boot.
const UP_ARROW_EMOJI: char = '🡅';
/// Emoji for node going offline.
const DOWN_ARROW_EMOJI: char = '🡇';
/// Emoji for impossible reboot.
const BOOM_EMOJI: char = '💥';
/// Emoji for node uptime drift.
const CLOCK_EMOJI: char = '🕑';
/// Emoji for unknown state.
const QUESTION_MARK_EMOJI: char = '❓';

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_enum, default_value_t = Network::Mainnet)]
    network: Network,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum Network {
    Mainnet,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate the state changes of a node
    NodeState {
        /// The id of the node for which to check the node state
        node_id: u32,
        /// The period for which to check the uptime
        period: i64,
    },
    /// Calculate the total amount billed for the last hours
    TotalBilled {
        /// Amount of hours to get bills for
        hours: u32,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let client = match cli.network {
        Network::Mainnet => Client::mainnet()?,
    };

    match cli.command {
        Commands::NodeState { node_id, period } => {
            calculate_node_states(client, node_id, Period::at_offset(period))?;
        }
        Commands::TotalBilled { hours } => {
            calculate_contract_bills(client, hours)?;
        }
    };

    Ok(())
}

fn calculate_node_states(
    client: Client,
    node_id: u32,
    period: Period,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Checking node state between {} and {}",
        fmt_local_time(period.start()),
        fmt_local_time(period.end())
    );
    println!("Fetching uptime events");
    let uptimes = client.uptime_events(
        node_id,
        period.start(),
        period.end() + POST_PERIOD_UPTIME_FETCH,
    )?;

    if uptimes.is_empty() {
        println!("No uptime events found, node is down for the entire period");
        return Ok(());
    }

    println!("Calculating node changes");
    let node_states = calculate_node_state_changes(&uptimes, period.start(), period.end());
    println!();

    println!(
        "   Event                                                                 Event detected"
    );
    for ns in node_states {
        let (emoji, msg) = node_state_formatted(ns.state());
        println!("{:<2}{:<70}{}", emoji, msg, fmt_local_time(ns.timestamp()),);
    }
    Ok(())
}

fn calculate_contract_bills(client: Client, hours: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!("Calculating amount of tokens billed for the last {hours} hours");
    println!("Fetching bill events");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as i64;
    let start = now - SECONDS_IN_HOUR * hours as i64;
    let bills = client.contract_bill_reports(start, now)?;
    println!("Calculate total bill cost");
    println!();

    println!(
        "Total billed from {} to {}: ",
        fmt_local_time(start),
        fmt_local_time(now)
    );
    let total: u64 = bills.into_iter().map(|bill| bill.amount_billed).sum();
    println!("\t{}.{} TFT", total / UNITS_PER_TFT, total % UNITS_PER_TFT);
    Ok(())
}

fn node_state_formatted(state: NodeState) -> (char, String) {
    match state {
        NodeState::Offline(ts) => (
            DOWN_ARROW_EMOJI,
            format!("Node went down at {}", fmt_local_time(ts)),
        ),
        NodeState::Booted(ts) => (
            UP_ARROW_EMOJI,
            format!("Node booted at {}", fmt_local_time(ts),),
        ),
        NodeState::ImpossibleReboot(ts) => (
            BOOM_EMOJI,
            format!(
                "Supposed boot at {} which conflicts with other info",
                fmt_local_time(ts),
            ),
        ),
        NodeState::Drift(drift) => (
            CLOCK_EMOJI,
            format!("Uptime drift of {drift} seconds detected"),
        ),
        NodeState::Unknown(since) => (
            QUESTION_MARK_EMOJI,
            format!(
                "Node status is unknown since {}, presumed down",
                fmt_local_time(since),
            ),
        ),
    }
}

fn fmt_local_time(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .expect("Local time from timestamp is unambiguous")
        .format("%d/%m/%Y %H:%M:%S")
        .to_string()
}
