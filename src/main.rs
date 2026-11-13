#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod config;
mod media;
mod model;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1420.0, 880.0])
            .with_min_inner_size([1080.0, 680.0])
            .with_title("ZAPL Recorder")
            .with_icon(std::sync::Arc::new(app_icon())),
        ..Default::default()
    };

    eframe::run_native(
        "ZAPL Recorder",
        options,
        Box::new(|cc| Ok(Box::new(app::RecorderApp::new(cc)))),
    )
}

/// Иконка рисуется в коде: красная точка записи на тёмном квадрате.
fn app_icon() -> egui::IconData {
    const SIZE: u32 = 64;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let center = SIZE as f32 / 2.0 - 0.5;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let index = ((y * SIZE + x) * 4) as usize;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 30.0 {
                continue;
            }
            let mut color = [24u8, 26, 32, 255];
            if dist < 12.0 {
                color = [229, 72, 77, 255];
            } else if dist < 15.0 {
                color = [40, 43, 52, 255];
            }
            rgba[index..index + 4].copy_from_slice(&color);
        }
    }

    egui::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}
