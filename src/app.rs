use eframe::{egui, App};

pub struct UiState {}

impl UiState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        log::debug!("{:?}", cc.integration_info);

        Self {}
    }
}

impl App for UiState {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
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
            // todo
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // todo
        });
    }
}

enum MenuSelection {}
