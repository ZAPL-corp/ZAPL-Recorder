//! Окна «О программе», журнала, подтверждения и всплывающие уведомления.

use eframe::egui::{self, RichText};

use crate::app::{Confirm, RecorderApp, APP_NAME, APP_VERSION};
use crate::ui::{format_duration, theme, TEXT_WEAK};

pub fn windows(app: &mut RecorderApp, ctx: &egui::Context) {
    about(app, ctx);
    log(app, ctx);
    confirm(app, ctx);
    toast(app, ctx);
}

fn about(app: &mut RecorderApp, ctx: &egui::Context) {
    if !app.show_about {
        return;
    }
    let mut open = true;
    egui::Window::new("О программе")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .show(ctx, |ui| {
            ui.heading(format!("{APP_NAME} {APP_VERSION}"));
            theme::weak(ui, "Лёгкий рекордер и стример экрана: Rust + egui + ffmpeg.");
            ui.add_space(6.0);
            theme::kv(ui, "Платформа", app.os_name.clone());
            theme::kv(ui, "Интерфейс", "egui / glow (OpenGL)");
            theme::kv(
                ui,
                "Захват",
                if app.has_ddagrab() {
                    "DXGI Desktop Duplication + gdigrab"
                } else if cfg!(windows) {
                    "gdigrab (GDI)"
                } else {
                    "x11grab"
                },
            );
            theme::kv(
                ui,
                "Кодирование",
                app.probe
                    .as_ref()
                    .map(|probe| probe.summary())
                    .unwrap_or_else(|| "—".to_string()),
            );
            theme::kv(
                ui,
                "ffmpeg",
                app.ffmpeg
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "не найден".to_string()),
            );

            ui.add_space(10.0);
            ui.label(RichText::new("Системные характеристики").strong());
            egui::Grid::new("specs_grid")
                .striped(true)
                .num_columns(3)
                .spacing([16.0, 5.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Категория").strong());
                    ui.label(RichText::new("Минимальные").strong());
                    ui.label(RichText::new("Рекомендуемые").strong());
                    ui.end_row();

                    ui.label("Процессор CPU");
                    ui.label("AMD A4-9120 / Intel Celeron N4000");
                    ui.label("любой 4-ядерный");
                    ui.end_row();

                    ui.label("Графика GPU");
                    ui.label("AMD Radeon R3 / Intel UHD 600");
                    ui.label("NVIDIA GTX 1630 / AMD RX 550");
                    ui.end_row();

                    ui.label("ОЗУ RAM");
                    ui.label("4 ГБ");
                    ui.label("8 ГБ");
                    ui.end_row();

                    ui.label("Место на диске");
                    ui.label("от 100 ГБ для записей");
                    ui.label("от 100 ГБ, программа ~10 МБ");
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label(RichText::new("Как это устроено").strong());
            for line in [
                "Приложение не кодирует видео само: оно собирает команду ffmpeg и запускает его.",
                "Сцена — это набор источников и их расположение на холсте.",
                "Превью — отдельный поток ffmpeg на 5 кадрах/с, во время записи он выключается.",
                "Ключи стрима лежат в config.json рядом с настройками (на Linux права 600).",
            ] {
                ui.label(RichText::new(format!("• {line}")).small().color(TEXT_WEAK));
            }

            ui.add_space(8.0);
            ui.label(RichText::new("Ограничения").strong());
            for line in [
                "Захват браузера — это захват области экрана: окно должно быть видимым.",
                "Wayland поддерживается только через XWayland.",
                "Стрим — RTMP; SRT и мультиплатформенная одновременная трансляция не входят в эту версию.",
            ] {
                ui.label(RichText::new(format!("• {line}")).small().color(TEXT_WEAK));
            }
        });
    app.show_about = open;
}

fn log(app: &mut RecorderApp, ctx: &egui::Context) {
    if !app.show_log {
        return;
    }
    let mut open = true;
    egui::Window::new("Журнал и команда ffmpeg")
        .open(&mut open)
        .default_size([780.0, 470.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(command) = &app.last_command {
                    if ui.button("Скопировать команду").clicked() {
                        ctx.copy_text(command.clone());
                    }
                }
                if ui.button("Очистить журнал").clicked() {
                    app.log.clear();
                }
                if let Some(summary) = &app.last_summary {
                    theme::weak(
                        ui,
                        format!(
                            "Последняя сессия: {} · {}",
                            summary.kind.label(),
                            format_duration(summary.duration)
                        ),
                    );
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("log_scroll")
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if app.log.is_empty() {
                        theme::weak(ui, "Журнал пуст.");
                    }
                    for line in &app.log {
                        ui.label(RichText::new(line).monospace().size(11.0));
                    }
                });
            if let Some(command) = &app.last_command {
                ui.separator();
                theme::weak(ui, "Команда последнего запуска:");
                ui.label(RichText::new(command).monospace().size(10.0));
            }
        });
    app.show_log = open;
}

fn confirm(app: &mut RecorderApp, ctx: &egui::Context) {
    let Some(kind) = app.confirm else {
        return;
    };
    let text = kind.text(app);
    let mut open = true;
    let mut decision: Option<bool> = None;
    egui::Window::new("Подтверждение")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.label(text);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if theme::button(ui, "Удалить", theme::DANGER).clicked() {
                    decision = Some(true);
                }
                if ui.button("Отмена").clicked() {
                    decision = Some(false);
                }
            });
        });

    match decision {
        Some(true) => {
            match kind {
                Confirm::Scene(id) => app.remove_scene(id),
                Confirm::Source(id) => app.remove_source(id),
            }
            app.confirm = None;
        }
        Some(false) => app.confirm = None,
        None => {
            if !open {
                app.confirm = None;
            }
        }
    }
}

fn toast(app: &mut RecorderApp, ctx: &egui::Context) {
    let Some(toast) = app.toast.clone() else {
        return;
    };
    egui::Area::new(egui::Id::unique("zapl_toast"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 64.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            egui::Frame::NONE
                .fill(if toast.error {
                    theme::DANGER
                } else {
                    theme::ACCENT
                })
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(toast.text)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                });
        });
}
