//! Сборка команд ffmpeg для записи, стрима и предпросмотра.
//!
//! Композиция сцены повторяет логику OBS: снизу чёрный холст, затем источники
//! от нижнего к верхнему (в списке UI первый источник — самый верхний).

use std::path::{Path, PathBuf};

use crate::media::{sanitize_name, timestamp};
use crate::model::{
    AudioSource, CanvasRect, CaptureEngine, Container, Encoder, Scene, Settings, Source, SourceKind,
};

pub const PREVIEW_FPS: u32 = 5;
pub const PREVIEW_WIDTH: u32 = 720;

#[derive(Clone, Debug)]
pub enum Destination {
    File(PathBuf),
    Stream(String),
}

#[derive(Clone, Debug)]
pub struct CapturePlan {
    pub ffmpeg: PathBuf,
    pub args: Vec<String>,
    pub output: Option<PathBuf>,
    pub label: String,
}

impl CapturePlan {
    pub fn command_line(&self) -> String {
        let mut parts = vec![format!("\"{}\"", self.ffmpeg.display())];
        for arg in &self.args {
            if arg.contains(' ') || arg.is_empty() {
                parts.push(format!("\"{arg}\""));
            } else {
                parts.push(arg.clone());
            }
        }
        parts.join(" ")
    }
}

#[derive(Clone, Debug)]
pub struct PreviewPlan {
    pub ffmpeg: PathBuf,
    pub args: Vec<String>,
    pub source_name: String,
}

pub struct PlanInput<'a> {
    pub ffmpeg: &'a Path,
    pub settings: &'a Settings,
    pub scene: &'a Scene,
    pub audio: &'a [AudioSource],
    pub destination: &'a Destination,
    pub has_ddagrab: bool,
}

fn even(value: u32) -> u32 {
    let value = value.max(2);
    value & !1
}

pub fn effective_engine(settings: &Settings, has_ddagrab: bool) -> CaptureEngine {
    let requested = settings.capture_engine;
    if cfg!(windows) {
        match requested {
            CaptureEngine::Auto | CaptureEngine::Dxgi if has_ddagrab => CaptureEngine::Dxgi,
            CaptureEngine::X11 => CaptureEngine::Gdi,
            CaptureEngine::Dxgi => CaptureEngine::Gdi,
            _ => CaptureEngine::Gdi,
        }
    } else {
        CaptureEngine::X11
    }
}

pub fn record_path(settings: &Settings, scene_name: &str) -> PathBuf {
    let dir = PathBuf::from(&settings.output_dir);
    let name = format!(
        "{}_{}_{}.{}",
        sanitize_name(scene_name),
        sanitize_name(settings.mode.label()),
        timestamp(),
        settings.container.ext()
    );
    dir.join(name)
}

fn rect_dims(rect: CanvasRect, canvas_w: u32, canvas_h: u32) -> (u32, u32) {
    let w = even(rect.w.max(2).min(canvas_w.max(2) * 4));
    let h = even(rect.h.max(2).min(canvas_h.max(2) * 4));
    (w, h)
}

/// Аргументы входа и префильтр (например `hwdownload,format=bgra`) для источника.
fn source_input(
    source: &Source,
    settings: &Settings,
    engine: CaptureEngine,
    has_ddagrab: bool,
) -> Result<(Vec<String>, String), String> {
    let fps = settings.fps.clamp(1, 240).to_string();
    match source.kind {
        SourceKind::Display => {
            #[cfg(windows)]
            {
                let monitor: u32 = source.target.trim().parse::<u32>().unwrap_or(0).min(7);
                if engine == CaptureEngine::Dxgi && has_ddagrab {
                    return Ok((
                        vec![
                            "-f".into(),
                            "lavfi".into(),
                            "-i".into(),
                            format!(
                                "ddagrab=framerate={}:output_idx={}:draw_mouse=1",
                                settings.fps.max(1),
                                monitor
                            ),
                        ],
                        "hwdownload,format=bgra".into(),
                    ));
                }
                Ok((
                    vec![
                        "-f".into(),
                        "gdigrab".into(),
                        "-framerate".into(),
                        fps,
                        "-draw_mouse".into(),
                        "1".into(),
                        "-i".into(),
                        "desktop".into(),
                    ],
                    String::new(),
                ))
            }
            #[cfg(not(windows))]
            {
                let _ = (engine, has_ddagrab);
                let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_string());
                Ok((
                    vec![
                        "-f".into(),
                        "x11grab".into(),
                        "-framerate".into(),
                        fps,
                        "-draw_mouse".into(),
                        "1".into(),
                        "-i".into(),
                        display,
                    ],
                    String::new(),
                ))
            }
        }
        SourceKind::Window => {
            let title = source.target.trim();
            if title.is_empty() {
                return Err(format!(
                    "У источника «{}» не задан заголовок окна.",
                    source.name
                ));
            }
            #[cfg(windows)]
            {
                Ok((
                    vec![
                        "-f".into(),
                        "gdigrab".into(),
                        "-framerate".into(),
                        fps,
                        "-i".into(),
                        format!("title={title}"),
                    ],
                    String::new(),
                ))
            }
            #[cfg(not(windows))]
            {
                let _ = fps;
                let (x, y, w, h) = x11_window_geometry(title)?;
                region_input(x, y, w, h, settings.fps.max(1))
            }
        }
        SourceKind::Browser => {
            if source.target.trim().is_empty() {
                return Err(format!(
                    "У источника «{}» не задан адрес страницы.",
                    source.name
                ));
            }
            let rect = source.screen;
            region_input(rect.x, rect.y, rect.w, rect.h, settings.fps.max(1))
        }
        SourceKind::Image => {
            let path = source.target.trim();
            if path.is_empty() {
                return Err(format!("У источника «{}» не задан файл.", source.name));
            }
            if !Path::new(path).is_file() {
                return Err(format!("Файл «{path}» не найден."));
            }
            Ok((
                vec![
                    "-loop".into(),
                    "1".into(),
                    "-framerate".into(),
                    fps,
                    "-i".into(),
                    path.to_string(),
                ],
                String::new(),
            ))
        }
    }
}

#[cfg(windows)]
fn region_input(x: i32, y: i32, w: u32, h: u32, fps: u32) -> Result<(Vec<String>, String), String> {
    let w = even(w.max(2)).to_string();
    let h = even(h.max(2)).to_string();
    Ok((
        vec![
            "-f".into(),
            "gdigrab".into(),
            "-framerate".into(),
            fps.to_string(),
            "-draw_mouse".into(),
            "1".into(),
            "-offset_x".into(),
            x.max(0).to_string(),
            "-offset_y".into(),
            y.max(0).to_string(),
            "-video_size".into(),
            format!("{w}x{h}"),
            "-i".into(),
            "desktop".into(),
        ],
        String::new(),
    ))
}

#[cfg(not(windows))]
fn region_input(x: i32, y: i32, w: u32, h: u32, fps: u32) -> Result<(Vec<String>, String), String> {
    let w = even(w.max(2)).to_string();
    let h = even(h.max(2)).to_string();
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".to_string());
    let target = format!("{display}+{}+{}", x.max(0), y.max(0));
    Ok((
        vec![
            "-f".into(),
            "x11grab".into(),
            "-framerate".into(),
            fps.to_string(),
            "-video_size".into(),
            format!("{w}x{h}"),
            "-i".into(),
            target,
        ],
        String::new(),
    ))
}

#[cfg(not(windows))]
fn x11_window_geometry(title: &str) -> Result<(i32, i32, u32, u32), String> {
    use std::process::Command;

    let search = Command::new("xdotool")
        .args(["search", "--name", title])
        .output()
        .map_err(|_| {
            "Для захвата окна на Linux нужен xdotool (sudo apt install xdotool). \
             Альтернатива — источник «Захват экрана»."
                .to_string()
        })?;
    let ids = String::from_utf8_lossy(&search.stdout);
    let id = ids
        .split_whitespace()
        .last()
        .ok_or_else(|| format!("Окно с заголовком «{title}» не найдено."))?;

    let geometry = Command::new("xdotool")
        .args(["getwindowgeometry", "--shell", id])
        .output()
        .map_err(|e| format!("xdotool: {e}"))?;
    let text = String::from_utf8_lossy(&geometry.stdout).to_string();

    let mut x = 0;
    let mut y = 0;
    let mut w = 0;
    let mut h = 0;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "X" => x = value.trim().parse().unwrap_or(0),
            "Y" => y = value.trim().parse().unwrap_or(0),
            "WIDTH" => w = value.trim().parse().unwrap_or(0),
            "HEIGHT" => h = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    if w < 16 || h < 16 {
        return Err(format!(
            "Не удалось определить геометрию окна «{title}» (x={x}, y={y}, {w}x{h})."
        ));
    }
    Ok((x, y, w, h))
}

fn audio_input(device: &str) -> Result<Vec<String>, String> {
    let device = device.trim();
    if device.is_empty() {
        return Err("Не выбрано устройство звука.".to_string());
    }
    #[cfg(windows)]
    {
        let value = if device.starts_with("audio=") {
            device.to_string()
        } else {
            format!("audio={device}")
        };
        Ok(vec!["-f".into(), "dshow".into(), "-i".into(), value])
    }
    #[cfg(not(windows))]
    {
        Ok(vec![
            "-f".into(),
            "pulse".into(),
            "-i".into(),
            device.to_string(),
        ])
    }
}

fn valid_preset(preset: &str) -> String {
    const PRESETS: [&str; 8] = [
        "ultrafast",
        "superfast",
        "veryfast",
        "faster",
        "fast",
        "medium",
        "slow",
        "veryslow",
    ];
    let preset = preset.trim();
    if PRESETS.contains(&preset) {
        preset.to_string()
    } else {
        "veryfast".to_string()
    }
}

fn encoder_args(settings: &Settings, gop: u32) -> Vec<String> {
    let bitrate = format!("{}k", settings.bitrate_kbps.max(200));
    let bufsize = format!("{}k", settings.bitrate_kbps.max(200) * 2);
    let gop = gop.max(1).to_string();
    let mut args: Vec<String> = Vec::new();
    match settings.encoder {
        Encoder::X264 => {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                valid_preset(&settings.x264_preset),
            ]);
            if let Some(tune) = settings.tune() {
                args.extend(["-tune".into(), tune.into()]);
            }
            args.extend([
                "-b:v".into(),
                bitrate.clone(),
                "-maxrate".into(),
                bitrate.clone(),
                "-bufsize".into(),
                bufsize,
                "-g".into(),
                gop,
                "-keyint_min".into(),
                settings.fps.max(1).to_string(),
                "-sc_threshold".into(),
                "0".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
            ]);
        }
        Encoder::Nvenc => {
            args.extend([
                "-c:v".into(),
                "h264_nvenc".into(),
                "-preset".into(),
                "p4".into(),
                "-tune".into(),
                "ll".into(),
                "-rc".into(),
                "cbr".into(),
                "-b:v".into(),
                bitrate.clone(),
                "-maxrate".into(),
                bitrate,
                "-bufsize".into(),
                bufsize,
                "-g".into(),
                gop,
                "-pix_fmt".into(),
                "yuv420p".into(),
            ]);
        }
        Encoder::Qsv => {
            args.extend([
                "-c:v".into(),
                "h264_qsv".into(),
                "-preset".into(),
                "veryfast".into(),
                "-b:v".into(),
                bitrate.clone(),
                "-maxrate".into(),
                bitrate,
                "-bufsize".into(),
                bufsize,
                "-g".into(),
                gop,
            ]);
        }
        Encoder::Amf => {
            args.extend([
                "-c:v".into(),
                "h264_amf".into(),
                "-quality".into(),
                "speed".into(),
                "-rc".into(),
                "cbr".into(),
                "-b:v".into(),
                bitrate.clone(),
                "-maxrate".into(),
                bitrate,
                "-bufsize".into(),
                bufsize,
                "-g".into(),
                gop,
            ]);
        }
        Encoder::Vaapi => {
            args.extend([
                "-c:v".into(),
                "h264_vaapi".into(),
                "-b:v".into(),
                bitrate.clone(),
                "-maxrate".into(),
                bitrate,
                "-bufsize".into(),
                bufsize,
                "-g".into(),
                gop,
            ]);
        }
    }
    args
}

pub fn build_record(input: &PlanInput<'_>) -> Result<CapturePlan, String> {
    let settings = input.settings;
    let fps = settings.fps.clamp(1, 240);
    let canvas_w = even(settings.canvas_w.max(320));
    let canvas_h = even(settings.canvas_h.max(240));
    let engine = effective_engine(settings, input.has_ddagrab);

    let visual: Vec<&Source> = input.scene.sources.iter().filter(|s| s.visible).collect();
    if visual.is_empty() {
        return Err(
            "В сцене нет видимых источников. Добавьте «Захват экрана» или другое видео."
                .to_string(),
        );
    }

    if let Destination::File(path) = input.destination {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать {}: {e}", parent.display()))?;
        }
    }

    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "level+info".into(),
        "-stats_period".into(),
        "0.5".into(),
    ];
    if settings.encoder == Encoder::Vaapi {
        args.extend(["-vaapi_device".into(), "/dev/dri/renderD128".into()]);
    }

    // Вход 0 — чёрный холст, темп задаёт реальное время.
    args.extend([
        "-re".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!("color=c=black:s={canvas_w}x{canvas_h}:r={fps}"),
    ]);

    let mut video_inputs: Vec<(usize, String, &Source)> = Vec::new();
    for source in visual.iter().rev() {
        let (src_args, prelude) = source_input(source, settings, engine, input.has_ddagrab)?;
        let index = 1 + video_inputs.len();
        args.extend(src_args);
        video_inputs.push((index, prelude, *source));
    }

    let active_audio: Vec<&AudioSource> = input
        .audio
        .iter()
        .filter(|a| a.enabled && !a.muted)
        .collect();
    let mut audio_inputs: Vec<(usize, &AudioSource)> = Vec::new();
    for audio in &active_audio {
        if audio.device.trim().is_empty() {
            return Err(format!(
                "У звукового источника «{}» не выбрано устройство. Откройте настройки → Аудио.",
                audio.name
            ));
        }
        let index = 1 + video_inputs.len() + audio_inputs.len();
        args.extend(audio_input(&audio.device)?);
        audio_inputs.push((index, audio));
    }

    let mut filters: Vec<String> = Vec::new();
    let mut previous = "0:v".to_string();
    for (n, (index, prelude, source)) in video_inputs.iter().enumerate() {
        let (w, h) = rect_dims(source.canvas, canvas_w, canvas_h);
        let mut chain = String::new();
        if !prelude.is_empty() {
            chain.push_str(prelude);
            chain.push(',');
        }
        chain.push_str(&format!(
            "fps={fps},scale={w}:{h}:force_original_aspect_ratio=decrease:flags=bilinear,\
             pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1"
        ));
        filters.push(format!("[{index}:v]{chain}[s{n}]"));
        filters.push(format!(
            "[{previous}][s{n}]overlay={}:{}:shortest=0:eof_action=pass[o{n}]",
            source.canvas.x, source.canvas.y
        ));
        previous = format!("o{n}");
    }

    let tail = if settings.encoder == Encoder::Vaapi {
        "format=nv12,hwupload"
    } else {
        "format=yuv420p"
    };
    filters.push(format!("[{previous}]{tail}[vout]"));

    for (n, (index, audio)) in audio_inputs.iter().enumerate() {
        let volume = audio.volume.clamp(0.0, 2.0);
        filters.push(format!(
            "[{index}:a]volume={volume:.2},aresample=48000[a{n}]"
        ));
    }
    if audio_inputs.len() == 1 {
        filters.push("[a0]anull[aout]".to_string());
    } else if audio_inputs.len() > 1 {
        let labels: String = (0..audio_inputs.len())
            .map(|n| format!("[a{n}]"))
            .collect::<Vec<_>>()
            .join("");
        filters.push(format!(
            "{labels}amix=inputs={}:normalize=0:dropout_transition=0[aout]",
            audio_inputs.len()
        ));
    }

    args.push("-filter_complex".into());
    args.push(filters.join(";"));

    args.extend(["-map".into(), "[vout]".into()]);
    if !audio_inputs.is_empty() {
        args.extend(["-map".into(), "[aout]".into()]);
    }

    let gop = settings.fps.max(1) * settings.keyframe_sec.max(1);
    args.extend(encoder_args(settings, gop));

    if !audio_inputs.is_empty() {
        args.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            format!("{}k", settings.audio_bitrate_kbps.clamp(64, 512)),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
        ]);
    } else {
        args.push("-an".into());
    }

    args.extend(["-max_muxing_queue_size".into(), "2048".into()]);

    let (output, label) = match input.destination {
        Destination::File(path) => {
            args.extend([
                "-f".into(),
                settings.container.muxer().into(),
                path.to_string_lossy().to_string(),
            ]);
            if settings.container == Container::Mp4 {
                args.extend(["-movflags".into(), "+faststart".into()]);
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            (Some(path.clone()), format!("Файл · {name}"))
        }
        Destination::Stream(url) => {
            args.extend([
                "-f".into(),
                "flv".into(),
                "-flvflags".into(),
                "no_duration_filesize".into(),
                url.clone(),
            ]);
            (None, "Стрим".to_string())
        }
    };

    Ok(CapturePlan {
        ffmpeg: input.ffmpeg.to_path_buf(),
        args,
        output,
        label,
    })
}

pub fn build_preview(
    ffmpeg: &Path,
    settings: &Settings,
    scene: &Scene,
    has_ddagrab: bool,
) -> Result<PreviewPlan, String> {
    let source = scene
        .sources
        .iter()
        .find(|s| s.visible)
        .ok_or_else(|| "В сцене нет видимых источников.".to_string())?;
    let engine = effective_engine(settings, has_ddagrab);
    let (input_args, prelude) = source_input(source, settings, engine, has_ddagrab)?;

    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostats".into(),
        "-an".into(),
    ];
    args.extend(input_args);

    let mut vf = String::new();
    if !prelude.is_empty() {
        vf.push_str(&prelude);
        vf.push(',');
    }
    vf.push_str(&format!(
        "fps={PREVIEW_FPS},scale={PREVIEW_WIDTH}:-2:flags=fast_bilinear"
    ));
    args.extend([
        "-vf".into(),
        vf,
        "-c:v".into(),
        "mjpeg".into(),
        "-q:v".into(),
        "7".into(),
        "-f".into(),
        "mjpeg".into(),
        "pipe:1".into(),
    ]);

    Ok(PreviewPlan {
        ffmpeg: ffmpeg.to_path_buf(),
        args,
        source_name: source.name.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AudioSource;

    fn two_display_scene() -> Scene {
        let mut scene = Scene::default();
        scene.id = 1;
        scene.name = "Тест".to_string();
        let mut lower = Source::display(2, 1920, 1080);
        lower.target = "0".to_string();
        let mut upper = Source::display(3, 1920, 1080);
        upper.target = "1".to_string();
        upper.canvas = CanvasRect {
            x: 120,
            y: 80,
            w: 960,
            h: 540,
        };
        scene.sources = vec![upper, lower];
        scene
    }

    fn plan_for(
        scene: &Scene,
        audio: &[AudioSource],
        destination: &Destination,
    ) -> Result<CapturePlan, String> {
        let settings = Settings::default();
        let input = PlanInput {
            ffmpeg: Path::new("ffmpeg"),
            settings: &settings,
            scene,
            audio,
            destination,
            has_ddagrab: false,
        };
        build_record(&input)
    }

    #[test]
    fn composite_graph_has_overlays_and_maps() {
        let scene = two_display_scene();
        let dir = std::env::temp_dir().join("zapl-recorder-tests");
        let destination = Destination::File(dir.join("test-out.mkv"));
        let plan = plan_for(&scene, &[], &destination).expect("план записи строится");

        let index = plan
            .args
            .iter()
            .position(|arg| arg == "-filter_complex")
            .expect("фильтр есть");
        let filter = &plan.args[index + 1];
        assert_eq!(filter.matches("overlay=").count(), 2, "{filter}");
        assert!(filter.contains("[vout]"));
        assert!(plan.args.iter().any(|arg| arg == "[vout]"));
        assert_eq!(
            plan.output
                .as_ref()
                .and_then(|path| path.extension())
                .and_then(|ext| ext.to_str()),
            Some("mkv")
        );
    }

    #[test]
    fn empty_scene_is_rejected() {
        let scene = Scene::default();
        let destination = Destination::File(std::env::temp_dir().join("empty.mkv"));
        let error = plan_for(&scene, &[], &destination).expect_err("пустая сцена недопустима");
        assert!(error.contains("источников"), "{error}");
    }

    #[test]
    fn audio_without_device_is_rejected() {
        let scene = two_display_scene();
        let audio = vec![AudioSource {
            enabled: true,
            device: String::new(),
            ..Default::default()
        }];
        let destination = Destination::File(std::env::temp_dir().join("audio.mkv"));
        let error = plan_for(&scene, &audio, &destination).expect_err("нужно устройство");
        assert!(error.contains("устройство"), "{error}");
    }

    #[test]
    fn stream_destination_uses_flv() {
        let scene = two_display_scene();
        let destination = Destination::Stream("rtmp://example.invalid/live/key".to_string());
        let plan = plan_for(&scene, &[], &destination).expect("план стрима строится");
        assert!(plan.output.is_none());
        let flv = plan
            .args
            .iter()
            .position(|arg| arg == "flv")
            .expect("flv есть");
        assert_eq!(plan.args[flv - 1], "-f");
        assert_eq!(
            plan.args.last().map(String::as_str),
            Some("rtmp://example.invalid/live/key")
        );
    }

    #[test]
    fn record_path_uses_container_extension() {
        let mut settings = Settings::default();
        settings.output_dir = std::env::temp_dir().to_string_lossy().to_string();
        let path = record_path(&settings, "Моя сцена / тест");
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("mkv"));
        assert!(path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .contains("Моя-сцена-тест"));
    }
}
