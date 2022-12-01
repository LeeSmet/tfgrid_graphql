use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand, ValueEnum};
use prettytable::{row, Table};
use std::time::SystemTime;
use tfgrid_graphql::{
    contract::ContractState,
    graphql::Client,
    period::Period,
    uptime::{calculate_node_state_changes, NodeState},
};

/// Amount of time to wait after a period for possible uptime events for minting purposes.
const POST_PERIOD_UPTIME_FETCH: i64 = 3 * 60 * 60;

/// Amount of seconds in an hour.
const SECONDS_IN_HOUR: i64 = 3_600;

/// Amount of the smallest on chain currency unit which equate 1 TFT. In other words, 1 TFT can be
/// split up in this many pieces.
const UNITS_PER_TFT: u64 = 10_000_000;

/// Value of 1 GiB.
const GIB: u64 = 1 << 30;

/// The states of a contract which are considered to be active.
const ACTIVE_CONTRACT_STATES: [ContractState; 2] =
    [ContractState::Created, ContractState::GracePeriod];

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
    /// List the active contracts on one or more nodes
    NodeContracts {
        /// The node ids for which to list the contracts
        node_ids: Vec<u32>,
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
        Commands::NodeContracts { node_ids } => {
            list_node_contracts(client, node_ids)?;
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

fn list_node_contracts(
    client: Client,
    node_ids: Vec<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching contracts");
    let contracts = client.node_contracts(&node_ids, &ACTIVE_CONTRACT_STATES)?;
    if contracts.is_empty() {
        println!();
        println!("No contracts found for this/these nodes");
    }
    let mut table = Table::new();
    table.set_titles(row![
        r->"Contract ID",
        r->"Node ID",
        r->"Owner",
        r->"Solution Provider ID",
        r->"Cru",
        r->"Mru",
        r->"Sru",
        r->"Hru",
        r->"Public IPs",
        r->"Deployment Hash",
        r->"Deployment Data",
        r->"Created",
        r->"State"
    ]);
    for contract in contracts {
        table.add_row(row![
            r->contract.contract_id,
            r->contract.node_id,
            r->contract.twin_id,
            r->if let Some(spid) = contract.solution_provider_id {
                format!("{spid}")
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                format!("{}", r.cru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_gib(r.mru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_gib(r.sru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_gib(r.hru)
            } else {
                "-".to_string()
            },
            r->contract.number_of_public_ips,
            r->contract.deployment_hash,
            r->fmt_deployemnt_data(contract.deployment_data),
            r->fmt_local_time(contract.created_at / 1000),
            r->contract.state,
        ]);
    }
    table.printstd();
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

/// Format a raw byte value as GiB.
fn fmt_gib(value: u64) -> String {
    format!("{:.2} GiB", value as f64 / GIB as f64)
}

/// Format deployment data, only retraining the first portion.
fn fmt_deployemnt_data(mut data: String) -> String {
    if data.len() > 30 {
        // FIXME
        data = data[..30].to_string();
        data.push_str("...");
    }

    data
}
