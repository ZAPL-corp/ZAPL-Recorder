//! РњРѕРґРµР»СЊ РґР°РЅРЅС‹С…: СЃС†РµРЅС‹, РёСЃС‚РѕС‡РЅРёРєРё, Р°СѓРґРёРѕ, РЅР°СЃС‚СЂРѕР№РєРё РІС‹РІРѕРґР° Рё СЃС‚СЂРёРјР°.

use serde::{Deserialize, Serialize};

pub type Id = u64;

// ---------------------------------------------------------------------------
// РСЃС‚РѕС‡РЅРёРєРё
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
            SourceKind::Display => "Р—Р°С…РІР°С‚ СЌРєСЂР°РЅР°",
            SourceKind::Window => "РћРєРЅРѕ",
            SourceKind::Browser => "Р‘СЂР°СѓР·РµСЂ",
            SourceKind::Image => "РљР°СЂС‚РёРЅРєР°",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            SourceKind::Display => "Р­РљР РђРќ",
            SourceKind::Window => "РћРљРќРћ",
            SourceKind::Browser => "WEB",
            SourceKind::Image => "IMG",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            SourceKind::Display => "Р’РµСЃСЊ РјРѕРЅРёС‚РѕСЂ РёР»Рё РµРіРѕ РёРЅРґРµРєСЃ (0 вЂ” РѕСЃРЅРѕРІРЅРѕР№).",
            SourceKind::Window => "Р§Р°СЃС‚СЊ Р·Р°РіРѕР»РѕРІРєР° РѕРєРЅР°, РєРѕС‚РѕСЂРѕРµ РЅСѓР¶РЅРѕ Р·Р°С…РІР°С‚РёС‚СЊ.",
            SourceKind::Browser => "URL, РѕС‚РєСЂС‹РІР°РµРјС‹Р№ РІ РѕС‚РґРµР»СЊРЅРѕРј РѕРєРЅРµ Р±СЂР°СѓР·РµСЂР°.",
            SourceKind::Image => "РџСѓС‚СЊ Рє PNG/JPG вЂ” Р»РѕРіРѕС‚РёРї РёР»Рё РѕРІРµСЂР»РµР№.",
        }
    }
}

/// РљР°РґСЂ РёСЃС‚РѕС‡РЅРёРєР° РЅР° СЌРєСЂР°РЅРµ: РЅСѓР¶РµРЅ РґР»СЏ РѕРєРѕРЅ Рё Р±СЂР°СѓР·РµСЂР° (СЂРµРіРёРѕРЅ Р·Р°С…РІР°С‚Р°),
/// Р° С‚Р°РєР¶Рµ С‡С‚РѕР±С‹ РѕС‚РєСЂС‹С‚СЊ РѕРєРЅРѕ Р±СЂР°СѓР·РµСЂР° РІ РїСЂРµРґСЃРєР°Р·СѓРµРјРѕРј РјРµСЃС‚Рµ.
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

/// РџРѕР»РѕР¶РµРЅРёРµ РёСЃС‚РѕС‡РЅРёРєР° РЅР° С…РѕР»СЃС‚Рµ СЃС†РµРЅС‹.
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
            w: 1920,
            h: 1080,
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
    /// РњРѕРЅРёС‚РѕСЂ, Р·Р°РіРѕР»РѕРІРѕРє РѕРєРЅР°, URL РёР»Рё РїСѓС‚СЊ Рє РєР°СЂС‚РёРЅРєРµ.
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
            name: "Р—Р°С…РІР°С‚ СЌРєСЂР°РЅР°".to_string(),
            kind: SourceKind::Display,
            visible: true,
            target: "0".to_string(),
            canvas: CanvasRect::full(canvas_w, canvas_h),
            ..Default::default()
        }
    }

    pub fn of_kind(id: Id, kind: SourceKind) -> Self {
        let (name, target) = match kind {
            SourceKind::Display => ("Р—Р°С…РІР°С‚ СЌРєСЂР°РЅР°".to_string(), "0".to_string()),
            SourceKind::Window => ("РћРєРЅРѕ".to_string(), String::new()),
            SourceKind::Browser => ("Р‘СЂР°СѓР·РµСЂ".to_string(), "https://example.com".to_string()),
            SourceKind::Image => ("РљР°СЂС‚РёРЅРєР°".to_string(), String::new()),
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
            name: "РЎС†РµРЅР°".to_string(),
            sources: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// РђСѓРґРёРѕ
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
            AudioKind::Mic => "РњРёРєСЂРѕС„РѕРЅ",
            AudioKind::Desktop => "РЎРёСЃС‚РµРјРЅС‹Р№ Р·РІСѓРє",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AudioSource {
    pub id: Id,
    pub name: String,
    pub kind: AudioKind,
    /// РЎС‚СЂРѕРєР° СѓСЃС‚СЂРѕР№СЃС‚РІР° РґР»СЏ ffmpeg: `audio=...` (dshow) РёР»Рё `default` (pulse).
    pub device: String,
    pub volume: f32,
    pub muted: bool,
    pub enabled: bool,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            id: 0,
            name: "РњРёРєСЂРѕС„РѕРЅ".to_string(),
            kind: AudioKind::Mic,
            device: String::new(),
            volume: 1.0,
            muted: false,
            enabled: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Р РµР¶РёРјС‹ СЂР°Р±РѕС‚С‹ (РїСЂРѕС„РёР»Рё)
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
            Mode::Game => "Р—Р°РїРёСЃСЊ РёРіСЂС‹",
            Mode::Work => "Р—Р°РїРёСЃСЊ СЂР°Р±РѕС‚С‹",
            Mode::WorkStream => "РЎС‚СЂРёРј СЂР°Р±РѕС‚С‹",
            Mode::GameStream => "РЎС‚СЂРёРј РёРіСЂС‹",
            Mode::Ai => "Р—Р°РїРёСЃСЊ РР",
            Mode::AiStream => "РЎС‚СЂРёРј РР",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Mode::Game => "1080p60, РІС‹СЃРѕРєРёР№ Р±РёС‚СЂРµР№С‚, РјРёРЅРёРјСѓРј Р°СЂС‚РµС„Р°РєС‚РѕРІ РІ РґРІРёР¶РµРЅРёРё.",
            Mode::Work => "1080p30, СЂРµР·РєРёР№ С‚РµРєСЃС‚: tune stillimage, РЅРµР±РѕР»СЊС€РѕР№ Р±РёС‚СЂРµР№С‚.",
            Mode::WorkStream => "900p30, 3.5 РњР±РёС‚/СЃ вЂ” СЃС‚Р°Р±РёР»СЊРЅРѕ РґР»СЏ СЃР»Р°Р±РѕРіРѕ РёРЅС‚РµСЂРЅРµС‚Р°.",
            Mode::GameStream => "1080p60, 6 РњР±РёС‚/СЃ, РєР»СЋС‡РµРІРѕР№ РєР°РґСЂ РєР°Р¶РґС‹Рµ 2 СЃРµРєСѓРЅРґС‹.",
            Mode::Ai => "1080p30, СѓРїРѕСЂ РЅР° С‡РёС‚Р°РµРјРѕСЃС‚СЊ РєРѕРґР° Рё С‚Р°Р±Р»РёС† РІ РґР»РёРЅРЅС‹С… СЃРµСЃСЃРёСЏС….",
            Mode::AiStream => "1080p30, 5 РњР±РёС‚/СЃ вЂ” СЃРєСЂРёРЅРєР°СЃС‚ РЅРµР№СЂРѕСЃРµС‚РµР№ Рё С‡Р°С‚РѕРІ.",
        }
    }

    fn preset(self) -> ModePreset {
        match self {
            Mode::Game => ModePreset {
                width: 1920,
                height: 1080,
                fps: 60,
                bitrate_kbps: 12_000,
                x264_preset: "superfast",
                tune: None,
            },
            Mode::Work => ModePreset {
                width: 1920,
                height: 1080,
                fps: 30,
                bitrate_kbps: 6_000,
                x264_preset: "veryfast",
                tune: Some("stillimage"),
            },
            Mode::WorkStream => ModePreset {
                width: 1600,
                height: 900,
                fps: 30,
                bitrate_kbps: 3_500,
                x264_preset: "veryfast",
                tune: Some("stillimage"),
            },
            Mode::GameStream => ModePreset {
                width: 1920,
                height: 1080,
                fps: 60,
                bitrate_kbps: 6_000,
                x264_preset: "superfast",
                tune: None,
            },
            Mode::Ai => ModePreset {
                width: 1920,
                height: 1080,
                fps: 30,
                bitrate_kbps: 7_000,
                x264_preset: "veryfast",
                tune: Some("stillimage"),
            },
            Mode::AiStream => ModePreset {
                width: 1920,
                height: 1080,
                fps: 30,
                bitrate_kbps: 5_000,
                x264_preset: "veryfast",
                tune: Some("stillimage"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// РќР°СЃС‚СЂРѕР№РєРё РІС‹РІРѕРґР°
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
            Encoder::X264 => "Р Р°Р±РѕС‚Р°РµС‚ РІСЃРµРіРґР°. РќР° СЃР»Р°Р±РѕРј CPU СЃС‚Р°РІСЊС‚Рµ preset ultrafast.",
            Encoder::Nvenc => "РџРѕС‡С‚Рё РЅРµ РіСЂСѓР·РёС‚ РїСЂРѕС†РµСЃСЃРѕСЂ, РЅСѓР¶РЅР° РІРёРґРµРѕРєР°СЂС‚Р° NVIDIA.",
            Encoder::Qsv => "РђРїРїР°СЂР°С‚РЅС‹Р№ Intel, С…РѕСЂРѕС€Рѕ РґР»СЏ РЅРѕСѓС‚Р±СѓРєРѕРІ Рё РјРёРЅРё-РџРљ.",
            Encoder::Amf => "РђРїРїР°СЂР°С‚РЅС‹Р№ AMD, РґСЂР°Р№РІРµСЂ Adrenalin.",
            Encoder::Vaapi => "Linux, /dev/dri/renderD128, РЅСѓР¶РµРЅ ffmpeg СЃ VAAPI.",
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
            Container::Mkv => "MKV (СѓСЃС‚РѕР№С‡РёРІ Рє РїР°РґРµРЅРёСЋ)",
            Container::Mp4 => "MP4 (СЃРѕРІРјРµСЃС‚РёРј РІРµР·РґРµ)",
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
            CaptureEngine::Auto => "РђРІС‚Рѕ",
            CaptureEngine::Dxgi => "DXGI Desktop Duplication (GPU)",
            CaptureEngine::Gdi => "GDI (СЃРѕРІРјРµСЃС‚РёРјРѕ СЃРѕ РІСЃРµРј)",
            CaptureEngine::X11 => "X11 / XShm",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            CaptureEngine::Auto => {
                "DXGI, РµСЃР»Рё ffmpeg РµРіРѕ СѓРјРµРµС‚, РёРЅР°С‡Рµ GDI. РќР° Linux вЂ” X11."
            }
            CaptureEngine::Dxgi => "РЎР°РјС‹Р№ РґРµС€С‘РІС‹Р№ Р·Р°С…РІР°С‚ РЅР° Windows: РєР°РґСЂС‹ Р·Р°Р±РёСЂР°РµС‚ GPU.",
            CaptureEngine::Gdi => "Р Р°Р±РѕС‚Р°РµС‚ РІСЃРµРіРґР°, РЅРѕ Р·Р°РјРµС‚РЅРѕ РіСЂСѓР·РёС‚ CPU РЅР° СЃР»Р°Р±С‹С… РјР°С€РёРЅР°С….",
            CaptureEngine::X11 => "Linux СЃ X11. Wayland-СЃРµСЃСЃРёРё С‚СЂРµР±СѓСЋС‚ XWayland.",
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
            Platform::Custom => "РЎРІРѕР№ RTMP",
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
            "РєР»СЋС‡ РЅРµ Р·Р°РґР°РЅ".to_string()
        } else if key.len() <= 8 {
            "вЂў".repeat(key.len())
        } else {
            format!("{}вЂўвЂўвЂўвЂў{}", &key[..4], &key[key.len() - 4..])
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
            preview_enabled: true,
            output_dir: default_output_dir(),
            audio_bitrate_kbps: 128,
            keyframe_sec: 2,
            stream_targets: vec![StreamTarget::default()],
            active_target: Some(1),
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
