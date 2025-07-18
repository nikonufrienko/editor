use egui::{CursorIcon, LayerId, Pos2, Rect, RichText, Sense, Vec2, vec2};

use crate::{
    component_lib::{
        ComponentLibEntry, get_component_lib, get_component_lib_with_query, get_group_name,
    },
    field::Field,
    grid_db::Component,
    locale::{EN_LOCALE, Locale},
};

pub struct PreviewPanel {
    drag_vec: Vec2,
    pub is_expanded: bool,
    component_lib: Vec<Vec<ComponentLibEntry>>,
    query: String,
}

pub enum DragComponentResponse {
    Dragged {
        pos: Pos2,
        dim: (i32, i32),
        only_overlap: bool,
    },
    Released {
        pos: Pos2,
        component: Component,
    },
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
            component_lib: get_component_lib(),
            query: String::new(),
        }
    }

    pub fn component_preview(
        &mut self,
        ui: &mut egui::Ui,
        foreground: LayerId,
        field_scale: f32,
        group_id: usize,
        item_id: usize,
    ) -> DragComponentResponse {
        let comp = &self.component_lib[group_id][item_id].component;
        let mut drag_response = DragComponentResponse::None;
        let mut rect = ui.available_rect_before_wrap();
        rect.set_height(ui.available_width()); // TODO: optimize it
        let response = ui.allocate_rect(rect, Sense::all());
        let painter = ui.painter().with_clip_rect(rect);
        let comp = comp;
        comp.draw_preview(&rect, &painter, ui.ctx().theme());
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
                comp.draw_preview(&rect2, &painter, ui.ctx().theme());
                if !rect.contains(hover_pos) {
                    let ofs_vec = vec2(field_grid_size, field_grid_size);
                    drag_response = DragComponentResponse::Dragged {
                        pos: rect2.min + ofs_vec,
                        dim: (w, h),
                        only_overlap: comp.is_overlap_only(),
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
        locale: &'static Locale,
    ) -> DragComponentResponse {
        let mut drag_response = DragComponentResponse::None;
        let mut collapse_all_groups = false;
        let mut expand_all_groups = false;

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .min_width(180.0) // FIXME
            .show_animated(ctx, self.is_expanded, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(locale.components).heading().strong())
                        .selectable(false),
                );
                ui.separator();

                // Filtering:
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(locale.filter).selectable(false));
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.query)
                                .background_color(ui.visuals().faint_bg_color)
                                .desired_width(ui.available_width()),
                        )
                        .changed()
                    {
                        self.component_lib = get_component_lib_with_query(&self.query);
                        collapse_all_groups = self.query == "";
                        expand_all_groups = self.query != "";
                    }
                });
                ui.separator();

                // Previews:
                egui::ScrollArea::vertical()
                    .max_width(ui.available_width())
                    .show(ui, |ui| {
                        for group_id in 0..self.component_lib.len() {
                            if self.component_lib[group_id].is_empty() {
                                continue;
                            }
                            egui::CollapsingHeader::new(get_group_name(group_id, locale))
                                .id_salt(get_group_name(group_id, &EN_LOCALE))
                                .open(if expand_all_groups {
                                    Some(true)
                                } else if collapse_all_groups {
                                    Some(false)
                                } else {
                                    None
                                })
                                .show(ui, |ui| {
                                    for item_id in 0..self.component_lib[group_id].len() {
                                        ui.add(
                                            egui::Label::new(
                                                self.component_lib[group_id][item_id].name,
                                            )
                                            .selectable(false),
                                        );
                                        egui::Frame::default()
                                            .stroke(ui.visuals().window_stroke)
                                            .corner_radius(5.0)
                                            .inner_margin(10.0)
                                            .show(ui, |ui: &mut egui::Ui| {
                                                let resp = self.component_preview(
                                                    ui,
                                                    foreground,
                                                    field_scale,
                                                    group_id,
                                                    item_id,
                                                );
                                                match resp {
                                                    DragComponentResponse::None => {}
                                                    _ => drag_response = resp,
                                                }
                                            });
                                    }
                                });
                        }
                    });
            });
        return drag_response;
    }
}
