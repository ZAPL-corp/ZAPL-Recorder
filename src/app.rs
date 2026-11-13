//! Состояние приложения и связка интерфейса с ffmpeg.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, TextureHandle, TextureOptions};

use crate::config::{self, Config};
use crate::media::{
    self, ffmpeg, CapturePlan, Destination, Preview, PreviewFrame, Session, SessionKind,
};
use crate::model::{
    AudioSource, CanvasRect, Id, Mode, Scene, ScreenRect, Source, SourceKind, AudioKind,
};
use crate::ui;

pub const APP_NAME: &str = "ZAPL Recorder";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FFMPEG_HINT: &str = "ffmpeg не найден. Положите ffmpeg.exe (Linux: ffmpeg) в папку bin рядом \
с программой или установите его в PATH, затем нажмите «Проверить ffmpeg».";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    General,
    Video,
    Output,
    Stream,
    Audio,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Confirm {
    Scene(Id),
    Source(Id),
}

impl Confirm {
    pub fn text(self, app: &RecorderApp) -> String {
        match self {
            Confirm::Scene(id) => {
                let name = app
                    .cfg
                    .scene(id)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                format!("Удалить сцену «{name}» со всеми источниками?")
            }
            Confirm::Source(id) => {
                let name = app
                    .cfg
                    .active_scene()
                    .and_then(|s| s.sources.iter().find(|x| x.id == id))
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                format!("Удалить источник «{name}»?")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct SessionSummary {
    pub kind: SessionKind,
    pub title: String,
    pub output: Option<PathBuf>,
    pub duration: Duration,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

pub struct RecorderApp {
    pub cfg: Config,
    pub ffmpeg: Option<PathBuf>,
    pub probe: Option<ffmpeg::Probe>,
    pub session: Option<Session>,
    pub last_summary: Option<SessionSummary>,
    pub preview: Option<Preview>,
    pub preview_texture: Option<TextureHandle>,
    pub preview_error: Option<String>,
    pub preview_due: Option<Instant>,
    pub preview_last_at: Option<Instant>,
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    pub show_log: bool,
    pub show_about: bool,
    pub confirm: Option<Confirm>,
    pub toast: Option<Toast>,
    pub log: VecDeque<String>,
    pub selected_source: Option<Id>,
    pub show_keys: bool,
    pub dirty: bool,
    pub last_edit: Option<Instant>,
    pub config_path: Option<PathBuf>,
    pub last_command: Option<String>,
    pub os_name: String,
}

impl RecorderApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::theme::apply(&cc.egui_ctx);
        let cfg = config::load();
        config::save_example_if_missing(&cfg);
        let mut app = Self {
            cfg,
            ffmpeg: None,
            probe: None,
            session: None,
            last_summary: None,
            preview: None,
            preview_texture: None,
            preview_error: None,
            preview_due: None,
            preview_last_at: None,
            settings_open: false,
            settings_tab: SettingsTab::General,
            show_log: false,
            show_about: false,
            confirm: None,
            toast: None,
            log: VecDeque::new(),
            selected_source: None,
            show_keys: false,
            dirty: false,
            last_edit: None,
            config_path: None,
            last_command: None,
            os_name: os_label().to_string(),
        };
        app.selected_source = app
            .cfg
            .active_scene()
            .and_then(|scene| scene.sources.first().map(|s| s.id));
        app.refresh_ffmpeg();
        if app.cfg.settings.preview_enabled {
            app.request_preview_restart(400);
        }
        app
    }

    // ------------------------------------------------------------------
    // ffmpeg и предпросмотр
    // ------------------------------------------------------------------

    pub fn refresh_ffmpeg(&mut self) {
        self.ffmpeg = ffmpeg::locate();
        self.probe = self.ffmpeg.as_ref().map(|path| ffmpeg::probe(path));
        match (&self.ffmpeg, &self.probe) {
            (Some(path), Some(probe)) => {
                let version = probe
                    .version
                    .clone()
                    .unwrap_or_else(|| "версия неизвестна".to_string());
                self.push_log(format!("ffmpeg: {} ({version})", path.display()));
                self.preview_error = None;
                self.request_preview_restart(200);
            }
            _ => {
                self.preview_error = Some(FFMPEG_HINT.to_string());
                self.push_log(FFMPEG_HINT.to_string());
            }
        }
    }

    pub fn has_ddagrab(&self) -> bool {
        self.probe.as_ref().map(|p| p.has_ddagrab()).unwrap_or(false)
    }

    pub fn audio_devices(&self) -> &[String] {
        self.probe
            .as_ref()
            .map(|p| p.audio_devices.as_slice())
            .unwrap_or(&[])
    }

    pub fn request_preview_restart(&mut self, delay_ms: u64) {
        if self.cfg.settings.preview_enabled && self.session.is_none() {
            self.preview_due = Some(Instant::now() + Duration::from_millis(delay_ms));
        }
    }

    fn spawn_preview(&mut self) {
        self.preview_due = None;
        self.stop_preview();
        let Some(ffmpeg) = self.ffmpeg.clone() else {
            self.preview_error = Some(FFMPEG_HINT.to_string());
            return;
        };
        let settings = self.cfg.settings.clone();
        let Some(scene) = self.cfg.active_scene().cloned() else {
            return;
        };
        match media::build_preview(&ffmpeg, &settings, &scene, self.has_ddagrab()) {
            Ok(plan) => match Preview::start(&plan) {
                Ok(preview) => {
                    self.preview_error = None;
                    self.preview = Some(preview);
                }
                Err(err) => self.preview_error = Some(err),
            },
            Err(err) => self.preview_error = Some(err),
        }
    }

    pub fn stop_preview(&mut self) {
        self.preview_due = None;
        if let Some(mut preview) = self.preview.take() {
            preview.stop();
        }
    }

    fn upload_frame(&mut self, ctx: &egui::Context, frame: PreviewFrame) {
        let expected = frame.width.saturating_mul(frame.height).saturating_mul(4);
        if frame.width == 0 || frame.height == 0 || frame.rgba.len() < expected {
            return;
        }
        let image =
            egui::ColorImage::from_rgba_unmultiplied([frame.width, frame.height], &frame.rgba);
        match self.preview_texture.as_mut() {
            Some(texture) => texture.set(image, TextureOptions::LINEAR),
            None => {
                self.preview_texture =
                    Some(ctx.load_texture("zapl-preview", image, TextureOptions::LINEAR));
            }
        }
        self.preview_last_at = Some(Instant::now());
    }

    // ------------------------------------------------------------------
    // Запись и стрим
    // ------------------------------------------------------------------

    fn make_plan(&self, destination: Destination) -> Result<(CapturePlan, String), String> {
        let ffmpeg = self.ffmpeg.as_ref().ok_or_else(|| FFMPEG_HINT.to_string())?;
        let scene = self
            .cfg
            .active_scene()
            .cloned()
            .ok_or_else(|| "Нет активной сцены.".to_string())?;
        let settings = self.cfg.settings.clone();
        let audio = self.cfg.audio.clone();
        let input = media::graph::PlanInput {
            ffmpeg,
            settings: &settings,
            scene: &scene,
            audio: &audio,
            destination: &destination,
            has_ddagrab: self.has_ddagrab(),
        };
        let plan = media::build_record(&input)?;
        Ok((plan, scene.name))
    }

    pub fn start_record(&mut self) {
        if self.session.is_some() {
            return;
        }
        let scene_name = self
            .cfg
            .active_scene()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "scene".to_string());
        let path = media::graph::record_path(&self.cfg.settings, &scene_name);
        match self.make_plan(Destination::File(path.clone())) {
            Ok((plan, _)) => {
                let title = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "запись".to_string());
                self.begin_session(plan, SessionKind::Recording, title);
            }
            Err(err) => self.toast(err, true),
        }
    }

    pub fn start_stream(&mut self) {
        if self.session.is_some() {
            return;
        }
        let Some(target) = self.cfg.settings.active_stream_target().cloned() else {
            self.toast(
                "Нет площадки для стрима. Откройте Настройки → Стрим и добавьте её.",
                true,
            );
            return;
        };
        if target.url.trim().is_empty() {
            self.toast("У площадки пустой RTMP-адрес.", true);
            return;
        }
        if target.key.trim().is_empty() {
            self.toast("Укажите ключ стрима в настройках площадки.", true);
            return;
        }
        let destination = Destination::Stream(target.ingest());
        match self.make_plan(destination) {
            Ok((plan, _)) => {
                let title = format!("{} · {}", target.platform.label(), target.name);
                self.begin_session(plan, SessionKind::Streaming, title);
            }
            Err(err) => self.toast(err, true),
        }
    }

    fn begin_session(&mut self, plan: CapturePlan, kind: SessionKind, title: String) {
        self.stop_preview();
        self.log_command(&plan);
        match Session::start(&plan, kind, title.clone()) {
            Ok(session) => {
                self.session = Some(session);
                self.toast(format!("{}: старт — {title}", kind.label()), false);
            }
            Err(err) => self.toast(err, true),
        }
    }

    pub fn stop_session(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.stop();
            self.complete_session(session);
        }
        self.request_preview_restart(500);
    }

    fn complete_session(&mut self, session: Session) {
        let summary = SessionSummary {
            kind: session.kind,
            title: session.title.clone(),
            output: session.output.clone(),
            duration: session.elapsed(),
            message: session
                .exit
                .clone()
                .unwrap_or_else(|| "остановлено".to_string()),
        };
        if let Some(path) = &summary.output {
            self.push_log(format!("Файл записи: {}", path.display()));
        }
        self.push_log(format!(
            "{} «{}»: {} · {}",
            summary.kind.label(),
            summary.title,
            summary.message,
            ui::format_duration(summary.duration)
        ));
        let text = match &summary.output {
            Some(path) => format!(
                "Запись сохранена: {} · {}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                ui::format_duration(summary.duration)
            ),
            None => format!(
                "Стрим «{}» остановлен · {}",
                summary.title,
                ui::format_duration(summary.duration)
            ),
        };
        self.last_summary = Some(summary);
        self.toast(text, false);
    }

    fn log_command(&mut self, plan: &CapturePlan) {
        self.push_log(format!("Запуск: {}", plan.label));
        let line = plan.command_line();
        self.push_log(format!("$ {line}"));
        self.last_command = Some(line);
    }

    pub fn open_output_dir(&mut self) {
        let dir = PathBuf::from(&self.cfg.settings.output_dir);
        if let Err(err) = media::open_folder(&dir) {
            self.toast(err, true);
        }
    }

    // ------------------------------------------------------------------
    // Настройки, сцены, источники
    // ------------------------------------------------------------------

    pub fn apply_mode(&mut self, mode: Mode) {
        if self.cfg.settings.mode == mode {
            return;
        }
        self.cfg.settings.apply_mode(mode);
        self.toast(
            format!(
                "{}: {}p{} · {} кбит/с",
                mode.label(),
                self.cfg.settings.canvas_h,
                self.cfg.settings.fps,
                self.cfg.settings.bitrate_kbps
            ),
            false,
        );
        self.mark_dirty();
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_edit = Some(Instant::now());
        self.request_preview_restart(900);
    }

    pub fn mark_dirty_light(&mut self) {
        self.dirty = true;
        self.last_edit = Some(Instant::now());
    }

    pub fn save_now(&mut self, notify: bool) {
        match config::save(&self.cfg) {
            Ok(path) => {
                self.dirty = false;
                self.config_path = Some(path);
                if notify {
                    self.toast("Настройки сохранены", false);
                }
            }
            Err(err) => self.toast(err, true),
        }
    }

    pub fn toast(&mut self, text: impl Into<String>, error: bool) {
        self.toast = Some(Toast {
            text: text.into(),
            error,
            at: Instant::now(),
        });
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        let stamp = chrono::Local::now().format("%H:%M:%S");
        self.log.push_back(format!("[{stamp}] {}", line.into()));
        while self.log.len() > 400 {
            self.log.pop_front();
        }
    }

    pub fn select_scene(&mut self, id: Id) {
        if self.cfg.active_scene == id {
            return;
        }
        self.cfg.active_scene = id;
        self.selected_source = self
            .cfg
            .active_scene()
            .and_then(|scene| scene.sources.first().map(|s| s.id));
        self.mark_dirty_light();
        self.request_preview_restart(400);
    }

    pub fn add_scene(&mut self) {
        let id = self.cfg.next_id();
        let number = self.cfg.scenes.len() + 1;
        self.cfg.scenes.push(Scene {
            id,
            name: format!("Сцена {number}"),
            sources: Vec::new(),
        });
        self.cfg.active_scene = id;
        self.selected_source = None;
        self.mark_dirty();
    }

    pub fn duplicate_scene(&mut self) {
        let Some(scene) = self.cfg.active_scene().cloned() else {
            return;
        };
        let new_id = self.cfg.next_id();
        let mut copy = scene.clone();
        copy.id = new_id;
        copy.name = format!("{} (копия)", scene.name);
        for source in &mut copy.sources {
            source.id = self.cfg.next_id();
        }
        self.cfg.scenes.push(copy);
        self.cfg.active_scene = new_id;
        self.selected_source = self
            .cfg
            .active_scene()
            .and_then(|s| s.sources.first().map(|x| x.id));
        self.mark_dirty();
    }

    pub fn remove_scene(&mut self, id: Id) {
        if self.cfg.scenes.len() <= 1 {
            self.toast("Нельзя удалить последнюю сцену.", true);
            return;
        }
        self.cfg.scenes.retain(|scene| scene.id != id);
        if self.cfg.active_scene == id {
            let next = self.cfg.scenes[0].id;
            self.cfg.active_scene = next;
        }
        self.selected_source = None;
        self.mark_dirty();
    }

    pub fn move_scene(&mut self, id: Id, delta: i32) {
        let Some(index) = self.cfg.scenes.iter().position(|s| s.id == id) else {
            return;
        };
        let target = index as i32 + delta;
        if target < 0 || target as usize >= self.cfg.scenes.len() {
            return;
        }
        self.cfg.scenes.swap(index, target as usize);
        self.mark_dirty_light();
    }

    pub fn add_source(&mut self, kind: SourceKind) {
        let id = self.cfg.next_id();
        let (canvas_w, canvas_h) = (self.cfg.settings.canvas_w, self.cfg.settings.canvas_h);
        let mut source = Source::of_kind(id, kind);
        source.canvas = match kind {
            SourceKind::Display => CanvasRect::full(canvas_w, canvas_h),
            _ => {
                let w = (canvas_w / 2).max(320);
                let h = (canvas_h / 2).max(180);
                CanvasRect {
                    x: ((canvas_w.saturating_sub(w)) / 2) as i32,
                    y: ((canvas_h.saturating_sub(h)) / 2) as i32,
                    w,
                    h,
                }
            }
        };
        source.screen = ScreenRect {
            x: 60,
            y: 60,
            w: canvas_w.min(1280),
            h: canvas_h.min(720),
        };
        if let Some(scene) = self.cfg.active_scene_mut() {
            scene.sources.insert(0, source);
        }
        self.selected_source = Some(id);
        self.mark_dirty();
    }

    pub fn remove_source(&mut self, id: Id) {
        if let Some(scene) = self.cfg.active_scene_mut() {
            scene.sources.retain(|source| source.id != id);
        }
        if self.selected_source == Some(id) {
            self.selected_source = self
                .cfg
                .active_scene()
                .and_then(|scene| scene.sources.first().map(|s| s.id));
        }
        self.mark_dirty();
    }

    pub fn move_source(&mut self, id: Id, delta: i32) {
        let Some(scene) = self.cfg.active_scene_mut() else {
            return;
        };
        let Some(index) = scene.sources.iter().position(|s| s.id == id) else {
            return;
        };
        let target = index as i32 + delta;
        if target < 0 || target as usize >= scene.sources.len() {
            return;
        }
        scene.sources.swap(index, target as usize);
        self.mark_dirty();
    }

    pub fn set_source_visible(&mut self, id: Id, visible: bool) {
        if let Some(scene) = self.cfg.active_scene_mut() {
            if let Some(source) = scene.sources.iter_mut().find(|s| s.id == id) {
                source.visible = visible;
            }
        }
        self.mark_dirty();
    }

    pub fn apply_source_draft(&mut self, id: Id, draft: Source) {
        if let Some(scene) = self.cfg.active_scene_mut() {
            if let Some(source) = scene.sources.iter_mut().find(|s| s.id == id) {
                *source = draft;
            }
        }
        self.mark_dirty();
    }

    pub fn open_browser_for(&mut self, source: &Source) {
        match media::open_browser(&source.target, source.screen) {
            Ok(()) => self.toast("Окно браузера открыто — не перекрывайте его другими окнами.", false),
            Err(err) => self.toast(err, true),
        }
    }

    pub fn add_audio(&mut self, kind: AudioKind) {
        let id = self.cfg.next_id();
        let audio = AudioSource {
            id,
            name: match kind {
                AudioKind::Mic => "Микрофон".to_string(),
                AudioKind::Desktop => "Системный звук".to_string(),
            },
            kind,
            device: self
                .audio_devices()
                .first()
                .cloned()
                .unwrap_or_else(|| default_device(kind)),
            volume: 1.0,
            muted: false,
            enabled: true,
        };
        self.cfg.audio.push(audio);
        self.mark_dirty_light();
    }

    pub fn refresh_devices(&mut self) {
        if let Some(ffmpeg) = self.ffmpeg.clone() {
            let devices = ffmpeg::audio_devices(&ffmpeg);
            let count = devices.len();
            if let Some(probe) = self.probe.as_mut() {
                probe.audio_devices = devices;
            }
            self.toast(format!("Найдено устройств звука: {count}"), false);
        } else {
            self.toast(FFMPEG_HINT, true);
        }
    }

    // ------------------------------------------------------------------
    // Цикл приложения
    // ------------------------------------------------------------------

    fn tick(&mut self, ctx: &egui::Context) {
        if let Some(session) = self.session.as_mut() {
            session.poll();
        }
        if self
            .session
            .as_ref()
            .map(|session| session.is_finished())
            .unwrap_or(false)
        {
            if let Some(session) = self.session.take() {
                self.complete_session(session);
            }
        }

        let frame = self.preview.as_ref().and_then(|preview| preview.next_frame());
        if let Some(frame) = frame {
            self.upload_frame(ctx, frame);
        }
        let preview_error = self
            .preview
            .as_ref()
            .and_then(|preview| preview.last_error());
        if let Some(err) = preview_error {
            self.preview_error = Some(err);
        }

        if self.session.is_some() {
            if self.preview.is_some() {
                self.stop_preview();
            }
        } else if let Some(due) = self.preview_due {
            if Instant::now() >= due {
                if self.cfg.settings.preview_enabled {
                    self.spawn_preview();
                } else {
                    self.preview_due = None;
                }
            }
        }

        if self.dirty
            && self
                .last_edit
                .map(|at| at.elapsed() >= Duration::from_millis(800))
                .unwrap_or(false)
        {
            self.save_now(false);
        }

        if let Some(toast) = &self.toast {
            if toast.at.elapsed() > Duration::from_secs(5) {
                self.toast = None;
            }
        }

        if self.session.is_some() || self.preview.is_some() || self.toast.is_some() || self.dirty {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }
}

impl Drop for RecorderApp {
    fn drop(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.stop();
        }
        self.stop_preview();
        let _ = config::save(&self.cfg);
    }
}

impl eframe::App for RecorderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);

        let panel_frame =
            egui::Frame::NONE.fill(ui::theme::BG_PANEL).inner_margin(egui::Margin::same(8));
        let status_frame =
            egui::Frame::NONE.fill(ui::theme::BG_PANEL).inner_margin(egui::Margin::same(8));

        egui::Panel::top("menu_bar")
            .frame(panel_frame)
            .show(ui, |ui| ui::top_bar(self, ui));

        egui::Panel::top("toolbar")
            .frame(egui::Frame::NONE.fill(ui::theme::BG).inner_margin(egui::Margin::same(8)))
            .show(ui, |ui| ui::toolbar(self, ui));

        egui::Panel::bottom("status_bar")
            .frame(status_frame)
            .show(ui, |ui| ui::status_bar(self, ui));

        egui::Panel::left("scenes_panel")
            .resizable(true)
            .default_size(268.0)
            .size_range(220.0..=400.0)
            .frame(egui::Frame::NONE.fill(ui::theme::BG).inner_margin(egui::Margin::same(8)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("left_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui::scenes::panel(self, ui);
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui::sources::panel(self, ui);
                    });
            });

        egui::Panel::right("mixer_panel")
            .resizable(true)
            .default_size(262.0)
            .size_range(220.0..=380.0)
            .frame(egui::Frame::NONE.fill(ui::theme::BG).inner_margin(egui::Margin::same(8)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("right_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| ui::mixer::panel(self, ui));
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(ui::theme::BG).inner_margin(egui::Margin::same(8)))
            .show(ui, |ui| {
                ui::preview::panel(self, ui);
                ui.add_space(8.0);
                ui::controls::bar(self, ui);
            });

        ui::settings::window(self, &ctx);
        ui::dialogs::windows(self, &ctx);
    }
}

fn os_label() -> &'static str {
    if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    }
}

fn default_device(kind: AudioKind) -> String {
    match kind {
        AudioKind::Mic => "default".to_string(),
        AudioKind::Desktop => "default".to_string(),
    }
}
