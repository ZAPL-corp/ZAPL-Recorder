//! Панель сцен.

use eframe::egui::{self, RichText};

use crate::app::{Confirm, RecorderApp};
use crate::model::Id;
use crate::ui::{theme, TEXT_WEAK};

pub fn panel(app: &mut RecorderApp, ui: &mut egui::Ui) {
    theme::section(ui, "СЦЕНЫ");

    let mut renamed = false;
    if let Some(scene) = app.cfg.active_scene_mut() {
        let response = ui.add_sized(
            [ui.available_width(), 24.0],
            egui::TextEdit::singleline(&mut scene.name).hint_text("Название сцены"),
        );
        if response.changed() {
            renamed = true;
        }
    }
    if renamed {
        app.mark_dirty_light();
    }

    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .id_salt("scenes_list")
        .max_height(168.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let scenes: Vec<(Id, String, usize)> = app
                .cfg
                .scenes
                .iter()
                .map(|scene| (scene.id, scene.name.clone(), scene.sources.len()))
                .collect();
            if scenes.is_empty() {
                theme::weak(ui, "Сцен нет. Добавьте первую.");
            }
            for (id, name, count) in scenes {
                let selected = app.cfg.active_scene == id;
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, &name).clicked() {
                        app.select_scene(id);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(count.to_string()).size(10.0).color(TEXT_WEAK))
                            .on_hover_text("Источников в сцене");
                    });
                });
            }
        });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("+ Сцена").clicked() {
            app.add_scene();
        }
        if ui.button("Копия").clicked() {
            app.duplicate_scene();
        }
        if ui.button("Удалить").clicked() {
            let id = app.cfg.active_scene;
            app.confirm = Some(Confirm::Scene(id));
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Выше").clicked() {
            let id = app.cfg.active_scene;
            app.move_scene(id, -1);
        }
        if ui.button("Ниже").clicked() {
            let id = app.cfg.active_scene;
            app.move_scene(id, 1);
        }
    });
    ui.add_space(2.0);
    theme::weak(
        ui,
        "Клик по сцене переключает программу. Первая сцена в списке — верхняя в панели, порядок не влияет на запись.",
    );
}
