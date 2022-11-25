use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand, ValueEnum};
use tfgrid_graphql::{
    graphql::Client,
    period::Period,
    uptime::{calculate_node_state_changes, NodeState},
};

/// Amount of time to wait after a period for possible uptime events for minting purposes.
const POST_PERIOD_UPTIME_FETCH: i64 = 3 * 60 * 60;

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
    NodeState {
        /// The id of the node for which to check the node state
        node_id: u32,
        /// The period for which to check the uptime
        period: i64,
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

    for ns in node_states {
        let (emoji, msg) = node_state_formatted(ns.state());
        println!(
            "{:<2} {:<50} {}",
            emoji,
            msg,
            fmt_local_time(ns.timestamp()),
        );
    }
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
        NodeState::Unknown => (
            QUESTION_MARK_EMOJI,
            "Node status is unknown, presumed down".to_string(),
        ),
    }
}

fn fmt_local_time(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .expect("Can format local timezone timestamp")
        .format("%d/%m/%Y %H:%M:%S")
        .to_string()
}
