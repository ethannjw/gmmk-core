mod device;
mod layout;
mod profile;
mod protocol;
mod ui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "GMMK Config",
        native_options,
        Box::new(|cc| Ok(Box::new(ui::GmmkApp::new(cc)))),
    )
}
