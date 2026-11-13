//! Правая панель: микшер звука и сводка вывода.

use eframe::egui::{self, RichText};

use crate::app::{RecorderApp, SettingsTab};
use crate::model::{AudioKind, AudioSource};
use crate::ui::{theme, TEXT_WEAK};

pub fn panel(app: &mut RecorderApp, ui: &mut egui::Ui) {
    theme::section(ui, "МИКШЕР ЗВУКА");

    let audio: Vec<AudioSource> = app.cfg.audio.clone();
    if audio.is_empty() {
        theme::weak(ui, "Звуковых источников нет. Добавьте микрофон или системный звук.");
    }

    for item in audio {
        let mut changed = false;
        let mut enabled = item.enabled;
        let mut muted = item.muted;
        let mut volume = item.volume;

        theme::card().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut enabled, "").changed() {
                    changed = true;
                }
                ui.label(RichText::new(&item.name).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.selectable_label(muted, "Mute").clicked() {
                        muted = !muted;
                        changed = true;
                    }
                    theme::tag(ui, item.kind.label(), theme::ACCENT);
                });
            });
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.5).show_value(false))
                .changed()
            {
                changed = true;
            }
            ui.horizontal(|ui| {
                theme::weak(ui, format!("{:.0}%", volume * 100.0));
                if !item.device.trim().is_empty() {
                    theme::weak(ui, format!("· {}", item.device));
                }
            });
        });
        ui.add_space(4.0);

        if changed {
            if let Some(target) = app.cfg.audio.iter_mut().find(|source| source.id == item.id) {
                target.enabled = enabled;
                target.muted = muted;
                target.volume = volume;
            }
            app.mark_dirty_light();
        }
    }

    ui.horizontal(|ui| {
        if ui.button("+ Микрофон").clicked() {
            app.add_audio(AudioKind::Mic);
        }
        if ui.button("+ Системный").clicked() {
            app.add_audio(AudioKind::Desktop);
        }
    });
    if !app.audio_devices().is_empty() {
        theme::weak(ui, format!("Найдено устройств: {}", app.audio_devices().len()));
    }
    if ui
        .add(egui::Button::new("Устройства и уровни").min_size(egui::vec2(0.0, 24.0)))
        .clicked()
    {
        app.settings_tab = SettingsTab::Audio;
        app.settings_open = true;
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    theme::section(ui, "ВЫВОД");
    let settings = &app.cfg.settings;
    ui.label(
        RichText::new(format!(
            "{}×{} · {} fps · {}",
            settings.canvas_w,
            settings.canvas_h,
            settings.fps,
            settings.encoder.label()
        ))
        .small(),
    );
    ui.label(
        RichText::new(format!(
            "{} · {} кбит/с · ключевой кадр {} с",
            settings.container.label(),
            settings.bitrate_kbps,
            settings.keyframe_sec
        ))
        .small()
        .color(TEXT_WEAK),
    );
    theme::weak(ui, format!("Папка: {}", settings.output_dir));

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Открыть папку").clicked() {
            app.open_output_dir();
        }
        if ui.button("Настройки").clicked() {
            app.settings_tab = SettingsTab::Output;
            app.settings_open = true;
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);

    theme::section(ui, "ПЛОЩАДКА СТРИМА");
    let target = app
        .cfg
        .settings
        .active_stream_target()
        .map(|target| {
            (
                target.platform.label().to_string(),
                target.name.clone(),
                target.masked_key(),
                target.url.clone(),
            )
        });
    match target {
        Some((platform, name, masked, url)) => {
            ui.label(RichText::new(format!("{platform} · {name}")).strong());
            theme::weak(ui, format!("ключ: {masked}"));
            theme::weak(ui, url);
        }
        None => {
            theme::weak(ui, "Площадка не выбрана.");
        }
    }
    if ui.button("Настроить площадки").clicked() {
        app.settings_tab = SettingsTab::Stream;
        app.settings_open = true;
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    theme::section(ui, "ОПТИМИЗАЦИЯ");
    theme::weak(
        ui,
        "Профиль задаёт разрешение, FPS и битрейт. Для слабого ПК выберите x264 ultrafast или аппаратный NVENC/QSV.",
    );
    if app.session.is_some() {
        theme::weak(ui, "Во время записи превью остановлено, чтобы не тратить CPU.");
    }
}
