//! Холст программы: превью, рамки источников и оверлей записи.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind,
};

use crate::app::RecorderApp;
use crate::media::SessionKind;
use crate::model::CanvasRect;
use crate::ui::{format_duration, theme, TEXT_WEAK};

pub fn panel(app: &mut RecorderApp, ui: &mut egui::Ui) {
    let reserve = 68.0;
    let size = egui::vec2(
        ui.available_width().max(200.0),
        (ui.available_height() - reserve).max(150.0),
    );
    let (response, painter) = ui.allocate_painter(size, Sense::click());
    let outer = response.rect;

    painter.rect_filled(outer, CornerRadius::same(8), theme::BG_PANEL);

    let header = Rect::from_min_max(
        outer.min,
        egui::pos2(outer.max.x, outer.min.y + 26.0),
    );
    let scene_name = app
        .cfg
        .active_scene()
        .map(|scene| scene.name.clone())
        .unwrap_or_else(|| "нет сцены".to_string());
    let canvas = (app.cfg.settings.canvas_w, app.cfg.settings.canvas_h);

    painter.text(
        header.min + egui::vec2(12.0, 7.0),
        Align2::LEFT_TOP,
        format!("Программа · {scene_name}"),
        FontId::proportional(12.0),
        theme::TEXT,
    );
    let preview_note = app
        .preview
        .as_ref()
        .map(|preview| format!(" · превью: {}", preview.source_name))
        .unwrap_or_default();
    painter.text(
        header.right_top() + egui::vec2(-12.0, 7.0),
        Align2::RIGHT_TOP,
        format!(
            "{}×{} · {} fps · {}{}",
            canvas.0,
            canvas.1,
            app.cfg.settings.fps,
            app.cfg.settings.encoder.label(),
            preview_note
        ),
        FontId::proportional(11.0),
        TEXT_WEAK,
    );

    let area = Rect::from_min_max(
        egui::pos2(outer.min.x + 12.0, header.max.y + 4.0),
        egui::pos2(outer.max.x - 12.0, outer.max.y - 12.0),
    );
    if area.width() < 8.0 || area.height() < 8.0 {
        return;
    }
    let aspect = canvas.0.max(1) as f32 / canvas.1.max(1) as f32;
    let inner = fit(area, aspect);
    painter.rect_filled(inner, CornerRadius::same(4), Color32::BLACK);

    if let Some(texture) = &app.preview_texture {
        painter.image(
            texture.id(),
            inner,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        let message = if app.probe.is_none() {
            "ffmpeg не найден — превью недоступно".to_string()
        } else if let Some(error) = &app.preview_error {
            shorten(error, 120)
        } else if !app.cfg.settings.preview_enabled {
            "Предпросмотр выключен".to_string()
        } else {
            "Запускаю предпросмотр…".to_string()
        };
        painter.text(
            inner.center(),
            Align2::CENTER_CENTER,
            message,
            FontId::proportional(13.0),
            TEXT_WEAK,
        );
    }

    if let Some(scene) = app.cfg.active_scene() {
        for source in scene.sources.iter().rev() {
            if !source.visible {
                continue;
            }
            let rect = map_rect(inner, source.canvas, canvas);
            if !rect.intersects(inner) {
                continue;
            }
            let selected = app.selected_source == Some(source.id);
            let stroke = if selected {
                Stroke::new(1.6, theme::ACCENT)
            } else {
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 46))
            };
            painter.rect_stroke(rect, CornerRadius::same(3), stroke, StrokeKind::Inside);
            if rect.width() > 80.0 && rect.height() > 26.0 {
                painter.text(
                    rect.left_top() + egui::vec2(6.0, 4.0),
                    Align2::LEFT_TOP,
                    &source.name,
                    FontId::proportional(10.0),
                    if selected { theme::ACCENT } else { TEXT_WEAK },
                );
            }
        }
    }

    if let Some(session) = &app.session {
        let live = session.kind == SessionKind::Streaming;
        let color = if live { theme::LIVE } else { theme::DANGER };
        painter.rect_stroke(outer, CornerRadius::same(8), Stroke::new(2.0, color), StrokeKind::Inside);

        let badge = format!(
            "{}  {}",
            if live { "LIVE" } else { "REC" },
            format_duration(session.elapsed())
        );
        let badge_rect = Rect::from_min_size(
            inner.left_top() + egui::vec2(10.0, 10.0),
            egui::vec2(148.0, 26.0),
        );
        painter.rect_filled(
            badge_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(0, 0, 0, 170),
        );
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            &badge,
            FontId::monospace(13.0),
            color,
        );

        let stats = if live {
            format!(
                "{} · {} · дроп {}",
                session.stats.fps_text(),
                session.stats.bitrate_text(),
                session.stats.dropped
            )
        } else {
            let size = session
                .stats
                .size_kb
                .map(|kb| format!("{:.1} МБ", kb as f32 / 1024.0))
                .unwrap_or_else(|| "—".to_string());
            format!("{} · {} · {}", size, session.stats.fps_text(), session.title)
        };
        let timer_rect = Rect::from_min_size(
            inner.left_bottom() + egui::vec2(10.0, -58.0),
            egui::vec2(240.0, 48.0),
        );
        painter.rect_filled(
            timer_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(0, 0, 0, 185),
        );
        painter.text(
            timer_rect.min + egui::vec2(10.0, 6.0),
            Align2::LEFT_TOP,
            format_duration(session.elapsed()),
            FontId::monospace(21.0),
            color,
        );
        painter.text(
            timer_rect.min + egui::vec2(10.0, 31.0),
            Align2::LEFT_TOP,
            shorten(&stats, 42),
            FontId::proportional(10.0),
            TEXT_WEAK,
        );
    }

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if inner.contains(pos) && inner.width() > 0.0 && inner.height() > 0.0 {
                let cx = (pos.x - inner.min.x) / inner.width() * canvas.0.max(1) as f32;
                let cy = (pos.y - inner.min.y) / inner.height() * canvas.1.max(1) as f32;
                let hit = app.cfg.active_scene().and_then(|scene| {
                    scene
                        .sources
                        .iter()
                        .find(|source| {
                            source.visible
                                && cx >= source.canvas.x as f32
                                && cy >= source.canvas.y as f32
                                && cx <= (source.canvas.x + source.canvas.w as i32) as f32
                                && cy <= (source.canvas.y + source.canvas.h as i32) as f32
                        })
                        .map(|source| source.id)
                });
                if let Some(id) = hit {
                    app.selected_source = Some(id);
                }
            }
        }
    }
}

fn fit(area: Rect, aspect: f32) -> Rect {
    let mut width = area.width().max(1.0);
    let mut height = width / aspect;
    if height > area.height() {
        height = area.height().max(1.0);
        width = height * aspect;
    }
    Rect::from_center_size(area.center(), egui::vec2(width, height))
}

fn map_rect(inner: Rect, rect: CanvasRect, canvas: (u32, u32)) -> Rect {
    let sx = inner.width() / canvas.0.max(1) as f32;
    let sy = inner.height() / canvas.1.max(1) as f32;
    Rect::from_min_size(
        inner.min + egui::vec2(rect.x as f32 * sx, rect.y as f32 * sy),
        egui::vec2(rect.w.max(1) as f32 * sx, rect.h.max(1) as f32 * sy),
    )
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max).collect();
        format!("{cut}…")
    }
}
