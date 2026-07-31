#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod vault;

use app::VaultApp;
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([420.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "BlobVault",
        options,
        Box::new(|cc| Ok(Box::new(VaultApp::new(cc)))),
    )
}
