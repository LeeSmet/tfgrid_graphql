use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand, ValueEnum};
use prettytable::{row, Table};
use std::{collections::HashMap, time::SystemTime};
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

/// Value of 1 KiB.
const KIB: u64 = 1 << 10;
/// Value of 1 MiB.
const MIB: u64 = 1 << 20;
/// Value of 1 GiB.
const GIB: u64 = 1 << 30;
/// Value of 1 TiB.
const TIB: u64 = 1 << 40;

/// The states of a contract which are considered to be active.
const ACTIVE_CONTRACT_STATES: [ContractState; 2] =
    [ContractState::Created, ContractState::GracePeriod];
/// All contract states, this includes expired contract states.
const ALL_STATES: [ContractState; 4] = [
    ContractState::Created,
    ContractState::GracePeriod,
    ContractState::OutOfFunds,
    ContractState::Deleted,
];

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
    /// List contracts with given parameters
    ///
    /// All provided filters apply to the result at the same time, e.g. if both nodes and twins are
    /// set, only contracts deployed on the given nodes by the given twins will be returned, but
    /// contracts on the given nodes by different twins, or contracts deployed by the twins on
    /// different nodes will be excluded.
    Contracts {
        /// Include expired contracts as well
        #[arg(short = 'e', long)]
        include_expired: bool,
        /// Nodes for which to list contracts
        #[arg(short = 'n')]
        nodes: Option<Vec<u32>>,
        /// Twin ID's which must own the contracts
        #[arg(short = 't')]
        twins: Option<Vec<u32>>,
        /// Caluclate the total cost in TFT of all contracts. This might take a while
        ///
        /// This does not account for the variance in TFT price, and just shows the total amount of
        /// TFT billed over the life of the contract. Specifically, for longer running contracts,
        /// this might give a wrong idea of the average cost of the contract over time, as drops in
        /// TFT price will cause this amount to inflate, and similarly spikes in TFT price will
        /// cause this amount to deflate. As a result, this value is just informational.
        #[arg(short = 'c', long)]
        include_cost: bool,
        /// Calculate the total amount of public network used by the contract. This might take a
        /// while.
        #[arg(long)]
        include_network: bool,
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
        Commands::Contracts {
            include_expired,
            nodes,
            twins,
            include_cost,
            include_network,
        } => {
            list_contracts(
                client,
                nodes,
                twins,
                include_expired,
                include_cost,
                include_network,
            )?;
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

fn list_contracts(
    client: Client,
    node_ids: Option<Vec<u32>>,
    twin_ids: Option<Vec<u32>>,
    include_expired: bool,
    include_cost: bool,
    include_network: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching contracts");
    let contracts = client.contracts(
        node_ids.as_deref(),
        if include_expired {
            &ALL_STATES
        } else {
            &ACTIVE_CONTRACT_STATES
        },
        twin_ids.as_deref(),
    )?;
    if contracts.is_empty() {
        println!();
        println!("No contracts found for this/these nodes");
        return Ok(());
    }
    let contract_ids = contracts.iter().map(|c| c.contract_id).collect::<Vec<_>>();
    let mut contract_costs = if include_cost {
        println!("Fetching contract bills");
        client
            .contract_bill_reports(None, None, &contract_ids)?
            .into_iter()
            .fold(HashMap::new(), |mut acc: HashMap<u64, u64>, value| {
                *acc.entry(value.contract_id).or_default() += value.amount_billed;
                acc
            })
    } else {
        HashMap::new()
    };
    let mut network_usage = if include_network {
        println!("Fetching NRU consumption reports");
        client.nru_consumptions(&contract_ids)?.into_iter().fold(
            HashMap::new(),
            |mut acc: HashMap<u64, u64>, value| {
                *acc.entry(value.contract_id).or_default() += value.nru;
                acc
            },
        )
    } else {
        HashMap::new()
    };
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
        r->"Nru",
        r->"Public IPs",
        r->"Total Cost",
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
                fmt_resources(r.mru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_resources(r.sru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_resources(r.hru)
            } else {
                "-".to_string()
            },
            r->fmt_resources(network_usage.remove(&contract.contract_id).unwrap_or_default()),
            r->contract.number_of_public_ips,
            r->fmt_tft(contract_costs.remove(&contract.contract_id).unwrap_or_default()),
            r->contract.deployment_hash,
            r->fmt_deployemnt_data(contract.deployment_data),
            r->fmt_local_time(contract.created_at / 1000),
            r->contract.state,
        ]);
    }
    table.printstd();
    Ok(())
}
fn list_node_contracts(
    client: Client,
    node_ids: Vec<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching contracts");
    let contracts = client.contracts(Some(&node_ids), &ACTIVE_CONTRACT_STATES, None)?;
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
                fmt_resources(r.mru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_resources(r.sru)
            } else {
                "-".to_string()
            },
            r->if let Some(ref r) = contract.resources_used {
                fmt_resources(r.hru)
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
    let bills = client.contract_bill_reports(Some(start), Some(now), &[])?;
    println!("Calculate total bill cost");
    println!();

    println!(
        "Total billed from {} to {}: ",
        fmt_local_time(start),
        fmt_local_time(now)
    );
    let total: u64 = bills.into_iter().map(|bill| bill.amount_billed).sum();
    println!("\t{}", fmt_tft(total));
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

/// Format a raw byte value as more human readable item.
fn fmt_resources(value: u64) -> String {
    match value {
        v if v > TIB => format!("{:.2} TiB", value as f64 / TIB as f64),
        v if v > GIB => format!("{:.2} GiB", value as f64 / GIB as f64),
        v if v > MIB => format!("{:.2} MiB", value as f64 / MIB as f64),
        v if v > KIB => format!("{:.2} KiB", value as f64 / KIB as f64),
        v => format!("{} B", v),
    }
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

/// Format an amount as value in TFT
fn fmt_tft(amount: u64) -> String {
    format!("{}.{} TFT", amount / UNITS_PER_TFT, amount % UNITS_PER_TFT)
}
