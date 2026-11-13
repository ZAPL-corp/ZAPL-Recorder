//! Захват и кодирование через ffmpeg: сборка команд, сессии, предпросмотр.

pub mod ffmpeg;
pub mod graph;
pub mod preview;
pub mod session;

use std::path::{Path, PathBuf};

use crate::model::ScreenRect;

pub use graph::{build_preview, build_record, CapturePlan, Destination};
pub use preview::{Preview, PreviewFrame};
pub use session::{Session, SessionKind};

pub fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

pub fn sanitize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
    for ch in name.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "scene".to_string()
    } else {
        trimmed
    }
}

/// Открывает папку в системном файловом менеджере.
pub fn open_folder(path: &Path) -> Result<(), String> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        let mut cmd = Command::new("explorer");
        cmd.arg(path);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.spawn().map_err(|e| format!("explorer: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Открывает URL в отдельном окне Chromium-браузера в заданном регионе.
///
/// Это нужно браузерному источнику: ffmpeg не умеет рендерить HTML, поэтому
/// сцена захватывает область экрана, куда мы ставим окно браузера.
pub fn open_browser(url: &str, rect: ScreenRect) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("У источника не задан адрес страницы.".to_string());
    }
    let profile = browser_profile_dir();
    let _ = std::fs::create_dir_all(&profile);

    #[cfg(windows)]
    {
        let mut candidates: Vec<PathBuf> = Vec::new();
        for var in ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"] {
            if let Some(base) = std::env::var_os(var) {
                let base = PathBuf::from(base);
                candidates.push(
                    base.join("Microsoft")
                        .join("Edge")
                        .join("Application")
                        .join("msedge.exe"),
                );
                candidates.push(
                    base.join("Google")
                        .join("Chrome")
                        .join("Application")
                        .join("chrome.exe"),
                );
            }
        }
        for candidate in candidates {
            if candidate.is_file() {
                return spawn_browser(&candidate, url, rect, &profile);
            }
        }
        if let Some(path) = find_in_path("msedge.exe").or_else(|| find_in_path("chrome.exe")) {
            return spawn_browser(&path, url, rect, &profile);
        }
        open_url_default(url)?;
        Err(
            "Chromium/Chrome/Edge не найден: страница открыта в браузере по умолчанию, \
             но окно не удалось поставить в нужный регион. Используйте источник «Окно»."
                .to_string(),
        )
    }
    #[cfg(not(windows))]
    {
        for name in [
            "chromium",
            "chromium-browser",
            "google-chrome",
            "google-chrome-stable",
            "brave-browser",
            "microsoft-edge",
        ] {
            if let Some(path) = find_in_path(name) {
                return spawn_browser(&path, url, rect, &profile);
            }
        }
        open_url_default(url)?;
        Err(
            "Chromium/Chrome не найден: страница открыта браузером по умолчанию, \
             но окно не удалось поставить в нужный регион. Используйте источник «Окно»."
                .to_string(),
        )
    }
}

fn browser_profile_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ZAPL-Recorder")
        .join("browser-profile")
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn spawn_browser(exe: &Path, url: &str, rect: ScreenRect, profile: &Path) -> Result<(), String> {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(format!("--app={url}"))
        .arg(format!(
            "--window-size={},{}",
            rect.w.max(320),
            rect.h.max(240)
        ))
        .arg(format!(
            "--window-position={},{}",
            rect.x.max(0),
            rect.y.max(0)
        ))
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.spawn()
        .map_err(|e| format!("Не удалось запустить браузер {}: {e}", exe.display()))?;
    Ok(())
}

fn open_url_default(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("Не удалось открыть URL: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Не удалось открыть URL: {e}"))?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Не удалось открыть URL: {e}"))?;
        Ok(())
    }
}
