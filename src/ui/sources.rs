//! Панель источников сцены и их свойств.

use eframe::egui::{self, RichText};

use crate::app::{Confirm, RecorderApp};
use crate::model::{CanvasRect, Id, SourceKind};
use crate::ui::{theme, TEXT_WEAK};

pub fn panel(app: &mut RecorderApp, ui: &mut egui::Ui) {
    theme::section(ui, "ИСТОЧНИКИ");

    ui.horizontal(|ui| {
        ui.menu_button("+ Источник", |ui| {
            for kind in SourceKind::ALL {
                if ui.button(kind.label()).on_hover_text(kind.hint()).clicked() {
                    app.add_source(kind);
                    ui.close();
                }
            }
        });
        if ui.button("Удалить").clicked() {
            if let Some(id) = app.selected_source {
                app.confirm = Some(Confirm::Source(id));
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Выше").clicked() {
            if let Some(id) = app.selected_source {
                app.move_source(id, -1);
            }
        }
        if ui.button("Ниже").clicked() {
            if let Some(id) = app.selected_source {
                app.move_source(id, 1);
            }
        }
    });

    ui.add_space(4.0);

    let sources: Vec<(Id, String, bool, SourceKind)> = app
        .cfg
        .active_scene()
        .map(|scene| {
            scene
                .sources
                .iter()
                .map(|source| (source.id, source.name.clone(), source.visible, source.kind))
                .collect()
        })
        .unwrap_or_default();

    if sources.is_empty() {
        theme::weak(
            ui,
            "Источников нет. Начните с «Захват экрана» или добавьте окно/браузер.",
        );
    }

    for (id, name, visible, kind) in sources {
        ui.horizontal(|ui| {
            let mut is_visible = visible;
            if ui.checkbox(&mut is_visible, "").changed() {
                app.set_source_visible(id, is_visible);
            }
            let selected = app.selected_source == Some(id);
            if ui.selectable_label(selected, &name).clicked() {
                app.selected_source = Some(id);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                theme::tag(ui, kind.short(), theme::ACCENT);
            });
        });
    }

    ui.add_space(2.0);
    theme::weak(ui, "Первый источник в списке — верхний слой композиции.");

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    let Some(id) = app.selected_source else {
        return;
    };
    let Some(mut draft) = app
        .cfg
        .active_scene()
        .and_then(|scene| scene.sources.iter().find(|source| source.id == id))
        .cloned()
    else {
        return;
    };

    properties(app, ui, &mut draft, id);
}

fn properties(app: &mut RecorderApp, ui: &mut egui::Ui, draft: &mut crate::model::Source, id: Id) {
    let mut changed = false;
    let (canvas_w, canvas_h) = (app.cfg.settings.canvas_w, app.cfg.settings.canvas_h);

    theme::section(ui, "СВОЙСТВА ИСТОЧНИКА");
    ui.horizontal(|ui| {
        ui.add_sized(
            [64.0, 18.0],
            egui::Label::new(RichText::new("Название").small().color(TEXT_WEAK)),
        );
        let mut name = draft.name.clone();
        if ui
            .add_sized(
                [ui.available_width().max(80.0), 22.0],
                egui::TextEdit::singleline(&mut name),
            )
            .changed()
        {
            draft.name = name;
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        theme::tag(ui, draft.kind.short(), theme::ACCENT);
        theme::weak(ui, draft.kind.label());
    });
    ui.add_space(4.0);

    match draft.kind {
        SourceKind::Display => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Монитор").small().color(TEXT_WEAK));
                let mut monitor: u32 = draft.target.trim().parse().unwrap_or(0);
                egui::ComboBox::from_id_salt("source_monitor")
                    .width(150.0)
                    .selected_text(format!("Монитор {monitor}"))
                    .show_ui(ui, |ui| {
                        for index in 0..4u32 {
                            ui.selectable_value(&mut monitor, index, format!("Монитор {index}"));
                        }
                    });
                let as_text = monitor.to_string();
                if as_text != draft.target {
                    draft.target = as_text;
                    changed = true;
                }
            });
            theme::weak(
                ui,
                "На Windows с DXGI источник берёт кадры напрямую с GPU, без нагрузки на CPU.",
            );
        }
        SourceKind::Window => {
            ui.label(RichText::new("Заголовок окна (достаточно части)").small().color(TEXT_WEAK));
            let mut target = draft.target.clone();
            if ui.text_edit_singleline(&mut target).changed() {
                draft.target = target;
                changed = true;
            }
            theme::weak(
                ui,
                "Windows: окно можно перекрывать. Linux: нужен xdotool и видимое окно.",
            );
        }
        SourceKind::Browser => {
            ui.label(RichText::new("Адрес страницы").small().color(TEXT_WEAK));
            let mut target = draft.target.clone();
            if ui.text_edit_singleline(&mut target).changed() {
                draft.target = target;
                changed = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Открыть окно браузера").clicked() {
                    app.open_browser_for(draft);
                }
                theme::weak(ui, "окно встанет в регион ниже");
            });
            theme::weak(
                ui,
                "Захватывается область экрана: держите окно браузера открытым и не перекрывайте.",
            );
        }
        SourceKind::Image => {
            ui.label(RichText::new("Файл PNG/JPG").small().color(TEXT_WEAK));
            let mut target = draft.target.clone();
            if ui.text_edit_singleline(&mut target).changed() {
                draft.target = target;
                changed = true;
            }
        }
    }

    ui.add_space(6.0);
    theme::section(ui, "ПОЛОЖЕНИЕ НА ХОЛСТЕ");
    ui.horizontal(|ui| {
        changed |= drag_i(ui, "X", &mut draft.canvas.x, -4000..=4000);
        changed |= drag_i(ui, "Y", &mut draft.canvas.y, -4000..=4000);
    });
    ui.horizontal(|ui| {
        changed |= drag_u(ui, "Ш", &mut draft.canvas.w, 16..=8192);
        changed |= drag_u(ui, "В", &mut draft.canvas.h, 16..=8192);
    });
    ui.horizontal(|ui| {
        if ui.button("На весь холст").clicked() {
            draft.canvas = CanvasRect::full(canvas_w, canvas_h);
            changed = true;
        }
        if ui.button("По центру").clicked() {
            draft.canvas.x = ((canvas_w.saturating_sub(draft.canvas.w)) / 2) as i32;
            draft.canvas.y = ((canvas_h.saturating_sub(draft.canvas.h)) / 2) as i32;
            changed = true;
        }
    });

    if matches!(draft.kind, SourceKind::Window | SourceKind::Browser) {
        ui.add_space(6.0);
        theme::section(ui, "РЕГИОН НА ЭКРАНЕ");
        ui.horizontal(|ui| {
            changed |= drag_i(ui, "X", &mut draft.screen.x, -4000..=4000);
            changed |= drag_i(ui, "Y", &mut draft.screen.y, -4000..=4000);
        });
        ui.horizontal(|ui| {
            changed |= drag_u(ui, "Ш", &mut draft.screen.w, 64..=3840);
            changed |= drag_u(ui, "В", &mut draft.screen.h, 64..=2160);
        });
        theme::weak(
            ui,
            "Область рабочего стола, из которой берутся пиксели для этого источника.",
        );
    }

    ui.add_space(4.0);
    let mut visible = draft.visible;
    if ui.checkbox(&mut visible, "Источник видим").changed() {
        draft.visible = visible;
        changed = true;
    }

    if changed {
        app.apply_source_draft(id, draft.clone());
    }
}

fn drag_i(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut i32,
    range: std::ops::RangeInclusive<i32>,
) -> bool {
    ui.label(RichText::new(label).small().color(TEXT_WEAK));
    ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(1.0)
            .max_decimals(0),
    )
    .changed()
}

fn drag_u(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) -> bool {
    ui.label(RichText::new(label).small().color(TEXT_WEAK));
    ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(1.0)
            .max_decimals(0),
    )
    .changed()
}
