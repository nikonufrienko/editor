use std::sync::Arc;

use crate::{
    field::FieldState,
    grid_db::{ComponentAction, ComponentColor, GridPos, Rotation, SvgColor},
};
use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rect, Shape, TextEdit, Theme, Ui, UiBuilder, Vec2,
    epaint::TextShape, pos2, vec2,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct TextField {
    pub text: String,
    pub size: (i32, i32),
    pub pos: GridPos,
}

impl TextField {
    pub const ACTIONS: &'static [ComponentAction] =
        &[ComponentAction::EditText, ComponentAction::Remove];
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
            Some(w as f32 * state.grid_size),
            Rotation::ROT0,
            Align2::LEFT_TOP,
        );
    }

    pub fn get_svg(&self, offset: GridPos, scale: f32, theme: Theme) -> String {
        // TODO: Add text wrapping!!!
        let color = theme.get_text_color().to_svg_hex();
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

pub fn show_text_with_debounce(
    pos: Pos2,
    text: String,
    state: &FieldState,
    painter: &Painter,
    wrap_width: Option<f32>,
    rotation: Rotation,
    anchor: Align2,
) {
    let theme = painter.ctx().theme();
    let color = theme.get_text_color();

    let align_x = anchor.x().to_factor();
    let align_y = anchor.y().to_factor();
    let align_factor = vec2(align_x, align_y);

    let rotated_offset = |size: Vec2, rotation: Rotation| -> Vec2 {
        match rotation {
            Rotation::ROT0 => vec2(align_factor.x * size.x, align_factor.y * size.y),
            Rotation::ROT90 => vec2(-align_factor.y * size.y, align_factor.x * size.x),
            Rotation::ROT180 => vec2(-align_factor.x * size.x, -align_factor.y * size.y),
            Rotation::ROT270 => vec2(align_factor.y * size.y, align_factor.x * size.x),
        }
    };

    if state.debounce {
        let prev_font_size = 64.0;
        let scale = state.grid_size * TextField::FONT_SCALE / prev_font_size;

        let galley = painter.fonts(|fonts| {
            if let Some(wrap) = wrap_width {
                let scaled_wrap = wrap / scale;
                fonts.layout(
                    text.clone(),
                    FontId::monospace(prev_font_size),
                    color,
                    scaled_wrap,
                )
            } else {
                fonts.layout_no_wrap(text.clone(), FontId::monospace(prev_font_size), color)
            }
        });

        let final_size = galley.size() * scale;
        let offset = rotated_offset(final_size, rotation);
        let aligned_pos = pos - offset;

        let mut shape = Shape::Text(
            TextShape::new(pos2(0.0, 0.0), galley, color).with_angle(rotation.to_radians()),
        );
        shape.scale(scale);
        shape.translate(aligned_pos.to_vec2());
        painter.add(shape);
        painter.ctx().request_repaint();
    } else {
        let font_size = state.grid_size * TextField::FONT_SCALE;

        let galley = painter.fonts(|fonts| {
            if let Some(wrap) = wrap_width {
                fonts.layout(text.clone(), FontId::monospace(font_size), color, wrap)
            } else {
                fonts.layout_no_wrap(text.clone(), FontId::monospace(font_size), color)
            }
        });

        let offset = rotated_offset(galley.size(), rotation);
        let aligned_pos = pos - offset;

        let shape = Shape::Text(
            TextShape::new(aligned_pos, galley, color).with_angle(rotation.to_radians()),
        );
        painter.add(shape);
    }
}

pub fn show_text_edit(
    text_edit_rect: Rect,
    single_line: bool,
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
    };
    let ui_builder = UiBuilder::new().max_rect(text_edit_rect).style(style);
    let bg_color = if state.debounce {
        Color32::TRANSPARENT
    } else {
        ui.ctx().theme().get_bg_color()
    };
    let font_size = state.grid_size * TextField::FONT_SCALE;
    ui.scope_builder(ui_builder, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink(true)
            .show(ui, |ui| {
                if single_line {
                    TextEdit::singleline(edit_buffer)
                } else {
                    TextEdit::multiline(edit_buffer)
                }
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
