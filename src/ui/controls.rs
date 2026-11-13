//! Кнопки записи/стрима под превью.

use eframe::egui::{self, RichText};

use crate::app::{RecorderApp, SettingsTab, FFMPEG_HINT};
use crate::media::SessionKind;
use crate::ui::{theme, TEXT_WEAK};

pub fn bar(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let kind = app.session.as_ref().map(|session| session.kind);
    let recording = kind == Some(SessionKind::Recording);
    let streaming = kind == Some(SessionKind::Streaming);
    let busy = kind.is_some();
    let has_ffmpeg = app.probe.is_some();

    let target_label = app
        .cfg
        .settings
        .active_stream_target()
        .map(|target| format!("{} · {}", target.platform.label(), target.masked_key()));

    ui.horizontal(|ui| {
        let record_text = if recording {
            "Остановить запись"
        } else {
            "Начать запись"
        };
        let record_fill = if recording { theme::DANGER } else { theme::ACCENT };
        let record = ui
            .add_enabled_ui(!busy || recording, |ui| {
                theme::button(ui, record_text, record_fill)
            })
            .inner;
        if record.clicked() {
            if recording {
                app.stop_session();
            } else if has_ffmpeg {
                app.start_record();
            } else {
                app.toast(FFMPEG_HINT, true);
            }
        }

        let stream_text = if streaming {
            "Остановить стрим"
        } else {
            "Начать стрим"
        };
        let stream_fill = if streaming { theme::DANGER } else { theme::LIVE };
        let stream = ui
            .add_enabled_ui(!busy || streaming, |ui| {
                theme::button(ui, stream_text, stream_fill)
            })
            .inner;
        if stream.clicked() {
            if streaming {
                app.stop_session();
            } else if !has_ffmpeg {
                app.toast(FFMPEG_HINT, true);
            } else {
                app.start_stream();
            }
        }

        ui.separator();

        let mut preview = app.cfg.settings.preview_enabled;
        if ui.checkbox(&mut preview, "Превью").changed() {
            app.cfg.settings.preview_enabled = preview;
            app.mark_dirty_light();
            if preview {
                app.request_preview_restart(200);
            } else {
                app.stop_preview();
            }
        }

        if ui.button("Настройки").clicked() {
            app.settings_open = true;
        }
        if ui.button("Журнал").clicked() {
            app.show_log = true;
        }
        if ui.button("Папка записей").clicked() {
            app.open_output_dir();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match &target_label {
                Some(label) => {
                    let response = ui.label(
                        RichText::new(format!("Стрим: {label}"))
                            .small()
                            .color(TEXT_WEAK),
                    );
                    if response.clicked() {
                        app.settings_tab = SettingsTab::Stream;
                        app.settings_open = true;
                    }
                }
                None => {
                    if ui.button("Добавить площадку").clicked() {
                        app.settings_tab = SettingsTab::Stream;
                        app.settings_open = true;
                    }
                }
            }
        });
    });

    if !has_ffmpeg {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("ffmpeg не найден — запись и стрим недоступны.")
                    .small()
                    .color(theme::DANGER),
            );
            if ui.small_button("Проверить снова").clicked() {
                app.refresh_ffmpeg();
            }
        });
    }
}
