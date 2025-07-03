use egui::{CursorIcon, LayerId, Pos2, Rect, Sense, Vec2, vec2};
use once_cell::sync::Lazy;

use crate::{component_lib::{AND9, AND2, AND3, EXAMPLE_UNIT}, field::Field, grid_db::{Component}};

pub struct PreviewPanel {
    drag_vec: Vec2,
    showcase: Vec<&'static Lazy<Component>>,
    pub is_expanded: bool
}

pub enum DragComponentResponse {
    Dragged { pos: Pos2, dim: (i32, i32) },
    Released { pos: Pos2, component: Component },
    None,
}

impl Default for DragComponentResponse {
    fn default() -> Self {
        Self::None
    }
}

impl PreviewPanel {
    pub fn new() -> Self {
        Self {
            is_expanded: true,
            drag_vec: vec2(0.0, 0.0),
            showcase: vec![
                &EXAMPLE_UNIT,
                &AND2,
                &AND3,
                &AND9
            ]
        }
    }

    pub fn component_preview(&mut self,
        ui: &mut egui::Ui,
        foreground: LayerId,
        field_scale: f32,
        i: usize
    ) -> DragComponentResponse {
        let mut drag_response = DragComponentResponse::None;
        let mut rect = ui.available_rect_before_wrap();
        rect.set_height(ui.available_width()); // TODO: optimize it
        let response = ui.allocate_rect(rect, Sense::all());
        let painter = ui.painter().with_clip_rect(rect);
        let comp = self.showcase[i];
        comp.draw_preview(&rect, &painter);
        let field_grid_size = field_scale * Field::BASE_GRID_SIZE;
        if let Some(hover_pos) = response.hover_pos() {
            if response.dragged() {
                let mut painter = ui.ctx().layer_painter(foreground);
                painter.set_opacity(0.25);
                self.drag_vec += response.drag_delta();
                let (w, h) = comp.get_dimension();
                let rect_size = vec2(
                    (w + 2) as f32 * field_grid_size,
                    (h + 2) as f32 * field_grid_size,
                );
                let rect2 = Rect::from_center_size(hover_pos, rect_size);
                comp.draw_preview(&rect2, &painter);
                if !rect.contains(hover_pos) {
                    let ofs_vec = vec2(field_grid_size, field_grid_size);
                    drag_response = DragComponentResponse::Dragged {
                        pos: rect2.min + ofs_vec,
                        dim: (w, h),
                    };
                }
                ui.ctx()
                    .output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
            } else {
                ui.ctx().output_mut(|o| o.cursor_icon = CursorIcon::Grab);
            }
        }
        if response.drag_stopped() {
            let (w, h) = comp.get_dimension();
            let rect_size = vec2(
                (w + 2) as f32 * field_grid_size,
                (h + 2) as f32 * field_grid_size,
            );
            let pos = response.interact_pointer_pos().unwrap();
            if !rect.contains(pos) {
                let rect2 = Rect::from_center_size(pos, rect_size);
                if ui.ctx().screen_rect().intersects(rect2) {
                    let ofs_vec = vec2(field_grid_size, field_grid_size);
                    drag_response = DragComponentResponse::Released {
                        pos: rect2.min + ofs_vec,
                        component: (*comp).clone(),
                    };
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
                }
            }
        }
        drag_response
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        foreground: LayerId,
        field_scale: f32,
    ) -> DragComponentResponse {
        let mut drag_response = DragComponentResponse::None;
        egui::SidePanel::left("left_panel").resizable(true).show_animated(ctx, self.is_expanded,  |ui| {
            egui::ScrollArea::vertical().max_width(ui.available_width()).show(ui, |ui| {
                for i in 0..self.showcase.len() {
                    let resp = self.component_preview(ui, foreground, field_scale, i);
                    match resp {
                        DragComponentResponse::None => {},
                        _ => {drag_response = resp}
                    }
                }
            });
        });


        /*
        egui::Window::new("Компоненты")
            .pivot(Align2::CENTER_BOTTOM)
            .movable(true)
            .show(ctx, |ui| {

        }); */

        return drag_response;
    }
}
