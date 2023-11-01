use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use chrono::{Local, NaiveDate, TimeZone};
use eframe::{
    egui::{
        self,
        plot::{Legend, Line, Plot, PlotPoints},
        Layout, Widget,
    },
    emath::Align,
    App,
};
use egui_extras::{Column, TableBuilder};
use poll_promise::Promise;
use tfgrid_graphql::{
    bill_report::ContractBillReport,
    contract::{ContractState, NameContract, NodeContract, RentContract},
    graphql::Contracts,
    uptime::{calculate_node_state_changes, NodeState, NodeStateChange, UptimeEvent},
};

pub struct UiState {
    client: tfgrid_graphql::graphql::Client,
    selected: MenuSelection,
    contract_overview: ContractOverviewPanel,
    node_state: NodeStatePanel,
    total_billed_state: TotalBilledPanel,
}

/// State for the contract overview panel
struct ContractOverviewPanel {
    node_id_input: String,
    twin_id_input: String,
    contract_id_input: String,
    node_ids: BTreeSet<u32>,
    twin_ids: BTreeSet<u32>,
    contract_ids: BTreeSet<u64>,
    node_id_error: String,
    twin_id_error: String,
    contract_id_error: String,
    contract_loading: Option<Promise<Result<Contracts, String>>>,
    node_nru_loads: Vec<Option<Promise<Result<u64, String>>>>,
    name_nru_loads: Vec<Option<Promise<Result<u64, String>>>>,
    node_price_loads: Vec<Option<Promise<Result<u64, String>>>>,
    name_price_loads: Vec<Option<Promise<Result<u64, String>>>>,
    rent_price_loads: Vec<Option<Promise<Result<u64, String>>>>,
    trigger_loads: bool,
}

/// helper type to avoid overly complex expressions.
// TODO: translate this to struct
type NodeStateInfo = (Vec<UptimeEvent>, Vec<NodeStateChange>);

/// State for the node state panel
struct NodeStatePanel {
    node_id_input: String,
    node_id_error: String,
    node_id: Option<u32>,
    range_start: chrono::NaiveDate,
    range_end: chrono::NaiveDate,
    node_loading: Option<Promise<Result<NodeStateInfo, String>>>,
}

type BillHistory = Vec<ContractBillReport>;

/// State for the total billed panel
struct TotalBilledPanel {
    hours_input: String,
    hours_error: String,
    hours: Option<usize>,
    bills_loading: Option<Vec<Promise<Result<BillHistory, String>>>>,
}

impl UiState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        log::debug!("{:?}", cc.integration_info);

        Self {
            client: tfgrid_graphql::graphql::Client::mainnet().expect("can initiate client, TODO"),
            selected: MenuSelection::ContractOverview,
            contract_overview: ContractOverviewPanel {
                node_id_input: String::new(),
                twin_id_input: String::new(),
                contract_id_input: String::new(),
                node_ids: BTreeSet::new(),
                twin_ids: BTreeSet::new(),
                contract_ids: BTreeSet::new(),
                node_id_error: String::new(),
                twin_id_error: String::new(),
                contract_id_error: String::new(),
                contract_loading: None,
                node_nru_loads: Vec::new(),
                name_nru_loads: Vec::new(),
                node_price_loads: Vec::new(),
                name_price_loads: Vec::new(),
                rent_price_loads: Vec::new(),
                trigger_loads: false,
            },
            node_state: NodeStatePanel {
                node_id_input: String::new(),
                node_id_error: String::new(),
                node_id: None,
                range_start: chrono::NaiveDate::default(),
                range_end: chrono::NaiveDate::default(),
                node_loading: None,
            },
            total_billed_state: TotalBilledPanel {
                hours_input: String::new(),
                hours_error: String::new(),
                hours: None,
                bills_loading: None,
            },
        }
    }
}

impl App for UiState {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let Self {
            client,
            selected,
            contract_overview,
            node_state,
            total_billed_state,
        } = self;

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label("CPU Usage:");
                    ui.label(format!("{:.2} %", frame.info().cpu_usage.unwrap_or(0.)));
                    ui.label("|");
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        egui::SidePanel::left("menu").show(ctx, |ui| {
            ui.heading("Menu");
            // todo
            for me in [
                MenuSelection::ContractOverview,
                MenuSelection::ContractDetails,
                MenuSelection::NodeState,
                MenuSelection::TotalBilled,
            ] {
                if ui
                    .add(egui::SelectableLabel::new(selected == &me, me.to_string()))
                    .clicked()
                {
                    *selected = me;
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match selected {
                MenuSelection::ContractOverview => {
                    let ContractOverviewPanel {
                        node_id_input,
                        twin_id_input,
                        contract_id_input,
                        node_ids,
                        twin_ids,
                        contract_ids,
                        node_id_error,
                        twin_id_error,
                        contract_id_error,
                        contract_loading,
                        node_nru_loads,
                        name_nru_loads,
                        node_price_loads,
                        name_price_loads,
                        rent_price_loads,
                        trigger_loads,
                    } = contract_overview;
                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        // Input elements
                        ui_multi_input(ui, "Node ID:", node_id_error, node_id_input, node_ids);
                        ui_multi_input(ui, "Twin ID:", twin_id_error, twin_id_input, twin_ids);
                        ui_multi_input(
                            ui,
                            "Contract ID:",
                            contract_id_error,
                            contract_id_input,
                            contract_ids,
                        );
                        if ui.button("Search").clicked() {
                            let loading = if let Some(promise) = contract_loading {
                                promise.ready().is_none()
                            } else {
                                false
                            };

                            if !loading {
                                let client = client.clone();
                                let node_ids = node_ids.iter().copied().collect::<Vec<_>>();
                                let twin_ids = twin_ids.iter().copied().collect::<Vec<_>>();
                                let contract_ids = contract_ids.iter().copied().collect::<Vec<_>>();
                                *contract_loading = Some(Promise::spawn_async(async move {
                                    client
                                        .contracts(
                                            if node_ids.is_empty() {
                                                None
                                            } else {
                                                Some(&node_ids)
                                            },
                                            // Static filter for now
                                            &[
                                                ContractState::Created,
                                                ContractState::GracePeriod,
                                                ContractState::OutOfFunds,
                                            ],
                                            if twin_ids.is_empty() {
                                                None
                                            } else {
                                                Some(&twin_ids)
                                            },
                                            &contract_ids,
                                            &[],
                                        )
                                        .await
                                }));
                                *trigger_loads = true;
                            }
                        }

                        if let Some(cl) = contract_loading {
                            match cl.ready() {
                                // todo
                                None => {
                                    ui.with_layout(
                                        Layout::centered_and_justified(egui::Direction::TopDown),
                                        |ui| {
                                            ui.spinner();
                                        },
                                    );
                                }
                                Some(Err(err)) => {
                                    ui.colored_label(ui.visuals().error_fg_color, err);
                                }
                                Some(Ok(contracts)) => {
                                    if *trigger_loads {
                                        *node_nru_loads =
                                            Vec::with_capacity(contracts.node_contracts.len());
                                        for _ in 0..contracts.node_contracts.len() {
                                            node_nru_loads.push(None);
                                        }
                                        *name_nru_loads =
                                            Vec::with_capacity(contracts.name_contracts.len());
                                        for _ in 0..contracts.name_contracts.len() {
                                            name_nru_loads.push(None);
                                        }
                                        *node_price_loads =
                                            Vec::with_capacity(contracts.node_contracts.len());
                                        for _ in 0..contracts.node_contracts.len() {
                                            node_price_loads.push(None);
                                        }
                                        *name_price_loads =
                                            Vec::with_capacity(contracts.name_contracts.len());
                                        for _ in 0..contracts.name_contracts.len() {
                                            name_price_loads.push(None);
                                        }
                                        *rent_price_loads =
                                            Vec::with_capacity(contracts.rent_contracts.len());
                                        for _ in 0..contracts.rent_contracts.len() {
                                            rent_price_loads.push(None);
                                        }
                                        *trigger_loads = false;
                                    }
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        let nru_loader = |contract_id| {
                                            let client = client.clone();
                                            move || {
                                                Promise::spawn_async(async move {
                                                    Ok(client
                                                        .nru_consumptions(&[contract_id])
                                                        .await?
                                                        .into_iter()
                                                        .map(|nru_consumption| nru_consumption.nru)
                                                        .sum())
                                                })
                                            }
                                        };
                                        let cost_loader = |contract_id| {
                                            let client = client.clone();
                                            move || {
                                                Promise::spawn_async(async move {
                                                    Ok(client
                                                        .contract_bill_reports(
                                                            None,
                                                            None,
                                                            &[contract_id],
                                                        )
                                                        .await?
                                                        .into_iter()
                                                        .map(|bill| bill.amount_billed)
                                                        .sum())
                                                })
                                            }
                                        };
                                        ui.collapsing("Node contracts", |ui| {
                                            ui_node_contracts(
                                                ui,
                                                &contracts.node_contracts,
                                                node_nru_loads,
                                                node_price_loads,
                                                nru_loader,
                                                cost_loader,
                                            );
                                        });
                                        ui.collapsing("Name contracts", |ui| {
                                            ui_name_contracts(
                                                ui,
                                                &contracts.name_contracts,
                                                name_nru_loads,
                                                name_price_loads,
                                                nru_loader,
                                                cost_loader,
                                            );
                                        });
                                        ui.collapsing("Rent contracts", |ui| {
                                            ui_rent_contracts(
                                                ui,
                                                &contracts.rent_contracts,
                                                rent_price_loads,
                                                cost_loader,
                                            );
                                        });
                                    });
                                }
                            }
                        }
                    });
                }
                MenuSelection::NodeState => {
                    let NodeStatePanel {
                        node_id_input,
                        node_id_error,
                        node_id,
                        range_start,
                        range_end,
                        node_loading,
                    } = node_state;
                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        // Input elements
                        ui_single_input(ui, "Node ID:", node_id_error, node_id_input, node_id);
                        ui.horizontal(|ui| {
                            ui.label("Range start:");
                            egui_extras::DatePickerButton::new(range_start)
                                .id_source("start_range")
                                .ui(ui);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Range end:  ");
                            egui_extras::DatePickerButton::new(range_end)
                                .id_source("end_range")
                                .ui(ui);
                        });
                        // only enable button if the node id input field contains something valid
                        if ui
                            .add_enabled(node_id.is_some(), egui::Button::new("Search"))
                            .clicked()
                        {
                            let loading = if let Some(promise) = node_loading {
                                promise.ready().is_none()
                            } else {
                                false
                            };
                            if !loading {
                                let client = client.clone();
                                let start = range_start
                                    .signed_duration_since(NaiveDate::default())
                                    .num_seconds();
                                let end = range_end
                                    .signed_duration_since(NaiveDate::default())
                                    .num_seconds();
                                // we can only reach here if the button is enabled, thus node_id
                                // is set.
                                let node_id = *node_id.as_ref().unwrap();
                                *node_loading = Some(Promise::spawn_async(async move {
                                    let uptimes = client.uptime_events(node_id, start, end).await?;
                                    let node_states =
                                        calculate_node_state_changes(&uptimes, start, end);
                                    Ok((uptimes, node_states))
                                }));
                            }
                        }

                        if let Some(cl) = node_loading {
                            match cl.ready() {
                                // todo
                                None => {
                                    ui.with_layout(
                                        Layout::centered_and_justified(egui::Direction::TopDown),
                                        |ui| {
                                            ui.spinner();
                                        },
                                    );
                                }
                                Some(Err(err)) => {
                                    ui.colored_label(ui.visuals().error_fg_color, err);
                                }
                                Some(Ok((uptime_events, state_changes))) => {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        ui.collapsing("Node state changes", |ui| {
                                            ui_node_state_changes(ui, state_changes);
                                        });
                                        ui.collapsing("Uptime event jitter", |ui| {
                                            ui_node_jitter_graph(ui, uptime_events);
                                        });
                                    });
                                }
                            }
                        }
                    });
                }
                MenuSelection::TotalBilled => {
                    let TotalBilledPanel {
                        hours_input,
                        hours_error,
                        hours,
                        bills_loading,
                    } = total_billed_state;
                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        // Input elements
                        ui_single_input(ui, "Hours to check:", hours_input, hours_error, hours);
                        if ui
                            .add_enabled(hours.is_some(), egui::Button::new("Calculate"))
                            .clicked()
                        {
                            let loading = if let Some(promises) = bills_loading {
                                promises.iter().any(|p| p.ready().is_none())
                            } else {
                                false
                            };
                            if !loading {
                                let client = client.clone();

                                let hours = *hours.as_ref().unwrap();
                                let end = chrono::offset::Local::now().timestamp();
                                //let start = end - hours as i64 * 3600;

                                *bills_loading = Some({
                                    // load bill reports individually per hour
                                    let mut promises = Vec::with_capacity(hours + 1);
                                    for i in 0..hours {
                                        let client = client.clone();
                                        promises.push(Promise::spawn_async(async move {
                                            let bills = client
                                                .clone()
                                                .contract_bill_reports(
                                                    Some(end - (3600 * (i + 1)) as i64),
                                                    Some(end - (3600 * i) as i64),
                                                    &[],
                                                )
                                                .await?;
                                            Ok(bills)
                                        }));
                                    }

                                    promises
                                });
                            }
                        }

                        if let Some(promises) = bills_loading {
                            let mut err = None;
                            let mut one_ready = false;
                            let mut ready_vals = Vec::with_capacity(promises.len());
                            for promise in promises {
                                match promise.ready() {
                                    None => {
                                        continue;
                                    }
                                    Some(Err(e)) => {
                                        err = Some(e);
                                        break;
                                    }
                                    Some(Ok(value)) => {
                                        one_ready = true;
                                        ready_vals.push(value);
                                    }
                                }
                            }

                            if let Some(e) = err {
                                ui.colored_label(ui.visuals().error_fg_color, e);
                            } else if !one_ready {
                                ui.with_layout(
                                    Layout::centered_and_justified(egui::Direction::TopDown),
                                    |ui| {
                                        ui.spinner();
                                    },
                                );
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui_bill_graph(
                                        ui,
                                        &ready_vals
                                            .into_iter()
                                            .flatten()
                                            .copied()
                                            .collect::<Vec<ContractBillReport>>(),
                                    );
                                });
                            }
                        }
                    });
                }
                _ => (),
            }
        });
    }
}

fn ui_node_contracts<C, N>(
    ui: &mut egui::Ui,
    node_contracts: &[NodeContract],
    nru_loads: &mut [Option<Promise<Result<u64, String>>>],
    node_price_loads: &mut [Option<Promise<Result<u64, String>>>],
    nru_loader: impl Fn(u64) -> N,
    cost_loader: impl Fn(u64) -> C,
) where
    N: FnOnce() -> Promise<Result<u64, String>>,
    C: FnOnce() -> Promise<Result<u64, String>>,
{
    egui::ScrollArea::horizontal().show(ui, |ui| {
        TableBuilder::new(ui)
            .cell_layout(Layout::centered_and_justified(egui::Direction::LeftToRight))
            .columns(Column::auto().resizable(true).clip(false), 14)
            .column(Column::remainder().clip(false).at_most(100.))
            .striped(true)
            .header(50.0, |mut header| {
                for title in [
                    "Contract ID",
                    "Node ID",
                    "Twin ID",
                    "Solution Provider ID",
                    "Cru",
                    "Mru",
                    "Sru",
                    "Hru",
                    "Nru",
                    "Public IPs",
                    "Total Cost",
                    "Deployment Hash",
                    "Deployment Data",
                    "Created",
                    "State",
                ] {
                    header.col(|ui| {
                        ui.heading(title);
                    });
                }
            })
            .body(|body| {
                body.rows(30.0, node_contracts.len(), |row_idx, mut row| {
                    let contract = &node_contracts[row_idx];
                    row.col(|ui| {
                        ui.label(format!("{}", contract.contract_id));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.node_id));
                    });
                    row.col(|ui| {
                        if ui.label(format!("{}", contract.twin_id)).hovered() {
                            egui::show_tooltip(
                                ui.ctx(),
                                egui::Id::new("contract_twin_id_tooltip"),
                                |ui| {
                                    ui.label(format!(
                                        "This contract is created and owned by twin {}",
                                        contract.twin_id
                                    ));
                                },
                            );
                        };
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.solution_provider_id.unwrap_or(0)));
                    });
                    row.col(|ui| {
                        ui.label(if let Some(ref res) = contract.resources_used {
                            format!("{}", res.cru)
                        } else {
                            "-".to_string()
                        });
                    });
                    row.col(|ui| {
                        ui.label(if let Some(ref res) = contract.resources_used {
                            fmt_resources(res.mru)
                        } else {
                            "-".to_string()
                        });
                    });
                    row.col(|ui| {
                        ui.label(if let Some(ref res) = contract.resources_used {
                            fmt_resources(res.sru)
                        } else {
                            "-".to_string()
                        });
                    });
                    row.col(|ui| {
                        ui.label(if let Some(ref res) = contract.resources_used {
                            fmt_resources(res.hru)
                        } else {
                            "-".to_string()
                        });
                    });
                    row.col(|ui| {
                        let nru_load =
                            nru_loads[row_idx].get_or_insert_with(nru_loader(contract.contract_id));
                        match nru_load.ready() {
                            Some(Ok(nru)) => ui.label(fmt_resources(*nru)),
                            Some(Err(err)) => ui.colored_label(ui.visuals().error_fg_color, err),
                            None => ui.spinner(),
                        };
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.number_of_public_ips));
                    });
                    row.col(|ui| {
                        let cost_load = node_price_loads[row_idx]
                            .get_or_insert_with(cost_loader(contract.contract_id));
                        match cost_load.ready() {
                            Some(Ok(cost)) => ui.label(fmt_tft(*cost)),
                            Some(Err(err)) => ui.colored_label(ui.visuals().error_fg_color, err),
                            None => ui.spinner(),
                        };
                    });
                    row.col(|ui| {
                        ui.label(&contract.deployment_hash);
                    });
                    row.col(|ui| {
                        if ui
                            .label(if contract.deployment_data.len() <= 30 {
                                contract.deployment_data.clone()
                            } else {
                                let mut dd = contract.deployment_data.clone();
                                dd.truncate(30);
                                dd
                            })
                            .hovered()
                        {
                            egui::show_tooltip(
                                ui.ctx(),
                                egui::Id::new("contract_deployment_data_tooltip"),
                                |ui| {
                                    ui.label(if contract.deployment_data.is_empty() {
                                        "No contract data set on chain for this contract"
                                    } else {
                                        &contract.deployment_data
                                    });
                                },
                            );
                        };
                    });
                    row.col(|ui| {
                        ui.label(fmt_local_time(contract.created_at));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.state));
                    });
                });
            });
    });
}

fn ui_name_contracts<C, N>(
    ui: &mut egui::Ui,
    name_contracts: &[NameContract],
    nru_loads: &mut [Option<Promise<Result<u64, String>>>],
    name_price_loads: &mut [Option<Promise<Result<u64, String>>>],
    nru_loader: impl Fn(u64) -> N,
    cost_loader: impl Fn(u64) -> C,
) where
    C: FnOnce() -> Promise<Result<u64, String>>,
    N: FnOnce() -> Promise<Result<u64, String>>,
{
    egui::ScrollArea::horizontal().show(ui, |ui| {
        TableBuilder::new(ui)
            .cell_layout(Layout::centered_and_justified(egui::Direction::LeftToRight))
            .columns(Column::auto().resizable(true).clip(false), 7)
            .column(Column::remainder().clip(false).at_most(100.))
            .striped(true)
            .header(50.0, |mut header| {
                for title in [
                    "Contract ID",
                    "Twin ID",
                    "Solution Provider ID",
                    "Name",
                    "Nru",
                    "Total Cost",
                    "Created",
                    "State",
                ] {
                    header.col(|ui| {
                        ui.heading(title);
                    });
                }
            })
            .body(|body| {
                body.rows(30.0, name_contracts.len(), |row_idx, mut row| {
                    let contract = &name_contracts[row_idx];
                    row.col(|ui| {
                        ui.label(format!("{}", contract.contract_id));
                    });
                    row.col(|ui| {
                        if ui.label(format!("{}", contract.twin_id)).hovered() {
                            egui::show_tooltip(
                                ui.ctx(),
                                egui::Id::new("contract_twin_id_tooltip"),
                                |ui| {
                                    ui.label(format!(
                                        "This contract is created and owned by twin {}",
                                        contract.twin_id
                                    ));
                                },
                            );
                        };
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.solution_provider_id.unwrap_or(0)));
                    });
                    row.col(|ui| {
                        ui.label(&contract.name);
                    });
                    row.col(|ui| {
                        let nru_load =
                            nru_loads[row_idx].get_or_insert_with(nru_loader(contract.contract_id));
                        match nru_load.ready() {
                            Some(Ok(nru)) => ui.label(fmt_resources(*nru)),
                            Some(Err(err)) => ui.colored_label(ui.visuals().error_fg_color, err),
                            None => ui.spinner(),
                        };
                    });
                    row.col(|ui| {
                        let cost_load = name_price_loads[row_idx]
                            .get_or_insert_with(cost_loader(contract.contract_id));
                        match cost_load.ready() {
                            Some(Ok(cost)) => ui.label(fmt_tft(*cost)),
                            Some(Err(err)) => ui.colored_label(ui.visuals().error_fg_color, err),
                            None => ui.spinner(),
                        };
                    });
                    row.col(|ui| {
                        ui.label(fmt_local_time(contract.created_at));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.state));
                    });
                });
            });
    });
}

fn ui_rent_contracts<C>(
    ui: &mut egui::Ui,
    rent_contracts: &[RentContract],
    rent_price_loads: &mut [Option<Promise<Result<u64, String>>>],
    cost_loader: impl Fn(u64) -> C,
) where
    C: FnOnce() -> Promise<Result<u64, String>>,
{
    egui::ScrollArea::horizontal().show(ui, |ui| {
        TableBuilder::new(ui)
            .cell_layout(Layout::centered_and_justified(egui::Direction::LeftToRight))
            .columns(Column::auto().resizable(true).clip(false), 6)
            .column(Column::remainder().clip(false).at_most(100.))
            .striped(true)
            .header(50.0, |mut header| {
                for title in [
                    "Contract ID",
                    "Node ID",
                    "Twin ID",
                    "Solution Provider ID",
                    "Total Cost",
                    "Created",
                    "State",
                ] {
                    header.col(|ui| {
                        ui.heading(title);
                    });
                }
            })
            .body(|body| {
                body.rows(30.0, rent_contracts.len(), |row_idx, mut row| {
                    let contract = &rent_contracts[row_idx];
                    row.col(|ui| {
                        ui.label(format!("{}", contract.contract_id));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.node_id));
                    });
                    row.col(|ui| {
                        if ui.label(format!("{}", contract.twin_id)).hovered() {
                            egui::show_tooltip(
                                ui.ctx(),
                                egui::Id::new("contract_twin_id_tooltip"),
                                |ui| {
                                    ui.label(format!(
                                        "This contract is created and owned by twin {}",
                                        contract.twin_id
                                    ));
                                },
                            );
                        };
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.solution_provider_id.unwrap_or(0)));
                    });
                    row.col(|ui| {
                        let cost_load = rent_price_loads[row_idx]
                            .get_or_insert_with(cost_loader(contract.contract_id));
                        match cost_load.ready() {
                            Some(Ok(cost)) => ui.label(fmt_tft(*cost)),
                            Some(Err(err)) => ui.colored_label(ui.visuals().error_fg_color, err),
                            None => ui.spinner(),
                        };
                    });
                    row.col(|ui| {
                        ui.label(fmt_local_time(contract.created_at));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", contract.state));
                    });
                });
            });
    });
}

fn ui_node_state_changes(ui: &mut egui::Ui, state_changes: &[NodeStateChange]) {
    egui::ScrollArea::horizontal().show(ui, |ui| {
        TableBuilder::new(ui)
            .cell_layout(Layout::centered_and_justified(egui::Direction::LeftToRight))
            .columns(Column::auto().resizable(true).clip(false), 2)
            .column(Column::remainder().clip(false).at_most(100.))
            .striped(true)
            .header(50.0, |mut header| {
                for title in ["", "Event", "Event detected"] {
                    header.col(|ui| {
                        ui.heading(title);
                    });
                }
            })
            .body(|body| {
                body.rows(30.0, state_changes.len(), |row_idx, mut row| {
                    let state_change = &state_changes[row_idx];
                    let (emoji, msg) = node_state_formatted(state_change.state());
                    row.col(|ui| {
                        ui.label(emoji.to_string());
                    });
                    row.col(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.label(msg);
                        });
                    });
                    row.col(|ui| {
                        ui.label(fmt_local_time(state_change.timestamp()));
                    });
                });
            });
    });
}

fn ui_node_jitter_graph(ui: &mut egui::Ui, uptime_events: &[UptimeEvent]) {
    let jitter_data: PlotPoints = uptime_events
        .windows(2)
        .map(|window| {
            [
                window[1].timestamp() as f64,
                if window[1].uptime() > (window[1].timestamp() - window[0].timestamp()) as u64 {
                    ((window[1].uptime() - window[0].uptime()) as i64
                        - (window[1].timestamp() - window[0].timestamp()))
                        as f64
                } else {
                    0.
                },
            ]
        })
        .collect();
    let delay_data: PlotPoints = uptime_events
        .windows(2)
        .map(|window| {
            [
                window[1].timestamp() as f64,
                if window[1].uptime() > (window[1].timestamp() - window[0].timestamp()) as u64 {
                    (window[1].timestamp() - window[0].timestamp()) as f64
                } else {
                    0.
                },
            ]
        })
        .collect();
    let jitter_line = Line::new(jitter_data).name("jitter");
    let delay_line = Line::new(delay_data).name("uptime spacing");
    Plot::new("jitter_plot")
        .label_formatter(|name, value| {
            if name == "jitter" {
                format!(
                    "{}: {} seconds jitter",
                    fmt_local_time(value.x as i64),
                    // cast to i64 to avoid weird rounding
                    value.y as i64,
                )
            } else if name == "uptime spacing" {
                format!(
                    "{}: {:.2} minutes",
                    fmt_local_time(value.x as i64),
                    value.y / 60.,
                )
            } else {
                "".to_string()
            }
        })
        .x_axis_formatter(|value, _range| fmt_local_time(value as i64))
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(jitter_line);
            plot_ui.line(delay_line);
        });
}

fn ui_bill_graph(ui: &mut egui::Ui, bill_reports: &[ContractBillReport]) {
    let mut hourly_cost = BTreeMap::new();
    for bill_report in bill_reports {
        *hourly_cost.entry(bill_report.timestamp / 3600).or_insert(0) += bill_report.amount_billed;
    }
    let bill_data: PlotPoints = hourly_cost
        .into_iter()
        .map(|(k, v)| [(k * 3600) as f64, v as f64])
        .collect();
    let bill_cost_line = Line::new(bill_data).name("bill cost");
    Plot::new("bill_cost_plot")
        .label_formatter(|_, value| {
            format!(
                "{}: {:.7} TFT",
                fmt_local_time(value.x as i64),
                value.y / 10_000_000.,
            )
        })
        .x_axis_formatter(|value, _range| fmt_local_time(value as i64))
        .y_axis_formatter(|value, _range| format!("{} TFT", value as u64 / 10_000_000))
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(bill_cost_line);
        });
}

fn ui_multi_input<T>(
    ui: &mut egui::Ui,
    label_text: &str,
    error_text: &mut String,
    buffer: &mut String,
    collection: &mut BTreeSet<T>,
) where
    T: FromStr + std::fmt::Display + Clone + Ord,
    T::Err: std::fmt::Display,
{
    ui.horizontal(|ui| {
        let label = ui.label(label_text);
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            let input_response = ui.text_edit_singleline(buffer).labelled_by(label.id);
            if input_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                // parse id, try to fetch data
                let parse_res = buffer.parse::<T>();
                match parse_res {
                    Ok(id) => {
                        collection.insert(id);
                        error_text.clear();
                    }
                    Err(e) => *error_text = e.to_string(),
                }
            }
            ui.colored_label(ui.visuals().error_fg_color, error_text);
        });
        ui.horizontal_top(|ui| {
            for id in collection.clone() {
                if ui.button(format!("{id}")).clicked() {
                    collection.remove(&id);
                };
            }
        });
    });
}

fn ui_single_input<T>(
    ui: &mut egui::Ui,
    label_text: &str,
    error_text: &mut String,
    buffer: &mut String,
    value: &mut Option<T>,
) where
    T: FromStr + std::fmt::Display + Clone + Ord,
    T::Err: std::fmt::Display,
{
    ui.horizontal(|ui| {
        let label = ui.label(label_text);
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            let input_response = ui.text_edit_singleline(buffer).labelled_by(label.id);
            if input_response.changed() {
                // parse id, try to fetch data
                let parse_res = buffer.parse::<T>();
                match parse_res {
                    Ok(id) => {
                        *value = Some(id);
                        error_text.clear();
                    }
                    Err(e) => {
                        *value = None;
                        *error_text = e.to_string();
                    }
                }
            }
            ui.colored_label(ui.visuals().error_fg_color, error_text);
        });
    });
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum MenuSelection {
    ContractOverview,
    ContractDetails,
    NodeState,
    TotalBilled,
}

impl std::fmt::Display for MenuSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractOverview => f.write_str("Contract overview"),
            Self::ContractDetails => f.write_str("Contract details"),
            Self::NodeState => f.write_str("Node state history"),
            Self::TotalBilled => f.write_str("Total billed on chain"),
        }
    }
}

/// Value of 1 KiB.
const KIB: u64 = 1 << 10;
/// Value of 1 MiB.
const MIB: u64 = 1 << 20;
/// Value of 1 GiB.
const GIB: u64 = 1 << 30;
/// Value of 1 TiB.
const TIB: u64 = 1 << 40;

fn fmt_resources(value: u64) -> String {
    match value {
        v if v > TIB => format!("{:.2} TiB", value as f64 / TIB as f64),
        v if v > GIB => format!("{:.2} GiB", value as f64 / GIB as f64),
        v if v > MIB => format!("{:.2} MiB", value as f64 / MIB as f64),
        v if v > KIB => format!("{:.2} KiB", value as f64 / KIB as f64),
        v => format!("{v} B"),
    }
}

// TODO: custom fonts
/// Emoji for node boot.
const UP_ARROW_EMOJI: char = '⬆';
/// Emoji for node going offline.
const DOWN_ARROW_EMOJI: char = '⬇';
/// Emoji for impossible reboot.
const BOOM_EMOJI: char = '☢';
/// Emoji for node uptime drift.
const CLOCK_EMOJI: char = '🕑';
/// Emoji for unknown state.
const QUESTION_MARK_EMOJI: char = '？';

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

/// Amount of the smallest on chain currency unit which equate 1 TFT. In other words, 1 TFT can be
/// split up in this many pieces.
const UNITS_PER_TFT: u64 = 10_000_000;

/// Format an amount as value in TFT
fn fmt_tft(amount: u64) -> String {
    format!("{}.{} TFT", amount / UNITS_PER_TFT, amount % UNITS_PER_TFT)
}
