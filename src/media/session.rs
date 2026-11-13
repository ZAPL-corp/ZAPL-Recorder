//! Запуск ffmpeg, чтение его лога, статистика и корректная остановка.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use crate::media::ffmpeg;
use crate::media::graph::CapturePlan;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionKind {
    Recording,
    Streaming,
}

impl SessionKind {
    pub fn label(self) -> &'static str {
        match self {
            SessionKind::Recording => "Запись",
            SessionKind::Streaming => "Стрим",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SessionStats {
    pub fps: Option<f32>,
    pub bitrate_kbps: Option<f32>,
    pub dropped: u32,
    pub frame: u64,
    pub time: Option<String>,
    pub speed: Option<f32>,
    pub size_kb: Option<u64>,
}

impl SessionStats {
    pub fn bitrate_text(&self) -> String {
        match self.bitrate_kbps {
            Some(value) => format!("{value:.0} кбит/с"),
            None => "—".to_string(),
        }
    }

    pub fn fps_text(&self) -> String {
        match self.fps {
            Some(value) => format!("{value:.1} fps"),
            None => "— fps".to_string(),
        }
    }
}

pub enum SessionEvent {
    Stats(SessionStats),
    Log(String),
}

pub struct Session {
    pub kind: SessionKind,
    pub title: String,
    pub output: Option<PathBuf>,
    pub started: Instant,
    pub stats: SessionStats,
    pub log: VecDeque<String>,
    pub exit: Option<String>,
    child: Child,
    stdin: Option<ChildStdin>,
    rx: Receiver<SessionEvent>,
}

impl Session {
    pub fn start(plan: &CapturePlan, kind: SessionKind, title: String) -> Result<Self, String> {
        let mut cmd = ffmpeg::command(&plan.ffmpeg);
        cmd.args(&plan.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Не удалось запустить ffmpeg: {e}\nПроверьте, что {} существует и запускается.",
                plan.ffmpeg.display()
            )
        })?;

        let stdin = child.stdin.take();
        let stderr = child.stderr.take();
        let (tx, rx) = channel();

        if let Some(stderr) = stderr {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    match parse_stats(&line) {
                        Some(stats) => {
                            let _ = tx.send(SessionEvent::Stats(stats));
                        }
                        None => {
                            let _ = tx.send(SessionEvent::Log(line));
                        }
                    }
                }
            });
        }

        Ok(Self {
            kind,
            title,
            output: plan.output.clone(),
            started: Instant::now(),
            stats: SessionStats::default(),
            log: VecDeque::new(),
            exit: None,
            child,
            stdin,
            rx,
        })
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn poll(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                SessionEvent::Stats(stats) => self.stats = stats,
                SessionEvent::Log(line) => {
                    self.log.push_back(line);
                    while self.log.len() > 300 {
                        self.log.pop_front();
                    }
                }
            }
        }
        if self.exit.is_none() {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.exit = Some(describe_exit(&status));
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.exit.is_some()
    }

    /// Мягкая остановка: `q` в stdin, затем kill по таймауту.
    pub fn stop(&mut self) {
        if self.exit.is_some() {
            return;
        }
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.write_all(b"q");
            let _ = stdin.flush();
        }
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.exit = Some(describe_exit(&status));
                    break;
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        self.exit = Some("Остановлено принудительно".to_string());
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(60));
                }
                Err(err) => {
                    self.exit = Some(format!("Ошибка ожидания ffmpeg: {err}"));
                    break;
                }
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.exit.is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn describe_exit(status: &std::process::ExitStatus) -> String {
    if status.success() {
        "ffmpeg завершил работу".to_string()
    } else {
        match status.code() {
            Some(code) => format!("ffmpeg завершился с кодом {code}"),
            None => "ffmpeg аварийно завершился".to_string(),
        }
    }
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let start = line.find(key)? + key.len();
    let rest = line[start..].trim_start();
    let end = rest
        .find(|c: char| c.is_whitespace())
        .unwrap_or(rest.len());
    Some(rest[..end].trim())
}

fn number(value: &str) -> Option<f32> {
    let cleaned: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    if cleaned.is_empty() || cleaned == "-" || cleaned == "." {
        None
    } else {
        cleaned.parse().ok()
    }
}

/// Разбирает строку прогресса ffmpeg вида
/// `frame= 120 fps= 30 q=28.0 size=  5120kB time=00:00:04.00 bitrate=10485.8kbits/s speed=1x`
pub fn parse_stats(line: &str) -> Option<SessionStats> {
    if !line.contains("time=") && !line.contains("bitrate=") {
        return None;
    }
    let mut stats = SessionStats::default();
    if let Some(value) = field(line, "fps=") {
        stats.fps = number(value);
    }
    if let Some(value) = field(line, "bitrate=") {
        stats.bitrate_kbps = number(value);
    }
    if let Some(value) = field(line, "drop=") {
        stats.dropped = number(value).unwrap_or(0.0) as u32;
    }
    if let Some(value) = field(line, "frame=") {
        stats.frame = number(value).unwrap_or(0.0) as u64;
    }
    if let Some(value) = field(line, "time=") {
        if !value.starts_with("N/A") && !value.is_empty() {
            stats.time = Some(value.to_string());
        }
    }
    if let Some(value) = field(line, "speed=") {
        stats.speed = number(value);
    }
    if let Some(value) = field(line, "size=") {
        stats.size_kb = number(value).map(|v| v as u64);
    }
    Some(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_line() {
        let line = "frame=  120 fps= 29.9 q=28.0 size=    5120kB time=00:00:04.01 bitrate=10485.8kbits/s speed=1.01x";
        let stats = parse_stats(line).expect("строка прогресса распознана");
        assert_eq!(stats.frame, 120);
        assert!((stats.fps.unwrap() - 29.9).abs() < 0.01);
        assert!((stats.bitrate_kbps.unwrap() - 10485.8).abs() < 0.1);
        assert_eq!(stats.time.as_deref(), Some("00:00:04.01"));
        assert_eq!(stats.size_kb, Some(5120));
    }

    #[test]
    fn ignores_regular_log_lines() {
        assert!(parse_stats("Press [q] to stop, [?] for help").is_none());
        assert!(parse_stats("Output #0, matroska, to 'out.mkv':").is_none());
    }
}
