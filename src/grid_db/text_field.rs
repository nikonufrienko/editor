use std::sync::Arc;

use crate::{
    field::FieldState,
    grid_db::{ComponentAction, ComponentColor, GridPos},
};
use egui::{
    Color32, FontId, Painter, Pos2, Rect, Shape, TextEdit, Theme, Ui, UiBuilder, epaint::TextShape,
    pos2, vec2,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct TextField {
    pub text: String,
    pub size: (i32, i32),
    pub pos: GridPos,
}

impl TextField {
    pub const ACTIONS: &'static [ComponentAction] = &[ComponentAction::Remove];
    pub const FONT_SCALE: f32 = 0.5;
    pub fn display(&self, state: &FieldState, painter: &Painter) {
        let screen_pos = state.grid_to_screen(&self.pos);
        let (w, h) = self.size;
        let rect = Rect::from_min_size(
            screen_pos,
            vec2(state.grid_size * w as f32, state.grid_size * h as f32),
        );
        show_text_with_debounce(
            screen_pos,
            self.text.clone(),
            state,
            &painter.with_clip_rect(rect),
            w as f32 * state.grid_size,
        );
    }

    pub fn get_svg(&self, offset: GridPos, scale: f32, theme: Theme) -> String {
        // TODO: Add text wrapping!!!
        let color = theme.get_text_color().to_hex();
        let GridPos { x, y } = self.pos + offset;
        let x = x as f32 * scale;
        let y = y as f32 * scale;
        let font_size = Self::FONT_SCALE * scale;
        let body = self
            .text
            .split("\n")
            .enumerate()
            .map(|(i, line)| {
                let dy = if i == 0 { 0.0 } else { font_size };
                format!(r#"<tspan x="{x}" dy="{dy}">{line}</tspan>"#)
            })
            .collect::<Vec<String>>()
            .join("");
        format!(
            r#"<text x="{x}" y="{y}" font-family="monospace" font-size="{font_size}" fill="{color}" text-anchor="start" dominant-baseline="hanging">{body}</text>"#
        )
    }
}

fn show_text_with_debounce(
    pos: Pos2,
    text: String,
    state: &FieldState,
    painter: &Painter,
    width: f32,
) {
    let theme = painter.ctx().theme();
    if state.debounce {
        let prev_font_size = 64.0;
        let galley = painter.fonts(|fonts| {
            fonts.layout(
                text,
                FontId::monospace(prev_font_size),
                theme.get_text_color(),
                width / (state.grid_size * TextField::FONT_SCALE / prev_font_size),
            )
        });

        let mut shape = Shape::Text(TextShape::new(
            pos2(0.0, 0.0),
            galley,
            theme.get_text_color(),
        ));
        shape.scale(state.grid_size * TextField::FONT_SCALE / prev_font_size);
        shape.translate(pos.to_vec2());
        painter.add(shape);
        painter.ctx().request_repaint(); // To apply debounce
    } else {
        let galley = painter.fonts(|fonts| {
            fonts.layout(
                text,
                FontId::monospace(state.grid_size * TextField::FONT_SCALE),
                theme.get_text_color(),
                width,
            )
        });

        let shape = Shape::Text(TextShape::new(pos, galley, theme.get_text_color()));
        painter.add(shape);
    }
}

pub fn show_text_edit(
    text_edit_rect: Rect,
    edit_buffer: &mut String,
    state: &FieldState,
    ui: &mut Ui,
) {
    let style = if state.debounce {
        let mut style = (*ui.ctx().style()).clone();
        style.visuals.selection.bg_fill = Color32::TRANSPARENT;
        style.visuals.selection.stroke.color = Color32::TRANSPARENT;
        style.visuals.text_cursor.blink = false;
        style.visuals.text_cursor.stroke.color = Color32::TRANSPARENT;
        Arc::new(style)
    } else {
        ui.ctx().style().clone()
        /*
        let mut style = (*ui.ctx().style()).clone();
        style.visuals.selection.stroke.color = Color32::TRANSPARENT;
        style.visuals.selection.stroke.color = Color32::TRANSPARENT;
        style.visuals.widgets.hovered.fg_stroke.color = Color32::TRANSPARENT;
        style.visuals.widgets.hovered.bg_stroke.color = Color32::TRANSPARENT;
        style.visuals.widgets.active.fg_stroke.color = Color32::TRANSPARENT;
        style.visuals.widgets.active.bg_stroke.color = Color32::TRANSPARENT;
        Arc::new(style)
        */
    };
    let ui_builder = UiBuilder::new().max_rect(text_edit_rect).style(style);
    let bg_color = if state.debounce {
        Color32::TRANSPARENT
    } else {
        ui.ctx().theme().get_bg_color()
    };
    let font_size = state.grid_size * TextField::FONT_SCALE;
    //ui.painter().rect_filled(text_edit_rect, 0.0, bg_color);
    ui.scope_builder(ui_builder, |ui| {
        egui::ScrollArea::vertical()
            .max_height(text_edit_rect.height())
            .show(ui, |ui| {
                TextEdit::multiline(edit_buffer)
                    .background_color(bg_color)
                    .desired_width(text_edit_rect.width())
                    .desired_rows(1)
                    .text_color(if state.debounce {
                        Color32::TRANSPARENT
                    } else {
                        ui.ctx().theme().get_text_color()
                    })
                    .font(egui::FontId::monospace(font_size))
                    .show(ui);
            });
        if state.debounce {
            ui.ctx().request_repaint();
        }
    });
}
