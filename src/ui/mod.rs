//! Верхние панели, статусная строка и общие помощники интерфейса.

pub mod controls;
pub mod dialogs;
pub mod mixer;
pub mod preview;
pub mod scenes;
pub mod settings;
pub mod sources;
pub mod theme;

use std::time::Duration;

use eframe::egui::{self, RichText};

use crate::app::{Confirm, RecorderApp};
use crate::media::SessionKind;
use crate::model::{Mode, SourceKind};

pub use theme::TEXT_WEAK;

pub fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub fn top_bar(app: &mut RecorderApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("Файл", |ui| {
            if ui.button("Открыть папку записей").clicked() {
                app.open_output_dir();
                ui.close();
            }
            if ui.button("Сохранить настройки").clicked() {
                app.save_now(true);
                ui.close();
            }
            if ui.button("Проверить ffmpeg").clicked() {
                app.refresh_ffmpeg();
                ui.close();
            }
            ui.separator();
            if ui.button("Выход").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        ui.menu_button("Сцены", |ui| {
            if ui.button("Добавить сцену").clicked() {
                app.add_scene();
                ui.close();
            }
            if ui.button("Дублировать текущую").clicked() {
                app.duplicate_scene();
                ui.close();
            }
            if ui.button("Удалить текущую").clicked() {
                let id = app.cfg.active_scene;
                app.confirm = Some(Confirm::Scene(id));
                ui.close();
            }
        });

        ui.menu_button("Источники", |ui| {
            for kind in SourceKind::ALL {
                if ui.button(format!("Добавить: {}", kind.label())).clicked() {
                    app.add_source(kind);
                    ui.close();
                }
            }
        });

        ui.menu_button("Профиль", |ui| {
            for mode in Mode::ALL {
                let selected = app.cfg.settings.mode == mode;
                if ui.selectable_label(selected, mode.label()).clicked() {
                    app.apply_mode(mode);
                    ui.close();
                }
            }
        });

        ui.menu_button("Инструменты", |ui| {
            if ui.button("Настройки").clicked() {
                app.settings_open = true;
                app.settings_tab = crate::app::SettingsTab::General;
                ui.close();
            }
            if ui.button("Журнал и команда ffmpeg").clicked() {
                app.show_log = true;
                ui.close();
            }
            if ui.button("Обновить превью").clicked() {
                app.request_preview_restart(0);
                ui.close();
            }
        });

        ui.menu_button("Справка", |ui| {
            if ui.button("О программе").clicked() {
                app.show_about = true;
                ui.close();
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (text, color) = match &app.probe {
                Some(probe) => (format!("ffmpeg · {}", probe.summary()), theme::LIVE),
                None => ("ffmpeg не найден".to_string(), theme::DANGER),
            };
            let response = ui.label(RichText::new(text).small().color(color));
            if response.clicked() {
                app.refresh_ffmpeg();
            }
        });
    });
}

pub fn toolbar(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let settings = &app.cfg.settings;
    let mode = settings.mode;
    let encoder = settings.encoder;
    let (canvas_h, fps, bitrate) = (settings.canvas_h, settings.fps, settings.bitrate_kbps);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Режим").small().color(TEXT_WEAK));
        let mut selected = mode;
        egui::ComboBox::from_id_salt("mode_combo")
            .width(180.0)
            .selected_text(selected.label())
            .show_ui(ui, |ui| {
                for value in Mode::ALL {
                    ui.selectable_value(&mut selected, value, value.label());
                }
            });
        if selected != mode {
            app.apply_mode(selected);
        }
        ui.label(
            RichText::new(format!("{canvas_h}p{fps} · {bitrate} кбит/с"))
                .small()
                .color(TEXT_WEAK),
        )
        .on_hover_text(mode.hint());

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new("Настройки").min_size(egui::vec2(96.0, 24.0)))
                .clicked()
            {
                app.settings_open = true;
            }
            ui.separator();
            ui.label(
                RichText::new(format!("{} · {}", encoder.label(), mode.label()))
                    .small()
                    .color(TEXT_WEAK),
            );
        });
    });
}

pub fn status_bar(app: &mut RecorderApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let mut big = "00:00".to_string();
        let mut tag = "IDLE";
        let mut color = theme::TEXT_WEAK;
        let mut subtitle = "Готов к записи".to_string();

        if let Some(session) = &app.session {
            big = format_duration(session.elapsed());
            let live = session.kind == SessionKind::Streaming;
            tag = if live { "LIVE" } else { "REC" };
            color = if live { theme::LIVE } else { theme::DANGER };
            subtitle = if live {
                format!(
                    "{} · {} · {} · потеряно {}",
                    session.title,
                    session.stats.fps_text(),
                    session.stats.bitrate_text(),
                    session.stats.dropped
                )
            } else {
                let size = session
                    .stats
                    .size_kb
                    .map(|kb| format!("{:.1} МБ", kb as f32 / 1024.0))
                    .unwrap_or_else(|| "—".to_string());
                format!(
                    "Запись · {} · {} · {}",
                    session.title,
                    size,
                    session.stats.fps_text()
                )
            };
        }

        theme::inset().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tag).size(10.0).strong().color(color));
                ui.label(RichText::new(big).monospace().size(22.0).strong().color(
                    if app.session.is_some() {
                        color
                    } else {
                        theme::TEXT
                    },
                ));
            });
        });
        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).small().color(TEXT_WEAK));
            let cfg_text = if app.dirty {
                "конфиг: есть несохранённые изменения".to_string()
            } else {
                "конфиг сохранён".to_string()
            };
            ui.label(RichText::new(cfg_text).size(10.0).color(theme::TEXT_WEAK));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Папка").clicked() {
                app.open_output_dir();
            }
            if ui.button("Журнал").clicked() {
                app.show_log = true;
            }
            if ui.button("Настройки").clicked() {
                app.settings_open = true;
            }
            ui.separator();
            let preview_text = if app.preview.is_some() {
                "превью активно"
            } else if !app.cfg.settings.preview_enabled {
                "превью выключено"
            } else {
                "превью: ожидание"
            };
            ui.label(RichText::new(preview_text).small().color(TEXT_WEAK));
        });
    });
}
