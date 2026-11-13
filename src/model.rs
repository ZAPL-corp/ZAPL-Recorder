//! Модуль и записи: сцены, источники, настройки, запись экрана и звук.

use serde::{Deserialize, Serialize};

pub type Id = u64;

// ---------------------------------------------------------------------------
// Настройки
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Display,
    Window,
    Browser,
    Image,
}

impl SourceKind {
    pub const ALL: [SourceKind; 4] = [
        SourceKind::Display,
        SourceKind::Window,
        SourceKind::Browser,
        SourceKind::Image,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SourceKind::Display => "Захват экрана",
            SourceKind::Window => "Окно",
            SourceKind::Browser => "Браузер",
            SourceKind::Image => "Картинка",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            SourceKind::Display => "ЭКРАН",
            SourceKind::Window => "ОКНО",
            SourceKind::Browser => "WEB",
            SourceKind::Image => "IMG",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            SourceKind::Display => {
                "Захват всего монитора или части экрана (0 — основной монитор)."
            }
            SourceKind::Window => {
                "Захват окна по заголовку. Окно должно быть видимым и не свёрнуто."
            }
            SourceKind::Browser => {
                "URL, страница открывается в отдельном окне браузера рядом с программой."
            }
            SourceKind::Image => "Картинка PNG/JPG — логотип или оверлей поверх композиции.",
        }
    }
}

/// Создаёт источник экрана: по умолчанию весь монитор, а также область окна
/// с нужным монитором (целевой регион).
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Default for ScreenRect {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            w: 1280,
            h: 720,
        }
    }
}

/// Размер и положение области на холсте compositing.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(default)]
pub struct CanvasRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Default for CanvasRect {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            w: 1280,
            h: 720,
        }
    }
}

impl CanvasRect {
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            w: width,
            h: height,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Source {
    pub id: Id,
    pub name: String,
    pub kind: SourceKind,
    pub visible: bool,
    /// Монитор, окно по заголовку, URL или файл картинки/логотипа.
    pub target: String,
    pub canvas: CanvasRect,
    pub screen: ScreenRect,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            kind: SourceKind::Display,
            visible: true,
            target: String::new(),
            canvas: CanvasRect::default(),
            screen: ScreenRect::default(),
        }
    }
}

impl Source {
    pub fn display(id: Id, canvas_w: u32, canvas_h: u32) -> Self {
        Self {
            id,
            name: "Захват экрана".to_string(),
            kind: SourceKind::Display,
            visible: true,
            target: "0".to_string(),
            canvas: CanvasRect::full(canvas_w, canvas_h),
            ..Default::default()
        }
    }

    pub fn of_kind(id: Id, kind: SourceKind) -> Self {
        let (name, target) = match kind {
            SourceKind::Display => ("Захват экрана".to_string(), "0".to_string()),
            SourceKind::Window => ("Окно".to_string(), String::new()),
            SourceKind::Browser => ("Браузер".to_string(), "https://example.com".to_string()),
            SourceKind::Image => ("Картинка".to_string(), String::new()),
        };
        Self {
            id,
            name,
            kind,
            visible: true,
            target,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Scene {
    pub id: Id,
    pub name: String,
    pub sources: Vec<Source>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Сцена".to_string(),
            sources: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Аудио
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AudioKind {
    Mic,
    Desktop,
}

impl AudioKind {
    pub fn label(self) -> &'static str {
        match self {
            AudioKind::Mic => "Микрофон",
            AudioKind::Desktop => "Системный звук",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AudioSource {
    pub id: Id,
    pub name: String,
    pub kind: AudioKind,
    /// Устройство ввода для ffmpeg: `audio=...` (dshow) или `default` (pulse).
    pub device: String,
    pub volume: f32,
    pub muted: bool,
    pub enabled: bool,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Микрофон".to_string(),
            kind: AudioKind::Mic,
            device: String::new(),
            volume: 1.0,
            muted: false,
            enabled: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Пресеты режимов записи и стрима
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Game,
    Work,
    WorkStream,
    GameStream,
    Ai,
    AiStream,
}

#[derive(Clone, Copy, Debug)]
pub struct ModePreset {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub x264_preset: &'static str,
    pub tune: Option<&'static str>,
}

impl Mode {
    pub const ALL: [Mode; 6] = [
        Mode::Game,
        Mode::Work,
        Mode::WorkStream,
        Mode::GameStream,
        Mode::Ai,
        Mode::AiStream,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Game => "Запись игры",
            Mode::Work => "Запись работы",
            Mode::WorkStream => "Стрим работы",
            Mode::GameStream => "Стрим игры",
            Mode::Ai => "Запись ИИ",
            Mode::AiStream => "Стрим ИИ",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Mode::Game => "1080p60, быстрое кодирование, низкая задержка для динамичных игр.",
            Mode::Work => "720p30, статичный текст: tune stillimage, низкий битрейт для слабого CPU.",
            Mode::WorkStream => "900p30, 3 500 кбит/с — стабильно на слабом интернете.",
            Mode::GameStream => "1080p60, 6 000 кбит/с, ключевой кадр 2 секунды.",
            Mode::Ai => "1080p30, текст и таблицы, долгие сессии скринкастов нейросетей.",
            Mode::AiStream => "1080p30, 5 000 кбит/с — скринкаст нейросетей.",
        }
    }

    fn preset(self) -> ModePreset {
        // Оптимизировано для слабого железа (AMD A4-9120 + Radeon R3, 2 ядра):
        // 720p по умолчанию, preset ultrafast, низкий битрейт.
        match self {
            Mode::Game => ModePreset {
                width: 1280,
                height: 720,
                fps: 60,
                bitrate_kbps: 4_000,
                x264_preset: "ultrafast",
                tune: None,
            },
            Mode::Work => ModePreset {
                width: 1280,
                height: 720,
                fps: 30,
                bitrate_kbps: 2_500,
                x264_preset: "ultrafast",
                tune: Some("stillimage"),
            },
            Mode::WorkStream => ModePreset {
                width: 960,
                height: 540,
                fps: 30,
                bitrate_kbps: 2_000,
                x264_preset: "ultrafast",
                tune: Some("stillimage"),
            },
            Mode::GameStream => ModePreset {
                width: 1280,
                height: 720,
                fps: 30,
                bitrate_kbps: 3_000,
                x264_preset: "ultrafast",
                tune: None,
            },
            Mode::Ai => ModePreset {
                width: 1280,
                height: 720,
                fps: 30,
                bitrate_kbps: 3_000,
                x264_preset: "ultrafast",
                tune: Some("stillimage"),
            },
            Mode::AiStream => ModePreset {
                width: 1280,
                height: 720,
                fps: 30,
                bitrate_kbps: 2_500,
                x264_preset: "ultrafast",
                tune: Some("stillimage"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Кодировщики, контейнеры, способы захвата
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Encoder {
    X264,
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
}

impl Encoder {
    pub const ALL: [Encoder; 5] = [
        Encoder::X264,
        Encoder::Nvenc,
        Encoder::Qsv,
        Encoder::Amf,
        Encoder::Vaapi,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Encoder::X264 => "x264 (CPU)",
            Encoder::Nvenc => "NVIDIA NVENC",
            Encoder::Qsv => "Intel Quick Sync",
            Encoder::Amf => "AMD AMF",
            Encoder::Vaapi => "VAAPI (Linux)",
        }
    }

    pub fn codec(self) -> &'static str {
        match self {
            Encoder::X264 => "libx264",
            Encoder::Nvenc => "h264_nvenc",
            Encoder::Qsv => "h264_qsv",
            Encoder::Amf => "h264_amf",
            Encoder::Vaapi => "h264_vaapi",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Encoder::X264 => {
                "Нагрузка на процессор. Для слабого CPU используйте preset ultrafast."
            }
            Encoder::Nvenc => "Аппаратный кодировщик NVIDIA, почти без нагрузки на CPU.",
            Encoder::Qsv => "Intel Quick Sync, кодирование на видеоядре.",
            Encoder::Amf => "AMD AMF, кодирование на видеоядре Adrenalin.",
            Encoder::Vaapi => "Linux, /dev/dri/renderD128, ffmpeg с VAAPI.",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    Mkv,
    Mp4,
}

impl Container {
    pub fn label(self) -> &'static str {
        match self {
            Container::Mkv => "MKV (устойчив к обрыву записи)",
            Container::Mp4 => "MP4 (быстро открывается после записи)",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Container::Mkv => "mkv",
            Container::Mp4 => "mp4",
        }
    }

    pub fn muxer(self) -> &'static str {
        match self {
            Container::Mkv => "matroska",
            Container::Mp4 => "mp4",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CaptureEngine {
    Auto,
    Dxgi,
    Gdi,
    X11,
}

impl CaptureEngine {
    pub const ALL: [CaptureEngine; 4] = [
        CaptureEngine::Auto,
        CaptureEngine::Dxgi,
        CaptureEngine::Gdi,
        CaptureEngine::X11,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CaptureEngine::Auto => "Авто",
            CaptureEngine::Dxgi => "DXGI Desktop Duplication (GPU)",
            CaptureEngine::Gdi => "GDI (больше нагрузки на процессор)",
            CaptureEngine::X11 => "X11 / XShm",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            CaptureEngine::Auto => {
                "DXGI, если ffmpeg его поддерживает, иначе GDI. Под Linux — X11."
            }
            CaptureEngine::Dxgi => "Без нагрузки на процессор, кадры с GPU (Windows).",
            CaptureEngine::Gdi => "Нагрузка на процессор, работает на любом железе.",
            CaptureEngine::X11 => "Linux через X11. Wayland работает только через XWayland.",
        }
    }

    pub fn applies_to_os(self) -> bool {
        if cfg!(windows) {
            !matches!(self, CaptureEngine::X11)
        } else {
            !matches!(self, CaptureEngine::Dxgi | CaptureEngine::Gdi)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Twitch,
    YouTube,
    Kick,
    Custom,
}

impl Platform {
    pub const ALL: [Platform; 4] = [
        Platform::Twitch,
        Platform::YouTube,
        Platform::Kick,
        Platform::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Platform::Twitch => "Twitch",
            Platform::YouTube => "YouTube",
            Platform::Kick => "Kick",
            Platform::Custom => "Свой RTMP",
        }
    }

    pub fn default_url(self) -> &'static str {
        match self {
            Platform::Twitch => "rtmp://live.twitch.tv/app",
            Platform::YouTube => "rtmp://a.rtmp.youtube.com/live2",
            Platform::Kick => "rtmp://fa723fc1b171.global-contribute.live-video.net/app",
            Platform::Custom => "",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct StreamTarget {
    pub id: Id,
    pub platform: Platform,
    pub name: String,
    pub url: String,
    pub key: String,
}

impl Default for StreamTarget {
    fn default() -> Self {
        Self {
            id: 0,
            platform: Platform::Twitch,
            name: "Twitch".to_string(),
            url: Platform::Twitch.default_url().to_string(),
            key: String::new(),
        }
    }
}

impl StreamTarget {
    pub fn ingest(&self) -> String {
        let base = self.url.trim().trim_end_matches('/');
        let key = self.key.trim();
        if key.is_empty() {
            base.to_string()
        } else {
            format!("{base}/{key}")
        }
    }

    pub fn masked_key(&self) -> String {
        let key = self.key.trim();
        if key.is_empty() {
            "Ключ не задан".to_string()
        } else if key.len() <= 8 {
            "·".repeat(key.len())
        } else {
            format!("{}····{}", &key[..4], &key[key.len() - 4..])
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub encoder: Encoder,
    pub x264_preset: String,
    pub container: Container,
    pub capture_engine: CaptureEngine,
    pub preview_enabled: bool,
    pub output_dir: String,
    pub audio_bitrate_kbps: u32,
    pub keyframe_sec: u32,
    pub stream_targets: Vec<StreamTarget>,
    pub active_target: Option<Id>,
}

impl Default for Settings {
    fn default() -> Self {
        let preset = Mode::Work.preset();
        Self {
            mode: Mode::Work,
            canvas_w: preset.width,
            canvas_h: preset.height,
            fps: preset.fps,
            bitrate_kbps: preset.bitrate_kbps,
            encoder: Encoder::X264,
            x264_preset: preset.x264_preset.to_string(),
            container: Container::Mkv,
            capture_engine: CaptureEngine::Auto,
            preview_enabled: false,
            output_dir: default_output_dir(),
            audio_bitrate_kbps: 128,
            keyframe_sec: 2,
            stream_targets: vec![StreamTarget::default()],
            active_target: Some(0),
        }
    }
}

impl Settings {
    pub fn active_stream_target(&self) -> Option<&StreamTarget> {
        let id = self.active_target?;
        self.stream_targets.iter().find(|t| t.id == id)
    }

    pub fn apply_mode(&mut self, mode: Mode) {
        let preset = mode.preset();
        self.mode = mode;
        self.canvas_w = preset.width;
        self.canvas_h = preset.height;
        self.fps = preset.fps;
        self.bitrate_kbps = preset.bitrate_kbps;
        self.x264_preset = preset.x264_preset.to_string();
    }

    pub fn tune(&self) -> Option<&'static str> {
        self.mode.preset().tune
    }
}

pub fn default_output_dir() -> String {
    let base = dirs::video_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("ZAPL Recorder").to_string_lossy().to_string()
}
