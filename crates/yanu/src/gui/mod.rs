pub mod app;

use eframe::egui;
use egui::{Context, Id, InnerResponse, Ui};

/// Centers arbitrary widgets vertically and horizontally using `Window`.
pub fn cross_centered<I, R>(
    id: I,
    ctx: &Context,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R>
where
    I: Into<Id>,
{
    let inner_response = egui::Window::new("center")
        .id(id.into())
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .frame(egui::Frame::none())
        .collapsible(false)
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| add_contents(ui))
        .expect("`Window` should always be open");

    InnerResponse::new(
        inner_response
            .inner
            .expect("`Window` should never be collapsed"),
        inner_response.response,
    )
}

pub fn increase_font_size_by(factor: f32, ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    for font_id in style.text_styles.values_mut() {
        font_id.size *= factor;
    }
    ctx.set_style(style);
}
