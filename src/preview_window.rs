use std::vec;

use egui::{pos2, response, vec2, Align2, LayerId, Pos2, Rect, Sense, Vec2};

use crate::{component_lib::EXAMPLE_UNIT, field::{self, Field}, primitives::Component};

pub struct PreviewWindow {
    drag_vec: Vec2,
}

pub enum DragComponentResponse {
    Dragged {
        pos: Pos2,
        dim: (i32, i32)
    },
    Released {
        pos: Pos2,
        component : Component
    },
    None
}


impl Default for DragComponentResponse {
    fn default() -> Self {
        Self::None
    }
}

impl PreviewWindow {
    pub fn new() -> Self {
        Self {
            drag_vec: vec2(0.0, 0.0),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context,  foreground: LayerId, field_scale:f32) -> DragComponentResponse {
        let mut drag_response = DragComponentResponse::None;
        egui::Window::new("Компоненты").pivot(Align2::CENTER_BOTTOM).movable(true).show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let response = ui.allocate_rect(rect, Sense::all());
            let painter = ui.painter().with_clip_rect(rect);
            let comp = &EXAMPLE_UNIT;
            comp.draw_preview(&rect, &painter);
            let field_grid_size = field_scale*Field::BASE_GRID_SIZE;
            if let Some(hover_pos) = response.hover_pos() {
                if response.dragged() {
                    let mut painter = ui.ctx().layer_painter(foreground);
                    painter.set_opacity(0.25);
                    self.drag_vec += response.drag_delta();
                    let (w, h) = comp.get_dimension();
                    let rect_size = vec2((w + 2) as f32 * field_grid_size, (h + 2) as f32 * field_grid_size);
                    let rect2 = Rect::from_center_size(hover_pos, rect_size);
                    EXAMPLE_UNIT.draw_preview(&rect2, &painter);
                    let ofs_vec = vec2(field_grid_size, field_grid_size);
                    drag_response = DragComponentResponse::Dragged { pos: rect2.min + ofs_vec, dim: (w, h) };
                }
            }
            if response.drag_stopped() {
                let (w, h) = comp.get_dimension();
                let rect_size = vec2((w + 2) as f32 * field_grid_size, (h + 2) as f32 * field_grid_size);
                let pos = response.interact_pointer_pos().unwrap();
                let rect2 = Rect::from_center_size(pos, rect_size);
                let ofs_vec = vec2(field_grid_size, field_grid_size);
                drag_response =  DragComponentResponse::Released { pos: rect2.min + ofs_vec, component: (*comp).clone() };
            }
        });
        return drag_response;
    }
}
