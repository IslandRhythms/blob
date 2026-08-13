#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod vault;

use app::{VaultApp, ICON_PNG};
use eframe::egui;

fn main() -> eframe::Result {
    let icon = load_window_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([420.0, 480.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "BlobVault",
        options,
        Box::new(|cc| Ok(Box::new(VaultApp::new(cc)))),
    )
}

fn load_window_icon() -> egui::IconData {
    let image = image::load_from_memory(ICON_PNG)
        .expect("app icon")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    let rgba = image.into_rgba8();
    egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}
