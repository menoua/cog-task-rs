use eframe::egui;
use eframe::egui::Response;
use egui_extras::{Size, Strip, StripBuilder};

pub fn header_body_controls(ui: &mut egui::Ui, content: impl FnOnce(&mut Strip)) -> Response {
    StripBuilder::new(ui)
        .size(Size::exact(30.0))
        .size(Size::exact(100.0))
        .size(Size::exact(30.0))
        .size(Size::remainder())
        .size(Size::exact(30.0))
        .size(Size::exact(100.0))
        .size(Size::exact(30.0))
        .vertical(|mut strip| {
            strip.empty();
            content(&mut strip);
            strip.empty();
        })
}

pub fn center_x(
    builder: StripBuilder,
    width: f32,
    content: impl FnOnce(&mut egui::Ui),
) -> Response {
    builder
        .size(Size::remainder())
        .size(Size::exact(width))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.empty();
            strip.cell(|ui| content(ui));
            strip.empty();
        })
}
