# Сторонние компоненты

## FFmpeg

Сборки в `bin/ffmpeg` — отдельные бинарники FFmpeg, которые CI скачивает из
официальных источников:

- Windows: https://www.gyan.dev/ffmpeg/builds/ (essentials, GPL)
- Linux: https://johnvansickle.com/ffmpeg/ (static amd64, GPL)

FFmpeg распространяется по GNU General Public License (GPL) версии 3 или
позднее; точная версия зависит от сборки. Лицензия и исходный код:
https://ffmpeg.org/legal.html

FFmpeg запускается как отдельный процесс и не линкуется с ZAPL Recorder.
Если вы распространяете эту сборку дальше, сохраняйте уведомления FFmpeg и
предоставляйте доступ к его исходникам согласно GPL.

## Rust-зависимости

| Компонент | Лицензия |
|---|---|
| eframe / egui | MIT OR Apache-2.0 |
| glow | MIT OR Apache-2.0 |
| serde / serde_json | MIT OR Apache-2.0 |
| image | MIT OR Apache-2.0 |
| chrono | MIT OR Apache-2.0 |
| dirs | MIT OR Apache-2.0 |

Полные тексты лицензий — в репозиториях соответствующих проектов.
