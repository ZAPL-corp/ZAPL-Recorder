//! Поиск бинарника ffmpeg и проверка его возможностей.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// Ищет ffmpeg рядом с программой (`bin/ffmpeg`, затем корень), в папке
/// запуска и в PATH.
pub fn locate() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("bin").join(binary_name()));
            candidates.push(dir.join(binary_name()));
            candidates.push(dir.join("tools").join("ffmpeg").join(binary_name()));
        }
    }
    candidates.push(PathBuf::from("bin").join(binary_name()));
    candidates.push(PathBuf::from(binary_name()));

    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            let candidate = dir.join(binary_name());
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Команда ffmpeg без лишнего консольного окна на Windows.
pub fn command(ffmpeg: &Path) -> Command {
    let mut cmd = Command::new(ffmpeg);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn run(ffmpeg: &Path, args: &[&str]) -> Option<Output> {
    command(ffmpeg).args(args).output().ok()
}

fn combined(out: &Output) -> String {
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text
}

pub fn version(ffmpeg: &Path) -> Option<String> {
    let out = run(ffmpeg, &["-hide_banner", "-version"])?;
    combined(&out)
        .lines()
        .next()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
}

#[derive(Clone, Debug, Default)]
pub struct Probe {
    pub version: Option<String>,
    pub encoders: String,
    pub filters: String,
    pub audio_devices: Vec<String>,
}

impl Probe {
    pub fn has_encoder(&self, codec: &str) -> bool {
        self.encoders.contains(codec)
    }

    pub fn has_filter(&self, name: &str) -> bool {
        self.filters.contains(name)
    }

    pub fn has_ddagrab(&self) -> bool {
        cfg!(windows) && self.has_filter("ddagrab")
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.has_encoder("h264_nvenc") {
            parts.push("NVENC");
        }
        if self.has_encoder("h264_qsv") {
            parts.push("QuickSync");
        }
        if self.has_encoder("h264_amf") {
            parts.push("AMF");
        }
        if self.has_encoder("h264_vaapi") {
            parts.push("VAAPI");
        }
        if self.has_ddagrab() {
            parts.push("DXGI");
        }
        if parts.is_empty() {
            "x264".to_string()
        } else {
            parts.join(" · ")
        }
    }
}

/// Устройства ввода звука. Windows — через dshow, Linux — через pactl.
pub fn audio_devices(ffmpeg: &Path) -> Vec<String> {
    #[cfg(windows)]
    {
        let out = match run(
            ffmpeg,
            &[
                "-hide_banner",
                "-list_devices",
                "true",
                "-f",
                "dshow",
                "-i",
                "dummy",
            ],
        ) {
            Some(out) => out,
            None => return Vec::new(),
        };
        let text = String::from_utf8_lossy(&out.stderr).to_string();
        let mut devices = Vec::new();
        for line in text.lines() {
            if !line.contains("(audio)") {
                continue;
            }
            if let Some(start) = line.find('"') {
                if let Some(len) = line[start + 1..].find('"') {
                    let name = &line[start + 1..start + 1 + len];
                    if !name.is_empty() {
                        devices.push(format!("audio={name}"));
                    }
                }
            }
        }
        devices
    }
    #[cfg(not(windows))]
    {
        let _ = ffmpeg;
        let out = match Command::new("pactl")
            .args(["list", "short", "sources"])
            .output()
        {
            Ok(out) => out,
            Err(_) => return Vec::new(),
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let mut devices = Vec::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let _index = parts.next();
            if let Some(name) = parts.next() {
                devices.push(name.to_string());
            }
        }
        devices
    }
}

pub fn probe(ffmpeg: &Path) -> Probe {
    let version = version(ffmpeg);
    let encoders = run(ffmpeg, &["-hide_banner", "-encoders"])
        .map(|out| combined(&out))
        .unwrap_or_default();
    let filters = run(ffmpeg, &["-hide_banner", "-filters"])
        .map(|out| combined(&out))
        .unwrap_or_default();
    Probe {
        version,
        encoders,
        filters,
        audio_devices: audio_devices(ffmpeg),
    }
}
