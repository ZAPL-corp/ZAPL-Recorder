//! Низкочастотный предпросмотр: ffmpeg отдаёт MJPEG-кадры в pipe.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStdout, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::media::ffmpeg;
use crate::media::graph::PreviewPlan;

#[derive(Clone, Debug)]
pub struct PreviewFrame {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

pub struct Preview {
    pub source_name: String,
    error: Arc<Mutex<Option<String>>>,
    child: Child,
    rx: Receiver<PreviewFrame>,
    reader: Option<JoinHandle<()>>,
    err_thread: Option<JoinHandle<()>>,
}

impl Preview {
    pub fn start(plan: &PreviewPlan) -> Result<Self, String> {
        let mut cmd = ffmpeg::command(&plan.ffmpeg);
        cmd.args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Не удалось запустить ffmpeg для предпросмотра: {e}\n{}",
                plan.ffmpeg.display()
            )
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "ffmpeg не отдал поток предпросмотра.".to_string())?;
        let stderr = child.stderr.take();

        let (tx, rx) = sync_channel(2);
        let reader = std::thread::spawn(move || read_frames(stdout, tx));

        let error = Arc::new(Mutex::new(None));
        let err_thread = stderr.map(|stderr| {
            let error = error.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(mut slot) = error.lock() {
                        *slot = Some(line);
                    }
                }
            })
        });

        Ok(Self {
            source_name: plan.source_name.clone(),
            error,
            child,
            rx,
            reader: Some(reader),
            err_thread,
        })
    }

    /// Самый свежий кадр, если он есть.
    pub fn next_frame(&self) -> Option<PreviewFrame> {
        let mut latest = None;
        while let Ok(frame) = self.rx.try_recv() {
            latest = Some(frame);
        }
        latest
    }

    pub fn last_error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|value| value.clone())
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(handle) = self.reader.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.err_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read_frames(mut stdout: ChildStdout, tx: SyncSender<PreviewFrame>) {
    let mut buffer: Vec<u8> = Vec::with_capacity(256 * 1024);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        match stdout.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.len() > 8 * 1024 * 1024 {
                    buffer.clear();
                }
                drain_frames(&mut buffer, &tx);
            }
            Err(_) => break,
        }
    }
}

fn drain_frames(buffer: &mut Vec<u8>, tx: &SyncSender<PreviewFrame>) {
    loop {
        let Some(start) = find(buffer, &[0xFF, 0xD8]) else {
            buffer.clear();
            return;
        };
        if start > 0 {
            buffer.drain(..start);
        }
        let Some(end) = find(buffer, &[0xFF, 0xD9]) else {
            return;
        };
        let jpeg = buffer[..end + 2].to_vec();
        buffer.drain(..end + 2);
        if let Ok(image) = image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg) {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            let frame = PreviewFrame {
                width: width as usize,
                height: height as usize,
                rgba: rgba.into_raw(),
            };
            let _ = tx.try_send(frame);
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}
