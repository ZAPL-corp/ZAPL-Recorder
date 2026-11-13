//! Загрузка и сохранение конфигурации в JSON.

use std::fs;
use std::path::PathBuf;

use crate::model::{
    default_output_dir, AudioKind, AudioSource, CanvasRect, Id, Scene, Settings, Source, StreamTarget,
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub settings: Settings,
    pub scenes: Vec<Scene>,
    pub active_scene: Id,
    pub audio: Vec<AudioSource>,
    pub next_id: Id,
}

impl Default for Config {
    fn default() -> Self {
        let mut next_id = 1;
        let mut take = || {
            let id = next_id;
            next_id += 1;
            id
        };

        let settings = Settings::default();
        let display = Source::display(take(), settings.canvas_w, settings.canvas_h);

        let mut scene = Scene {
            id: take(),
            name: "Рабочий стол".to_string(),
            sources: vec![display],
        };
        scene.sources[0].canvas = CanvasRect::full(settings.canvas_w, settings.canvas_h);

        let mic = AudioSource {
            id: take(),
            name: "Микрофон".to_string(),
            kind: AudioKind::Mic,
            device: default_mic_device(),
            volume: 1.0,
            muted: false,
            enabled: default_mic_enabled(),
        };
        let system = AudioSource {
            id: take(),
            name: "Системный звук".to_string(),
            kind: AudioKind::Desktop,
            device: default_desktop_device(),
            volume: 0.8,
            muted: false,
            enabled: false,
        };

        let twitch_id = take();
        let settings = Settings {
            stream_targets: vec![StreamTarget {
                id: twitch_id,
                platform: crate::model::Platform::Twitch,
                name: "Twitch".to_string(),
                url: crate::model::Platform::Twitch.default_url().to_string(),
                key: String::new(),
            }],
            active_target: Some(twitch_id),
            ..settings
        };

        Self {
            version: 1,
            settings,
            active_scene: scene.id,
            scenes: vec![scene],
            audio: vec![mic, system],
            next_id,
        }
    }
}

impl Config {
    pub fn next_id(&mut self) -> Id {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        id
    }

    pub fn scene(&self, id: Id) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.id == id)
    }

    pub fn scene_mut(&mut self, id: Id) -> Option<&mut Scene> {
        self.scenes.iter_mut().find(|s| s.id == id)
    }

    pub fn active_scene(&self) -> Option<&Scene> {
        self.scene(self.active_scene).or_else(|| self.scenes.first())
    }

    pub fn active_scene_mut(&mut self) -> Option<&mut Scene> {
        let id = self.active_scene;
        if self.scenes.iter().any(|s| s.id == id) {
            self.scene_mut(id)
        } else {
            let first = self.scenes.first().map(|s| s.id)?;
            self.active_scene = first;
            self.scene_mut(first)
        }
    }

    /// Приводит идентификаторы и ссылки в согласованное состояние после загрузки.
    pub fn normalize(&mut self) {
        if self.scenes.is_empty() {
            let (canvas_w, canvas_h) = (self.settings.canvas_w, self.settings.canvas_h);
            let scene_id = self.next_id();
            let source_id = self.next_id();
            self.scenes.push(Scene {
                id: scene_id,
                name: "Сцена 1".to_string(),
                sources: vec![Source::display(source_id, canvas_w, canvas_h)],
            });
            self.active_scene = scene_id;
        }
        if self.active_scene == 0 || !self.scenes.iter().any(|s| s.id == self.active_scene) {
            self.active_scene = self.scenes[0].id;
        }

        let mut max_id = self.next_id.saturating_sub(1);
        for scene in &self.scenes {
            max_id = max_id.max(scene.id);
            for source in &scene.sources {
                max_id = max_id.max(source.id);
            }
        }
        for audio in &self.audio {
            max_id = max_id.max(audio.id);
        }
        for target in &self.settings.stream_targets {
            max_id = max_id.max(target.id);
        }
        self.next_id = max_id + 1;

        if self.audio.is_empty() {
            self.audio.push(AudioSource::default());
        }
        if self.settings.stream_targets.is_empty() {
            self.settings.stream_targets.push(StreamTarget::default());
        }
        if self.settings.output_dir.trim().is_empty() {
            self.settings.output_dir = default_output_dir();
        }
        if !self
            .settings
            .active_target
            .map(|id| self.settings.stream_targets.iter().any(|t| t.id == id))
            .unwrap_or(false)
        {
            self.settings.active_target = self.settings.stream_targets.first().map(|t| t.id);
        }
        if self.settings.canvas_w < 320 {
            self.settings.canvas_w = 1280;
        }
        if self.settings.canvas_h < 240 {
            self.settings.canvas_h = 720;
        }
        if self.settings.fps == 0 || self.settings.fps > 240 {
            self.settings.fps = 30;
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ZAPL-Recorder")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn config_example_path() -> PathBuf {
    config_dir().join("config.example.json")
}

pub fn load() -> Config {
    let path = config_path();
    let mut cfg = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Config>(&text).unwrap_or_else(|err| {
            eprintln!("Конфиг повреждён ({err}), беру настройки по умолчанию");
            Config::default()
        }),
        Err(_) => Config::default(),
    };
    cfg.normalize();
    cfg
}

pub fn save(cfg: &Config) -> Result<PathBuf, String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Не удалось создать {}: {e}", dir.display()))?;
    let path = config_path();
    let text = serde_json::to_string_pretty(cfg).map_err(|e| format!("JSON: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("Не удалось записать {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

pub fn save_example_if_missing(cfg: &Config) {
    let path = config_example_path();
    if !path.exists() {
        if let Ok(text) = serde_json::to_string_pretty(cfg) {
            let _ = fs::write(path, text);
        }
    }
}

fn default_mic_enabled() -> bool {
    false
}

#[cfg(windows)]
fn default_mic_device() -> String {
    "audio=Микрофон".to_string()
}

#[cfg(not(windows))]
fn default_mic_device() -> String {
    "default".to_string()
}

#[cfg(windows)]
fn default_desktop_device() -> String {
    "audio=virtual-audio-capturer".to_string()
}

#[cfg(not(windows))]
fn default_desktop_device() -> String {
    "default".to_string()
}
