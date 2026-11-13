//! Окно настроек: режимы, видео, вывод, стрим и звук.

use eframe::egui::{self, RichText};

use crate::app::{RecorderApp, SettingsTab};
use crate::config;
use crate::model::{CaptureEngine, Container, Encoder, Mode, Platform, StreamTarget};
use crate::ui::{theme, TEXT_WEAK};

pub fn window(app: &mut RecorderApp, ctx: &egui::Context) {
    if !app.settings_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Настройки ZAPL Recorder")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([700.0, 620.0])
        .min_width(600.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (SettingsTab::General, "Общие"),
                    (SettingsTab::Video, "Видео"),
                    (SettingsTab::Output, "Вывод"),
                    (SettingsTab::Stream, "Стрим"),
                    (SettingsTab::Audio, "Аудио"),
                ] {
                    ui.selectable_value(&mut app.settings_tab, tab, label);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("settings_scroll")
                .auto_shrink([false, false])
                .max_height(470.0)
                .show(ui, |ui| match app.settings_tab {
                    SettingsTab::General => general(app, ui),
                    SettingsTab::Video => video(app, ui),
                    SettingsTab::Output => output(app, ui),
                    SettingsTab::Stream => stream(app, ui),
                    SettingsTab::Audio => audio(app, ui),
                });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Сохранить сейчас").clicked() {
                    app.save_now(true);
                }
                if ui.button("Закрыть").clicked() {
                    app.settings_open = false;
                }
            });
            theme::weak(ui, format!("Конфиг: {}", config::config_path().display()));
        });
    if !open {
        app.settings_open = false;
    }
}

fn general(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let mut changed = false;

    theme::section(ui, "ПРОФИЛЬ РЕЖИМА");
    let mut mode = app.cfg.settings.mode;
    egui::ComboBox::from_id_salt("settings_mode")
        .width(260.0)
        .selected_text(mode.label())
        .show_ui(ui, |ui| {
            for value in Mode::ALL {
                ui.selectable_value(&mut mode, value, value.label());
            }
        });
    if mode != app.cfg.settings.mode {
        app.apply_mode(mode);
    }
    theme::weak(ui, mode.hint());

    ui.add_space(8.0);
    theme::section(ui, "ПАПКА ЗАПИСЕЙ");
    ui.horizontal(|ui| {
        let mut dir = app.cfg.settings.output_dir.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut dir).desired_width(430.0))
            .changed()
        {
            app.cfg.settings.output_dir = dir;
            changed = true;
        }
        if ui.button("Открыть").clicked() {
            app.open_output_dir();
        }
    });

    ui.add_space(8.0);
    theme::section(ui, "КОНТЕЙНЕР");
    ui.horizontal(|ui| {
        let mut container = app.cfg.settings.container;
        for value in [Container::Mkv, Container::Mp4] {
            if ui
                .selectable_value(&mut container, value, value.label())
                .changed()
            {
                changed = true;
            }
        }
        app.cfg.settings.container = container;
    });
    if app.cfg.settings.container == Container::Mp4 {
        theme::weak(
            ui,
            "MP4 пишется с faststart; при аварийном завершении файл может не открыться. MKV надёжнее.",
        );
    }

    ui.add_space(8.0);
    theme::section(ui, "ПРЕВЬЮ");
    let mut preview = app.cfg.settings.preview_enabled;
    if ui.checkbox(&mut preview, "Показывать превью, когда запись не идёт").changed() {
        app.cfg.settings.preview_enabled = preview;
        if preview {
            app.request_preview_restart(200);
        } else {
            app.stop_preview();
        }
        changed = true;
    }
    theme::weak(
        ui,
        "Превью работает отдельным ffmpeg на 5 кадрах/с и останавливается во время записи.",
    );

    ui.add_space(8.0);
    theme::section(ui, "FFMPEG");
    match (&app.ffmpeg, &app.probe) {
        (Some(path), Some(probe)) => {
            theme::kv(ui, "Путь", path.display().to_string());
            theme::kv(
                ui,
                "Версия",
                probe
                    .version
                    .clone()
                    .unwrap_or_else(|| "неизвестно".to_string()),
            );
            theme::kv(ui, "Кодеки", probe.summary());
            theme::kv(ui, "Устройств звука", probe.audio_devices.len().to_string());
        }
        _ => {
            ui.label(RichText::new(crate::app::FFMPEG_HINT).color(theme::DANGER));
        }
    }
    ui.horizontal(|ui| {
        if ui.button("Проверить ffmpeg").clicked() {
            app.refresh_ffmpeg();
        }
        if ui.button("Обновить звуковые устройства").clicked() {
            app.refresh_devices();
        }
    });

    if changed {
        app.mark_dirty_light();
    }
}

fn video(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let mut changed = false;
    theme::section(ui, "ХОЛСТ");
    let (mut width, mut height) = (app.cfg.settings.canvas_w, app.cfg.settings.canvas_h);

    const PRESETS: [(u32, u32, &str); 5] = [
        (1920, 1080, "Full HD"),
        (2560, 1440, "2K"),
        (1600, 900, "HD+"),
        (1280, 720, "HD"),
        (1080, 1920, "Vertical"),
    ];
    let mut chosen: Option<(u32, u32)> = None;
    let label = PRESETS
        .iter()
        .find(|(w, h, _)| *w == width && *h == height)
        .map(|(_, _, name)| name.to_string())
        .unwrap_or_else(|| "Своё".to_string());
    egui::ComboBox::from_id_salt("canvas_preset")
        .width(260.0)
        .selected_text(format!("{width}×{height} · {label}"))
        .show_ui(ui, |ui| {
            for (w, h, name) in PRESETS {
                if ui
                    .selectable_label(width == w && height == h, format!("{w}×{h} · {name}"))
                    .clicked()
                {
                    chosen = Some((w, h));
                }
            }
        });
    if let Some((w, h)) = chosen {
        width = w;
        height = h;
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("Ширина").small().color(TEXT_WEAK));
        if ui
            .add(egui::DragValue::new(&mut width).range(320..=3840).speed(2.0))
            .changed()
        {
            changed = true;
        }
        ui.label(RichText::new("Высота").small().color(TEXT_WEAK));
        if ui
            .add(egui::DragValue::new(&mut height).range(240..=2160).speed(2.0))
            .changed()
        {
            changed = true;
        }
    });

    let mut fps = app.cfg.settings.fps;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Кадров/с").small().color(TEXT_WEAK));
        if ui
            .add(egui::Slider::new(&mut fps, 10..=120).suffix(" fps"))
            .changed()
        {
            changed = true;
        }
    });

    if changed {
        app.cfg.settings.canvas_w = width;
        app.cfg.settings.canvas_h = height;
        app.cfg.settings.fps = fps;
        app.mark_dirty();
    }

    ui.add_space(8.0);
    theme::section(ui, "ЗАХВАТ ЭКРАНА");
    let mut engine = app.cfg.settings.capture_engine;
    egui::ComboBox::from_id_salt("capture_engine")
        .width(320.0)
        .selected_text(engine.label())
        .show_ui(ui, |ui| {
            for value in CaptureEngine::ALL {
                if !value.applies_to_os() {
                    continue;
                }
                let enabled = match value {
                    CaptureEngine::Dxgi => app.has_ddagrab(),
                    _ => true,
                };
                let label = if enabled {
                    value.label().to_string()
                } else {
                    format!("{} · недоступно", value.label())
                };
                ui.selectable_value(&mut engine, value, label);
            }
        });
    if engine != app.cfg.settings.capture_engine {
        app.cfg.settings.capture_engine = engine;
        app.mark_dirty();
    }
    theme::weak(ui, engine.hint());
    if cfg!(windows) && !app.has_ddagrab() && app.probe.is_some() {
        theme::weak(
            ui,
            "ddagrab отсутствует в этой сборке ffmpeg — будет использован GDI. Для слабого ПК выгоднее сборка ffmpeg 6+ с DXGI.",
        );
    }
    if cfg!(not(windows)) {
        theme::weak(
            ui,
            "Linux: захват через X11. Wayland работает только через XWayland; окна ищутся через xdotool.",
        );
    }

    ui.add_space(8.0);
    theme::section(ui, "ОПТИМИЗАЦИЯ");
    theme::weak(
        ui,
        "Профиль записи влияет на битрейт и preset, но не на разрешение холста. Для Celeron N4000: 720p30, ultrafast, MKV.",
    );
}

fn output(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let mut changed = false;
    theme::section(ui, "КОДИРОВЩИК");
    let mut encoder = app.cfg.settings.encoder;
    let probe = app.probe.clone();
    egui::ComboBox::from_id_salt("encoder_combo")
        .width(320.0)
        .selected_text(encoder.label())
        .show_ui(ui, |ui| {
            for value in Encoder::ALL {
                let available = probe
                    .as_ref()
                    .map(|probe| probe.has_encoder(value.codec()))
                    .unwrap_or(false);
                let label = if available {
                    value.label().to_string()
                } else {
                    format!("{} · нет в ffmpeg", value.label())
                };
                ui.selectable_value(&mut encoder, value, label);
            }
        });
    if encoder != app.cfg.settings.encoder {
        app.cfg.settings.encoder = encoder;
        changed = true;
    }
    theme::weak(ui, encoder.hint());
    if let Some(probe) = &app.probe {
        if !probe.has_encoder(encoder.codec()) {
            ui.label(
                RichText::new("Этот кодировщик не найден в текущей сборке ffmpeg.")
                    .small()
                    .color(theme::DANGER),
            );
        }
    }

    ui.add_space(8.0);
    theme::section(ui, "X264 PRESET");
    ui.horizontal(|ui| {
        let mut preset = app.cfg.settings.x264_preset.clone();
        egui::ComboBox::from_id_salt("x264_preset")
            .width(180.0)
            .selected_text(preset.clone())
            .show_ui(ui, |ui| {
                for value in [
                    "ultrafast",
                    "superfast",
                    "veryfast",
                    "faster",
                    "fast",
                    "medium",
                ] {
                    ui.selectable_value(&mut preset, value.to_string(), value);
                }
            });
        if preset != app.cfg.settings.x264_preset {
            app.cfg.settings.x264_preset = preset;
            changed = true;
        }
        theme::weak(ui, "ultrafast — меньше всего грузит CPU");
    });

    ui.add_space(8.0);
    theme::section(ui, "БИТРЕЙТ И КЛЮЧЕВЫЕ КАДРЫ");
    let mut bitrate = app.cfg.settings.bitrate_kbps;
    if ui
        .add(
            egui::Slider::new(&mut bitrate, 500..=30_000)
                .suffix(" кбит/с")
                .step_by(250.0),
        )
        .changed()
    {
        app.cfg.settings.bitrate_kbps = bitrate;
        changed = true;
    }
    let mut keyframe = app.cfg.settings.keyframe_sec;
    if ui
        .add(egui::Slider::new(&mut keyframe, 1..=10).suffix(" с"))
        .changed()
    {
        app.cfg.settings.keyframe_sec = keyframe;
        changed = true;
    }
    theme::weak(
        ui,
        "Для стрима ключевой кадр 2 секунды — стандарт Twitch и YouTube.",
    );

    ui.add_space(8.0);
    theme::section(ui, "ЗВУК");
    let mut audio_bitrate = app.cfg.settings.audio_bitrate_kbps;
    if ui
        .add(
            egui::Slider::new(&mut audio_bitrate, 64..=320)
                .suffix(" кбит/с")
                .step_by(32.0),
        )
        .changed()
    {
        app.cfg.settings.audio_bitrate_kbps = audio_bitrate;
        changed = true;
    }

    ui.add_space(8.0);
    theme::section(ui, "ФОРМАТ");
    let mut container = app.cfg.settings.container;
    for value in [Container::Mkv, Container::Mp4] {
        ui.selectable_value(&mut container, value, value.label());
    }
    if container != app.cfg.settings.container {
        app.cfg.settings.container = container;
        changed = true;
    }

    if changed {
        app.mark_dirty_light();
    }
}

fn stream(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let mut targets: Vec<StreamTarget> = app.cfg.settings.stream_targets.clone();
    let mut active = app.cfg.settings.active_target;
    let mut changed = false;
    let mut add: Option<Platform> = None;
    let mut remove: Option<u64> = None;

    ui.horizontal(|ui| {
        for platform in Platform::ALL {
            if ui.button(format!("+ {}", platform.label())).clicked() {
                add = Some(platform);
            }
        }
    });
    ui.add_space(4.0);
    theme::weak(
        ui,
        "Twitch, YouTube и Kick подставляют RTMP-адрес — нужен только ключ. Для любого другого сервера выберите «Свой RTMP».",
    );

    ui.add_space(6.0);
    for target in &mut targets {
        ui.push_id(target.id, |ui| {
            theme::card().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut platform = target.platform;
                    egui::ComboBox::from_id_salt("platform_combo")
                        .width(150.0)
                        .selected_text(platform.label())
                        .show_ui(ui, |ui| {
                            for value in Platform::ALL {
                                ui.selectable_value(&mut platform, value, value.label());
                            }
                        });
                    if platform != target.platform {
                        let old_default = target.platform.default_url();
                        if target.url.trim().is_empty()
                            || target.url.trim_end_matches('/')
                                == old_default.trim_end_matches('/')
                        {
                            target.url = platform.default_url().to_string();
                        }
                        if target.name.trim().is_empty() || target.name == target.platform.label() {
                            target.name = platform.label().to_string();
                        }
                        target.platform = platform;
                        changed = true;
                    }

                    let mut name = target.name.clone();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut name)
                                .desired_width(170.0)
                                .hint_text("Название"),
                        )
                        .changed()
                    {
                        target.name = name;
                        changed = true;
                    }

                    if active == Some(target.id) {
                        theme::tag(ui, "АКТИВНА", theme::LIVE);
                    } else if ui.small_button("Выбрать").clicked() {
                        active = Some(target.id);
                        changed = true;
                    }
                    if ui.small_button("Удалить").clicked() {
                        remove = Some(target.id);
                    }
                });

                ui.horizontal(|ui| {
                    ui.add_sized(
                        [80.0, 18.0],
                        egui::Label::new(RichText::new("RTMP").small().color(TEXT_WEAK)),
                    );
                    let mut url = target.url.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut url).desired_width(430.0))
                        .changed()
                    {
                        target.url = url;
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [80.0, 18.0],
                        egui::Label::new(RichText::new("Ключ").small().color(TEXT_WEAK)),
                    );
                    let mut key = target.key.clone();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut key)
                                .password(!app.show_keys)
                                .desired_width(360.0)
                                .hint_text("ключ трансляции"),
                        )
                        .changed()
                    {
                        target.key = key;
                        changed = true;
                    }
                    ui.checkbox(&mut app.show_keys, "показать");
                });
                ui.horizontal(|ui| {
                    theme::weak(ui, format!("Ключ: {}", target.masked_key()));
                    theme::weak(ui, "·");
                    theme::weak(ui, platform_hint(target.platform));
                });
            });
        });
        ui.add_space(4.0);
    }

    if let Some(platform) = add {
        let id = app.cfg.next_id();
        targets.push(StreamTarget {
            id,
            platform,
            name: platform.label().to_string(),
            url: platform.default_url().to_string(),
            key: String::new(),
        });
        active = Some(id);
        changed = true;
    }
    if let Some(id) = remove {
        targets.retain(|target| target.id != id);
        if active == Some(id) {
            active = targets.first().map(|target| target.id);
        }
        changed = true;
    }
    if active
        .map(|id| targets.iter().any(|target| target.id == id))
        .unwrap_or(false)
        == false
    {
        active = targets.first().map(|target| target.id);
    }

    if targets.is_empty() {
        theme::weak(ui, "Площадок пока нет. Добавьте Twitch, YouTube или свой RTMP.");
    }

    if changed {
        app.cfg.settings.stream_targets = targets;
        app.cfg.settings.active_target = active;
        app.mark_dirty_light();
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Ключи хранятся локально в config.json (на Linux права 600). Не публикуйте этот файл.",
        )
        .small()
        .color(TEXT_WEAK),
    );
}

fn audio(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let devices = app.audio_devices().to_vec();
    let mut list = app.cfg.audio.clone();
    let mut changed = false;
    let mut remove: Option<u64> = None;

    theme::weak(
        ui,
        "Windows: устройства dshow, системный звук — через virtual-audio-capturer. Linux: pulse, системный звук — monitor-источник.",
    );
    ui.horizontal(|ui| {
        if ui.button("Найти устройства").clicked() {
            app.refresh_devices();
        }
        if ui.button("+ Микрофон").clicked() {
            app.add_audio(crate::model::AudioKind::Mic);
        }
        if ui.button("+ Системный звук").clicked() {
            app.add_audio(crate::model::AudioKind::Desktop);
        }
    });
    ui.add_space(6.0);

    for item in &mut list {
        theme::card().show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut enabled = item.enabled;
                if ui.checkbox(&mut enabled, "").changed() {
                    item.enabled = enabled;
                    changed = true;
                }
                let mut name = item.name.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut name).desired_width(180.0))
                    .changed()
                {
                    item.name = name;
                    changed = true;
                }
                theme::tag(ui, item.kind.label(), theme::ACCENT);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Удалить").clicked() {
                        remove = Some(item.id);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Устройство").small().color(TEXT_WEAK));
                let mut device = item.device.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut device).desired_width(260.0))
                    .changed()
                {
                    item.device = device.clone();
                    changed = true;
                }
                egui::ComboBox::from_id_salt(format!("device_combo_{}", item.id))
                    .width(230.0)
                    .selected_text(if items_contains(&devices, &item.device) {
                        "из списка"
                    } else {
                        "выбрать"
                    })
                    .show_ui(ui, |ui| {
                        if devices.is_empty() {
                            ui.label("Список пуст — введите строку вручную");
                        }
                        for value in &devices {
                            if ui
                                .selectable_label(item.device == *value, value)
                                .clicked()
                            {
                                item.device = value.clone();
                                changed = true;
                            }
                        }
                    });
            });
            let mut volume = item.volume;
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.5).text("Громкость"))
                .changed()
            {
                item.volume = volume;
                changed = true;
            }
        });
        ui.add_space(4.0);
    }

    if let Some(id) = remove {
        list.retain(|item| item.id != id);
        changed = true;
    }

    if changed {
        app.cfg.audio = list;
        app.mark_dirty_light();
    }
    if app.cfg.audio.is_empty() {
        theme::weak(ui, "Аудио не настроено — запись будет без звука.");
    }
}

fn items_contains(list: &[String], value: &str) -> bool {
    list.iter().any(|item| item == value)
}

fn platform_hint(platform: Platform) -> &'static str {
    match platform {
        Platform::Twitch => "ключ: Twitch → Настройки → Канал → Ключ основного потока",
        Platform::YouTube => "ключ: YouTube Студия → Трансляция → Параметры потока",
        Platform::Kick => "ключ: Kick → Настройки → Stream Key",
        Platform::Custom => "ключ: из панели вашего RTMP-сервера",
    }
}
