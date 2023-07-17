use chrono::{Local, TimeZone};
use eframe::{
    egui::{self, Layout},
    emath::Align,
    App,
};
use egui_extras::{Column, TableBuilder};
use poll_promise::Promise;
use tfgrid_graphql::{
    contract::{ContractState, NameContract, NodeContract, RentContract},
    graphql::Contracts,
};

pub struct UiState {
    client: tfgrid_graphql::graphql::Client,
    promise: Option<Promise<Result<Contracts, String>>>,
    selected: MenuSelection,
}

impl UiState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        log::debug!("{:?}", cc.integration_info);

        Self {
            client: tfgrid_graphql::graphql::Client::mainnet().expect("can initiate client, TODO"),
            promise: None,
            selected: MenuSelection::ContractOverview,
        }
    }
}

impl App for UiState {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            client,
            promise,
            selected,
        } = self;

        let promise = promise.get_or_insert_with(|| {
            let client = client.clone();
            Promise::spawn_async(async move {
                client
                    .contracts(
                        Some(&[1]),
                        &[
                            ContractState::Created,
                            ContractState::GracePeriod,
                            ContractState::OutOfFunds,
                        ],
                        None,
                        &[],
                        &[],
                    )
                    .await
            })
        });

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            // todo
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

        egui::CentralPanel::default().show(ctx, |ui| match promise.ready() {
            // todo
            None => {
                ui.spinner();
            }
            Some(Err(err)) => {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }
            Some(Ok(contracts)) => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.collapsing("Node contracts", |ui| {
                        ui_node_contracts(ui, &contracts.node_contracts);
                    });
                    ui.collapsing("Name contracts", |ui| {
                        ui_name_contracts(ui, &contracts.name_contracts);
                    });
                    ui.collapsing("Rent contracts", |ui| {
                        ui_rent_contracts(ui, &contracts.rent_contracts);
                    });
                });
            }
        });
    }
}

fn ui_node_contracts(ui: &mut egui::Ui, node_contracts: &[NodeContract]) {
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
            .body(|mut body| {
                for contract in node_contracts {
                    body.row(30.0, |mut row| {
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
                            ui.label("TODO");
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", contract.number_of_public_ips));
                        });
                        row.col(|ui| {
                            ui.label("TODO");
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
                            ui.label(
                                Local
                                    .timestamp_opt(contract.created_at, 0)
                                    .single()
                                    .expect("Local time from timestamp is unambiguous")
                                    .format("%d/%m/%Y %H:%M:%S")
                                    .to_string(),
                            );
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", contract.state));
                        });
                    });
                }
            });
    });
}

fn ui_name_contracts(ui: &mut egui::Ui, name_contracts: &[NameContract]) {
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
            .body(|mut body| {
                for contract in name_contracts {
                    body.row(30.0, |mut row| {
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
                            ui.label("TODO");
                        });
                        row.col(|ui| {
                            ui.label("TODO");
                        });
                        row.col(|ui| {
                            ui.label(
                                Local
                                    .timestamp_opt(contract.created_at, 0)
                                    .single()
                                    .expect("Local time from timestamp is unambiguous")
                                    .format("%d/%m/%Y %H:%M:%S")
                                    .to_string(),
                            );
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", contract.state));
                        });
                    });
                }
            });
    });
}

fn ui_rent_contracts(ui: &mut egui::Ui, rent_contracts: &[RentContract]) {
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
            .body(|mut body| {
                for contract in rent_contracts {
                    body.row(30.0, |mut row| {
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
                            ui.label("TODO");
                        });
                        row.col(|ui| {
                            ui.label(
                                Local
                                    .timestamp_opt(contract.created_at, 0)
                                    .single()
                                    .expect("Local time from timestamp is unambiguous")
                                    .format("%d/%m/%Y %H:%M:%S")
                                    .to_string(),
                            );
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", contract.state));
                        });
                    });
                }
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
