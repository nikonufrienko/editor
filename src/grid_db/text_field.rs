use crate::{
    field::{FieldState},
    grid_db::{ComponentAction, ComponentColor, GridPos},
};
use egui::{FontId, Painter, Pos2, Rect, Shape, epaint::TextShape, pos2, vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct TextField {
    pub text: String,
    pub size: (i32, i32),
    pub pos: GridPos,
}

impl TextField {
    pub const ACTIONS: &'static [ComponentAction] = &[ComponentAction::Remove];

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        // let theme = painter.ctx().theme();
        let screen_pos = state.grid_to_screen(&self.pos);
        let (w, h) = self.size;
        let rect = Rect::from_min_size(
            screen_pos,
            vec2(state.grid_size * w as f32, state.grid_size * h as f32),
        );

        /*
        draw_dashed_rect(
            painter,
            rect,
            theme.get_stroke_color(),
            STROKE_SCALE * 0.5 * state.grid_size,
            state.grid_size * 0.1,
            state.grid_size * 0.05,
        );
         */
        //painter.text(screen_pos + vec2(state.grid_size * 0.5, state.grid_size * 0.5), egui::Align2::LEFT_TOP, self.text.clone(), FontId::monospace(state.grid_size * 0.5), theme.get_text_color());
        show_text_with_debounce(
            screen_pos,
            self.text.clone(),
            state,
            &painter.with_clip_rect(rect),
            w as f32 * state.grid_size,
        );
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
                width / (state.grid_size * 0.5 / prev_font_size),
            )
        });

        let mut shape = Shape::Text(TextShape::new(
            pos2(0.0, 0.0),
            galley,
            theme.get_text_color(),
        ));
        shape.scale(state.grid_size * 0.5 / prev_font_size);
        shape.translate(pos.to_vec2());
        painter.add(shape);
        painter.ctx().request_repaint(); // To apply debounce
    } else {
        let galley = painter.fonts(|fonts| {
            fonts.layout(
                text,
                FontId::monospace(state.grid_size * 0.5),
                theme.get_text_color(),
                width,
            )
        });

        let shape = Shape::Text(TextShape::new(pos, galley, theme.get_text_color()));
        painter.add(shape);
    }
}
