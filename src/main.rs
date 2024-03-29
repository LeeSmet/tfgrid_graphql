#![warn(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use clap::{Args, Parser, Subcommand, ValueEnum};

mod app;

// /// Amount of time to wait after a period for possible uptime events for minting purposes.
// const POST_PERIOD_UPTIME_FETCH: i64 = 3 * 60 * 60;
//
// /// Amount of seconds in an hour.
// const SECONDS_IN_HOUR: i64 = 3_600;
//
// /// Amount of the smallest on chain currency unit which equate 1 TFT. In other words, 1 TFT can be
// /// split up in this many pieces.
// const UNITS_PER_TFT: u64 = 10_000_000;
//
// /// Value of 1 KiB.
// const KIB: u64 = 1 << 10;
// /// Value of 1 MiB.
// const MIB: u64 = 1 << 20;
// /// Value of 1 GiB.
// const GIB: u64 = 1 << 30;
// /// Value of 1 TiB.
// const TIB: u64 = 1 << 40;
//
// /// The states of a contract which are considered to be active.
// const ACTIVE_CONTRACT_STATES: [ContractState; 2] =
//     [ContractState::Created, ContractState::GracePeriod];
// /// All contract states, this includes expired contract states.
// const ALL_STATES: [ContractState; 4] = [
//     ContractState::Created,
//     ContractState::GracePeriod,
//     ContractState::OutOfFunds,
//     ContractState::Deleted,
// ];

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
    Testnet,
    Qanet,
    Devnet,
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
        #[command(flatten)]
        filters: ContractFilters,
    },
    /// Calculate the total amount billed for the last hours
    TotalBilled {
        /// Amount of hours to get bills for
        hours: u32,
    },
}

#[derive(Args)]
/// Filter fields for listing contracts.
struct ContractFilters {
    /// Nodes for which to list contracts
    #[arg(short = 'n', long = "nodes")]
    node_ids: Option<Vec<u32>>,
    /// Twin ID's which must own the contracts
    #[arg(short = 't', long = "twins")]
    twin_ids: Option<Vec<u32>>,
    /// Contract ID's to list
    #[arg(short = 'c', long = "contracts")]
    contract_ids: Vec<u64>,
    /// Solution provider ID's for which to list contracts
    #[arg(short = 's', long)]
    solution_provider_ids: Vec<u32>,
    /// Include expired contracts as well
    #[arg(short = 'e', long)]
    include_expired: bool,
    /// Caluclate the total cost in TFT of all contracts. This might take a while
    ///
    /// This does not account for the variance in TFT price, and just shows the total amount of
    /// TFT billed over the life of the contract. Specifically, for longer running contracts,
    /// this might give a wrong idea of the average cost of the contract over time, as drops in
    /// TFT price will cause this amount to inflate, and similarly spikes in TFT price will
    /// cause this amount to deflate. As a result, this value is just informational.
    #[arg(long)]
    include_cost: bool,
    /// Calculate the total amount of public network used by the contract. This might take a
    /// while.
    #[arg(long)]
    include_network: bool,
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use eframe::NativeOptions;

    pretty_env_logger::init();

    let native_options = NativeOptions::default();
    eframe::run_native(
        "tfgrid_graphql",
        native_options,
        Box::new(|cc| Box::new(app::UiState::new(cc))),
    )
    // let cli = Cli::parse();

    // let client = match cli.network {
    //     Network::Mainnet => Client::mainnet()?,
    //     Network::Testnet => Client::testnet()?,
    //     Network::Qanet => Client::qanet()?,
    //     Network::Devnet => Client::devnet()?,
    // };

    // match cli.command {
    //     Commands::NodeState { node_id, period } => {
    //         calculate_node_states(client, node_id, Period::at_offset(period))?;
    //     }
    //     Commands::Contracts { filters } => {
    //         list_contracts(client, filters)?;
    //     }
    //     Commands::TotalBilled { hours } => {
    //         calculate_contract_bills(client, hours)?;
    //     }
    // };

    // Ok(())
    //
}

// When compiling to web:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "tfgrid_graphql_canvas", // hardcode it
                web_options,
                Box::new(|cc| Box::new(app::UiState::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}

//fn calculate_node_states(
//    client: Client,
//    node_id: u32,
//    period: Period,
//) -> Result<(), Box<dyn std::error::Error>> {
//    println!(
//        "Checking node state between {} and {}",
//        fmt_local_time(period.start()),
//        fmt_local_time(period.end())
//    );
//    println!("Fetching uptime events");
//    let uptimes = client.uptime_events(
//        node_id,
//        period.start(),
//        period.end() + POST_PERIOD_UPTIME_FETCH,
//    )?;
//
//    if uptimes.is_empty() {
//        println!("No uptime events found, node is down for the entire period");
//        return Ok(());
//    }
//
//    println!("Calculating node changes");
//    let node_states = calculate_node_state_changes(&uptimes, period.start(), period.end());
//    println!();
//
//    let mut state_table = Table::new();
//    state_table.set_titles(row![
//        l->"",
//        l->"Event",
//        l->"Event detected",
//    ]);
//    for ns in node_states {
//        let (emoji, msg) = node_state_formatted(ns.state());
//        state_table.add_row(row![
//            l->emoji,
//            l->msg,
//            l->fmt_local_time(ns.timestamp()),
//        ]);
//    }
//    let mut fmt = TableFormat::new();
//    fmt.padding(1, 1);
//    *state_table.get_format() = fmt;
//    state_table.printstd();
//    Ok(())
//}
//
//fn list_contracts(
//    client: Client,
//    filters: ContractFilters,
//) -> Result<(), Box<dyn std::error::Error>> {
//    println!("Fetching contracts");
//    let ContractFilters {
//        node_ids,
//        twin_ids,
//        contract_ids,
//        solution_provider_ids,
//        include_expired,
//        include_cost,
//        include_network,
//    } = filters;
//    let Contracts {
//        node_contracts,
//        name_contracts,
//        rent_contracts,
//    } = client.contracts(
//        node_ids.as_deref(),
//        if include_expired {
//            &ALL_STATES
//        } else {
//            &ACTIVE_CONTRACT_STATES
//        },
//        twin_ids.as_deref(),
//        &contract_ids,
//        &solution_provider_ids,
//    )?;
//    if node_contracts.is_empty() && name_contracts.is_empty() && rent_contracts.is_empty() {
//        println!();
//        println!("No contracts found for this query");
//        return Ok(());
//    }
//    let contract_ids = node_contracts
//        .iter()
//        .map(|c| c.contract_id)
//        .chain(name_contracts.iter().map(|c| c.contract_id))
//        .chain(rent_contracts.iter().map(|c| c.contract_id))
//        .collect::<Vec<_>>();
//    let mut contract_costs = if include_cost {
//        println!("Fetching contract bills");
//        client
//            .contract_bill_reports(None, None, &contract_ids)?
//            .into_iter()
//            .fold(HashMap::new(), |mut acc: HashMap<u64, u64>, value| {
//                *acc.entry(value.contract_id).or_default() += value.amount_billed;
//                acc
//            })
//    } else {
//        HashMap::new()
//    };
//    let mut network_usage = if include_network {
//        println!("Fetching NRU consumption reports");
//        client.nru_consumptions(&contract_ids)?.into_iter().fold(
//            HashMap::new(),
//            |mut acc: HashMap<u64, u64>, value| {
//                *acc.entry(value.contract_id).or_default() += value.nru;
//                acc
//            },
//        )
//    } else {
//        HashMap::new()
//    };
//    if !node_contracts.is_empty() {
//        let mut node_table = Table::new();
//        node_table.set_titles(row![
//            r->"Contract ID",
//            r->"Node ID",
//            r->"Owner",
//            r->"Solution Provider ID",
//            r->"Cru",
//            r->"Mru",
//            r->"Sru",
//            r->"Hru",
//            r->"Nru",
//            r->"Public IPs",
//            r->"Total Cost",
//            r->"Deployment Hash",
//            r->"Deployment Data",
//            r->"Created",
//            r->"State"
//        ]);
//        for contract in node_contracts {
//            node_table.add_row(row![
//                r->contract.contract_id,
//                r->contract.node_id,
//                r->contract.twin_id,
//                r->if let Some(spid) = contract.solution_provider_id {
//                    format!("{spid}")
//                } else {
//                    "-".to_string()
//                },
//                r->if let Some(ref r) = contract.resources_used {
//                    format!("{}", r.cru)
//                } else {
//                    "-".to_string()
//                },
//                r->if let Some(ref r) = contract.resources_used {
//                    fmt_resources(r.mru)
//                } else {
//                    "-".to_string()
//                },
//                r->if let Some(ref r) = contract.resources_used {
//                    fmt_resources(r.sru)
//                } else {
//                    "-".to_string()
//                },
//                r->if let Some(ref r) = contract.resources_used {
//                    fmt_resources(r.hru)
//                } else {
//                    "-".to_string()
//                },
//                r->fmt_resources(network_usage.remove(&contract.contract_id).unwrap_or_default()),
//                r->contract.number_of_public_ips,
//                r->fmt_tft(contract_costs.remove(&contract.contract_id).unwrap_or_default()),
//                r->contract.deployment_hash,
//                r->fmt_deployemnt_data(contract.deployment_data),
//                r->fmt_local_time(contract.created_at),
//                r->contract.state,
//            ]);
//        }
//        node_table.printstd();
//    }
//    if !name_contracts.is_empty() {
//        let mut name_table = Table::new();
//        name_table.set_titles(row![
//            r->"Contract ID",
//            r->"Owner",
//            r->"Solution Provider ID",
//            r->"Name",
//            r->"Nru",
//            r->"Total Cost",
//            r->"Created",
//            r->"State"
//        ]);
//        for contract in name_contracts {
//            name_table.add_row(row![
//                r->contract.contract_id,
//                r->contract.twin_id,
//                r->if let Some(spid) = contract.solution_provider_id {
//                    format!("{spid}")
//                } else {
//                    "-".to_string()
//                },
//                r->contract.name,
//                r->fmt_resources(network_usage.remove(&contract.contract_id).unwrap_or_default()),
//                r->fmt_tft(contract_costs.remove(&contract.contract_id).unwrap_or_default()),
//                r->fmt_local_time(contract.created_at),
//                r->contract.state,
//            ]);
//        }
//        name_table.printstd();
//    }
//    if !rent_contracts.is_empty() {
//        let mut rent_table = Table::new();
//        rent_table.set_titles(row![
//            r->"Contract ID",
//            r->"Node ID",
//            r->"Owner",
//            r->"Solution Provider ID",
//            r->"Total Cost",
//            r->"Created",
//            r->"State"
//        ]);
//        for contract in rent_contracts {
//            rent_table.add_row(row![
//                r->contract.contract_id,
//                r->contract.node_id,
//                r->contract.twin_id,
//                r->if let Some(spid) = contract.solution_provider_id {
//                    format!("{spid}")
//                } else {
//                    "-".to_string()
//                },
//                r->fmt_tft(contract_costs.remove(&contract.contract_id).unwrap_or_default()),
//                r->fmt_local_time(contract.created_at),
//                r->contract.state,
//            ]);
//        }
//        rent_table.printstd();
//    }
//    Ok(())
//}
//
//fn calculate_contract_bills(client: Client, hours: u32) -> Result<(), Box<dyn std::error::Error>> {
//    println!("Calculating amount of tokens billed for the last {hours} hours");
//    println!("Fetching bill events");
//    let now = SystemTime::now()
//        .duration_since(SystemTime::UNIX_EPOCH)?
//        .as_secs() as i64;
//    let start = now - SECONDS_IN_HOUR * hours as i64;
//    let bills = client.contract_bill_reports(Some(start), Some(now), &[])?;
//    println!("Calculate total bill cost");
//    println!();
//
//    println!(
//        "Total billed from {} to {}: ",
//        fmt_local_time(start),
//        fmt_local_time(now)
//    );
//    let total: u64 = bills.into_iter().map(|bill| bill.amount_billed).sum();
//    println!("\t{}", fmt_tft(total));
//    Ok(())
//}
//
//fn node_state_formatted(state: NodeState) -> (char, String) {
//    match state {
//        NodeState::Offline(ts) => (
//            DOWN_ARROW_EMOJI,
//            format!("Node went down at {}", fmt_local_time(ts)),
//        ),
//        NodeState::Booted(ts) => (
//            UP_ARROW_EMOJI,
//            format!("Node booted at {}", fmt_local_time(ts),),
//        ),
//        NodeState::ImpossibleReboot(ts) => (
//            BOOM_EMOJI,
//            format!(
//                "Supposed boot at {} which conflicts with other info",
//                fmt_local_time(ts),
//            ),
//        ),
//        NodeState::Drift(drift) => (
//            CLOCK_EMOJI,
//            format!("Uptime drift of {drift} seconds detected"),
//        ),
//        NodeState::Unknown(since) => (
//            QUESTION_MARK_EMOJI,
//            format!(
//                "Node status is unknown since {}, presumed down",
//                fmt_local_time(since),
//            ),
//        ),
//    }
//}
//
//fn fmt_local_time(ts: i64) -> String {
//    Local
//        .timestamp_opt(ts, 0)
//        .single()
//        .expect("Local time from timestamp is unambiguous")
//        .format("%d/%m/%Y %H:%M:%S")
//        .to_string()
//}
//
///// Format a raw byte value as more human readable item.
//fn fmt_resources(value: u64) -> String {
//    match value {
//        v if v > TIB => format!("{:.2} TiB", value as f64 / TIB as f64),
//        v if v > GIB => format!("{:.2} GiB", value as f64 / GIB as f64),
//        v if v > MIB => format!("{:.2} MiB", value as f64 / MIB as f64),
//        v if v > KIB => format!("{:.2} KiB", value as f64 / KIB as f64),
//        v => format!("{v} B"),
//    }
//}
//
///// Format deployment data, only retraining the first portion.
//fn fmt_deployemnt_data(mut data: String) -> String {
//    if data.len() > 30 {
//        // FIXME
//        data = data[..30].to_string();
//        data.push_str("...");
//    }
//
//    data
//}
//
///// Format an amount as value in TFT
//fn fmt_tft(amount: u64) -> String {
//    format!("{}.{} TFT", amount / UNITS_PER_TFT, amount % UNITS_PER_TFT)
//}
