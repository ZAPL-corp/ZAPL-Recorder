//! Тёмная тема в духе OBS: плотные панели, синий акцент, красная запись.

use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, RichText, Stroke, TextStyle};

pub const BG: Color32 = Color32::from_rgb(23, 24, 28);
pub const BG_PANEL: Color32 = Color32::from_rgb(30, 31, 36);
pub const BG_DEEP: Color32 = Color32::from_rgb(14, 15, 18);
pub const WIDGET: Color32 = Color32::from_rgb(42, 44, 50);
pub const WIDGET_HOVER: Color32 = Color32::from_rgb(53, 56, 64);
pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(66, 70, 79);
pub const BORDER: Color32 = Color32::from_rgb(48, 50, 57);
pub const TEXT: Color32 = Color32::from_rgb(226, 229, 235);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(150, 156, 167);
pub const ACCENT: Color32 = Color32::from_rgb(58, 124, 246);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(35, 54, 92);
pub const DANGER: Color32 = Color32::from_rgb(229, 72, 77);
pub const LIVE: Color32 = Color32::from_rgb(64, 196, 120);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG_PANEL;
    visuals.faint_bg_color = Color32::from_rgb(35, 36, 42);
    visuals.extreme_bg_color = BG_DEEP;
    visuals.override_text_color = Some(TEXT);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.selection.bg_fill = ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, TEXT);
    visuals.hyperlink_color = ACCENT;

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_WEAK);

    visuals.widgets.inactive.bg_fill = WIDGET;
    visuals.widgets.inactive.weak_bg_fill = WIDGET;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);

    visuals.widgets.hovered.bg_fill = WIDGET_HOVER;
    visuals.widgets.hovered.weak_bg_fill = WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.active.bg_fill = WIDGET_ACTIVE;
    visuals.widgets.active.weak_bg_fill = WIDGET_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);

    visuals.widgets.open.bg_fill = WIDGET;
    visuals.widgets.open.weak_bg_fill = WIDGET;

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size = egui::vec2(40.0, 24.0);
        style.spacing.combo_width = 180.0;
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(19.0)),
            (TextStyle::Body, FontId::proportional(13.5)),
            (TextStyle::Button, FontId::proportional(13.5)),
            (TextStyle::Small, FontId::proportional(11.5)),
            (TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
    });
}

pub fn card() -> egui::Frame {
    egui::Frame::NONE
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
}

pub fn inset() -> egui::Frame {
    egui::Frame::NONE
        .fill(BG_DEEP)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(8))
}

pub fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(title).size(11.0).strong().color(TEXT_WEAK));
    ui.add_space(3.0);
}

pub fn button(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().color(Color32::WHITE))
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(6))
            .min_size(egui::vec2(0.0, 32.0)),
    )
}

pub fn tag(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(10.0).strong().color(color));
}

pub fn weak(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(11.5).color(TEXT_WEAK));
}

pub fn kv(ui: &mut egui::Ui, key: &str, value: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [150.0, 18.0],
            egui::Label::new(RichText::new(key).small().color(TEXT_WEAK)),
        );
        ui.label(RichText::new(value.into()).small());
    });
}
