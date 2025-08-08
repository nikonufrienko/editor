use egui::{
    Color32, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Vec2, pos2, vec2,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    components_panel::DragComponentResponse,
    grid_db::{GridDB, GridPos, LodLevel, grid_pos, grid_rect},
    interaction_manager::{InteractionManager, draw_component_drag_preview},
    locale::Locale,
};

use web_time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GridType {
    Dots,
    Cells,
    None,
}

pub const SUPPORTED_GRID_TYPES: &[GridType] = &[GridType::Cells, GridType::Dots, GridType::None];
impl GridType {
    pub fn get_name(&self, locale: &'static Locale) -> &'static str {
        match self {
            Self::Cells => locale.cells,
            Self::Dots => locale.dots,
            Self::None => locale.empty,
        }
    }
}

pub struct FieldState {
    pub scale: f32,
    pub offset: Vec2,
    pub grid_size: f32,
    pub rect: Rect,
    pub label_font: FontId,
    pub label_visible: bool,
    pub cursor_pos: Option<Pos2>,
    pub debounce: bool,
    pub debounce_scale: f32,
}

// Dummy state parameters used to generate SVG
pub const SVG_DUMMY_STATE: FieldState = FieldState {
    scale: 1.0 / Field::BASE_GRID_SIZE,
    offset: vec2(0.0, 0.0),
    grid_size: 1.0,
    cursor_pos: None,
    label_font: FontId {
        size: 0.8,
        family: egui::FontFamily::Monospace,
    },
    label_visible: true,
    rect: Rect::from_min_max(pos2(0.0, 0.0), pos2(0.0, 0.0)),
    debounce: false,
    debounce_scale: 1.0,
};

impl FieldState {
    pub fn grid_to_screen(&self, grid_pos: &GridPos) -> Pos2 {
        return pos2(
            self.rect.left() + self.offset.x + grid_pos.x as f32 * self.grid_size,
            self.rect.top() + self.offset.y + grid_pos.y as f32 * self.grid_size,
        );
    }

    pub fn screen_to_grid(&self, screen_pos: Pos2) -> GridPos {
        let grid_x = (screen_pos.x - self.rect.left() - self.offset.x) / self.grid_size;
        let grid_y = (screen_pos.y - self.rect.top() - self.offset.y) / self.grid_size;

        GridPos {
            x: grid_x.floor() as i32,
            y: grid_y.floor() as i32,
        }
    }

    pub fn lod_level(&self) -> LodLevel {
        if self.scale <= Field::LOD_LEVEL_MIN_SCALE {
            LodLevel::Min
        } else if self.scale <= Field::LOD_LEVEL_MID_SCALE {
            LodLevel::Mid
        } else {
            LodLevel::Max
        }
    }
}

pub fn filled_cells(
    state: &FieldState,
    grid_pos: &GridPos,
    width: i32,
    height: i32,
    color: Color32,
) -> Shape {
    let rect = Rect::from_min_size(
        state.grid_to_screen(&grid_pos),
        vec2(
            state.grid_size * width as f32,
            state.grid_size * height as f32,
        ),
    );
    Shape::rect_filled(rect, 0.0, color)
}

pub fn blocked_cell(state: &FieldState, pos: &GridPos) -> Vec<Shape> {
    let mut result = vec![];
    let base_p = state.grid_to_screen(&pos);
    let p1 = base_p + vec2(state.grid_size * 0.25, state.grid_size * 0.25);
    let p2 = base_p + vec2(state.grid_size * 0.75, state.grid_size * 0.75);
    let p3 = base_p + vec2(state.grid_size * 0.25, state.grid_size * 0.75);
    let p4 = base_p + vec2(state.grid_size * 0.75, state.grid_size * 0.25);
    result.push(Shape::line_segment(
        [p1, p2],
        Stroke::new(1.0, Color32::RED),
    ));
    result.push(Shape::line_segment(
        [p3, p4],
        Stroke::new(1.0, Color32::RED),
    ));
    result.push(filled_cells(
        &state,
        &pos,
        1,
        1,
        Color32::from_rgba_unmultiplied(255, 0, 0, 25),
    ));
    result
}

pub struct Field {
    pub state: FieldState,
    pub grid_type: GridType,
    pub grid_db: GridDB,
    external_drag_resp: DragComponentResponse,
    pub interaction_manager: InteractionManager,
    debounce_inst: Instant,
}

impl Field {
    // TODO: Move to settings
    pub const BASE_GRID_SIZE: f32 = 10.0;
    pub const MIN_SCALE: f32 = 0.1;
    pub const MAX_SCALE: f32 = 100.0;
    pub const MAX_FONT_SIZE: f32 = 32.0;
    pub const POINT_MIN_SCALE: f32 = 2.0;
    pub const GRID_MIN_SCALE: f32 = 0.6;
    pub const MIN_DISPLAY_TEXT_SIZE: f32 = 3.0;
    pub const LOD_LEVEL_MID_SCALE: f32 = 1.0; // ??
    pub const LOD_LEVEL_MIN_SCALE: f32 = 0.5;
    pub const DEBOUNCE_DURATION: Duration = Duration::from_millis(300);

    pub fn new() -> Self {
        let scale = (Self::MAX_SCALE / 40.0).max(Self::MIN_SCALE);
        let db = GridDB::new();
        Self {
            state: FieldState {
                scale: scale,
                grid_size: Self::BASE_GRID_SIZE * scale,
                offset: Vec2::default(),
                rect: Rect {
                    min: Pos2::default(),
                    max: Pos2::default(),
                },
                label_font: FontId::monospace(
                    (Self::BASE_GRID_SIZE * scale * 0.5).min(Self::MAX_FONT_SIZE),
                ),
                label_visible: Self::BASE_GRID_SIZE * scale * 0.5 >= Self::MIN_DISPLAY_TEXT_SIZE,
                cursor_pos: None,
                debounce: false,
                debounce_scale: scale,
            },
            grid_type: GridType::Cells,
            grid_db: db,
            external_drag_resp: DragComponentResponse::None,
            interaction_manager: InteractionManager::new(),
            debounce_inst: Instant::now(),
        }
    }

    fn display_grid(&self, ui: &mut egui::Ui) {
        let delta_x = if self.state.offset.x >= 0.0 {
            self.state.offset.x % self.state.grid_size
        } else {
            self.state.grid_size - (self.state.offset.x.abs() % self.state.grid_size)
        };
        let delta_y = if self.state.offset.y >= 0.0 {
            self.state.offset.y % self.state.grid_size
        } else {
            self.state.grid_size - (self.state.offset.y.abs() % self.state.grid_size)
        };

        let stroke = Stroke::new(1.0, ui.visuals().strong_text_color().gamma_multiply(0.1));
        let mut shapes = vec![];

        match self.grid_type {
            GridType::Cells => {
                if Self::GRID_MIN_SCALE < self.state.scale {
                    for i in
                        0..((self.state.rect.width() - delta_x) / self.state.grid_size) as i32 + 1
                    {
                        let x = self.state.rect.left() + delta_x + i as f32 * self.state.grid_size;
                        shapes.push(Shape::line_segment(
                            [
                                pos2(x, self.state.rect.top()),
                                pos2(x, self.state.rect.bottom()),
                            ],
                            stroke,
                        ));
                    }

                    for j in
                        0..((self.state.rect.height() - delta_y) / self.state.grid_size) as i32 + 1
                    {
                        let y = self.state.rect.top() + delta_y + j as f32 * self.state.grid_size;
                        shapes.push(Shape::line_segment(
                            [
                                pos2(self.state.rect.left(), y),
                                pos2(self.state.rect.right(), y),
                            ],
                            stroke,
                        ));
                    }
                }
            }
            GridType::Dots => {
                if Self::POINT_MIN_SCALE < self.state.scale {
                    let vertical_lines =
                        ((self.state.rect.width() - delta_x) / self.state.grid_size) as i32 + 1;
                    let horizontal_lines =
                        ((self.state.rect.height() - delta_y) / self.state.grid_size) as i32 + 1;

                    for i in 0..vertical_lines {
                        for j in 0..horizontal_lines {
                            let x =
                                self.state.rect.left() + delta_x + i as f32 * self.state.grid_size;
                            let y =
                                self.state.rect.top() + delta_y + j as f32 * self.state.grid_size;
                            shapes.push(Shape::circle_filled(pos2(x, y), 1.0, stroke.color));
                        }
                    }
                }
            }
            GridType::None => {}
        }

        ui.painter().with_clip_rect(self.state.rect).extend(shapes);

        ui.painter().add(Shape::rect_stroke(
            self.state.rect,
            0.0,
            ui.visuals().window_stroke,
            StrokeKind::Outside,
        ));
    }

    // Update state of field
    fn refresh(
        &mut self,
        ui: &mut egui::Ui,
        response: &Response,
        allocated_rect: Rect,
        locale: &'static Locale,
    ) {
        let delta_vec = allocated_rect.min - self.state.rect.min;
        self.state.offset -= delta_vec;
        self.state.rect = allocated_rect;
        let ongoing_interaction =
            self.interaction_manager
                .refresh(&mut self.grid_db, &self.state, response, ui, locale);
        if response.hovered() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let new_scale = (self.state.scale * zoom_delta).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let zoom_factor = new_scale / self.state.scale;

            if let Some(hover_pos) = response.hover_pos() {
                let local_pos = hover_pos - self.state.rect.min;
                self.state.offset = (self.state.offset - local_pos) * zoom_factor + local_pos;
            }

            if new_scale != self.state.scale {
                if !self.state.debounce {
                    self.state.debounce_scale = self.state.scale;
                }
                self.state.debounce = true;
                self.debounce_inst = Instant::now();
            } else if self.state.debounce && self.debounce_inst.elapsed() > Self::DEBOUNCE_DURATION
            {
                self.state.debounce = false;
            }

            self.state.scale = new_scale;
            if zoom_delta != 1.0 {
                self.state.grid_size = Self::BASE_GRID_SIZE * self.state.scale;
                let label_text_size = self.state.grid_size * 0.5;
                self.state.label_visible = label_text_size > Self::MIN_DISPLAY_TEXT_SIZE;
                self.state.label_font = FontId::monospace(label_text_size);
            }
            if !ongoing_interaction {
                if response.dragged() {
                    self.state.offset += response.drag_delta();
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
                }
            }
        } else if self.state.debounce && self.debounce_inst.elapsed() > Self::DEBOUNCE_DURATION {
            self.state.debounce = false;
        }
        self.state.cursor_pos = response.hover_pos();
    }

    fn handle_drag_resp(&mut self, painter: &Painter, fill_color: Color32) {
        match std::mem::take(&mut self.external_drag_resp) {
            DragComponentResponse::Dragged {
                dim,
                pos,
                only_overlap,
            } => {
                draw_component_drag_preview(
                    &self.grid_db,
                    &self.state,
                    dim,
                    painter,
                    pos,
                    None,
                    fill_color,
                    only_overlap,
                );
            }
            DragComponentResponse::Released { pos, mut component } => {
                component.set_pos(self.state.screen_to_grid(pos));
                let dim = component.get_dimension();
                let p0 = component.get_position();
                for x in 0..dim.0 {
                    for y in 0..dim.1 {
                        if !self
                            .grid_db
                            .is_free_cell(p0 + grid_pos(x, y), component.is_overlap_only())
                        {
                            return;
                        }
                    }
                }
                self.interaction_manager
                    .add_new_component(component, &mut self.grid_db);
            }
            _ => {}
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, locale: &'static Locale) {
        let theme = ui.ctx().theme();
        let allocated_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(self.state.rect, Sense::drag().union(Sense::all()));
        self.refresh(ui, &response, allocated_rect, locale);
        self.display_grid(ui);
        let grid_rect = grid_rect(
            0,
            self.state.screen_to_grid(self.state.rect.min),
            self.state.screen_to_grid(self.state.rect.max),
        );
        let painter: Painter = ui.painter().with_clip_rect(self.state.rect);

        // Display components:
        self.grid_db
            .get_visible_components(&grid_rect)
            .iter()
            .for_each(|u| {
                u.display(&self.state, &painter, theme);
            });

        // Display nets:
        let net_segments = self.grid_db.get_visible_net_segments(&grid_rect);
        painter.extend(net_segments.iter().map(|segment| {
            Shape::Mesh(Arc::new(segment.get_mesh(
                &self.grid_db,
                &self.state,
                theme,
            )))
        }));

        self.handle_drag_resp(
            &ui.painter().with_clip_rect(self.state.rect),
            ui.visuals().strong_text_color().gamma_multiply(0.08),
        );
        self.interaction_manager
            .draw(&mut self.grid_db, &self.state, &painter, ui);
    }

    pub fn set_external_drag_resp(&mut self, resp: DragComponentResponse) {
        self.external_drag_resp = resp;
    }
}
