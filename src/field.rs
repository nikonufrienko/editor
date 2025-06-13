

use std::sync::Arc;
use std::mem;
use eframe::egui_glow::painter;
use egui::{pos2, vec2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Vec2};

use crate::{preview_window::DragComponentResponse, primitives::{grid_pos, grid_rect, Component, ConnectionAlign, GridBD, GridBDConnectionPoint, GridPos, Id, Net, Port, Unit}};

pub enum GridType {
    Points,
    Cells
}


pub struct FieldState {
    pub scale: f32,
    pub offset: Vec2,
    pub grid_size: f32,
    pub rect: Rect,
    pub label_font: FontId,
    pub label_visible: bool,
    pub cursor_pos: Option<Pos2>,
}

impl FieldState {
    pub fn grid_to_screen(&self, grid_pos: &GridPos) -> Pos2 {
        return pos2(self.rect.left() + self.offset.x + grid_pos.x as f32 * self.grid_size, self.rect.top()  + self.offset.y + grid_pos.y as f32 * self.grid_size);
    }

    pub fn screen_to_grid(&self, screen_pos: Pos2) -> GridPos {
        let grid_x = (screen_pos.x - self.rect.left() - self.offset.x) / self.grid_size;
        let grid_y = (screen_pos.y - self.rect.top() - self.offset.y) / self.grid_size;

        GridPos {x: grid_x.floor() as i32, y: grid_y.floor() as i32}
    }
}

pub fn filled_cells(state: &FieldState, grid_pos: &GridPos, width: i32, height: i32, color: Color32) -> Shape {
    let rect = Rect::from_min_size(state.grid_to_screen(&grid_pos), vec2(state.grid_size * width as f32, state.grid_size * height as f32));
    Shape::rect_filled(rect, 0.0, color)
}

pub fn blocked_cell(state: &FieldState, pos: &GridPos) -> Vec<Shape> {
    let mut result = vec![];
    let base_p = state.grid_to_screen(&pos);
    let p1 = base_p + vec2(state.grid_size * 0.25, state.grid_size * 0.25);
    let p2 = base_p + vec2(state.grid_size * 0.75, state.grid_size * 0.75);
    let p3 = base_p + vec2(state.grid_size * 0.25, state.grid_size * 0.75);
    let p4 = base_p + vec2(state.grid_size * 0.75, state.grid_size * 0.25);
    result.push(Shape::line_segment([p1, p2], Stroke::new(1.0, Color32::RED)));
    result.push(Shape::line_segment([p3, p4], Stroke::new(1.0, Color32::RED)));
    result.push(filled_cells(&state,
        &pos, 1, 1, Color32::from_rgba_unmultiplied(255, 0, 0, 25)));
    result
}


pub struct Field {
    pub state: FieldState,
    pub grid_type: GridType,
    pub grid_db: GridBD,
    connection_builder: ConnectionBuilder,
    external_drag_resp: DragComponentResponse
}

impl Field {
    // TODO: Move to settings
    pub const BASE_GRID_SIZE: f32 = 10.0;
    pub const MIN_SCALE: f32 = 0.1;
    pub const MAX_SCALE: f32 = 100.0;
    pub const MAX_FONT_SIZE: f32 = 32.0;
    pub const POINT_MIN_SCALE: f32 = 2.0;
    pub const GRID_MIN_SCALE: f32 = 0.5;
    pub const MIN_DISPLAY_TEXT_SIZE: f32 = 3.0;
    pub const LED_LEVEL0_SCALE: f32 = 0.5;

    pub fn new() -> Self {
        let scale = (Self::MAX_SCALE/10.0).max(Self::MIN_SCALE);
        let mut db = GridBD::new();
        let mut cnt = 0;

        for i in 0..100 {
            for j in 0..100 {
                let unit = Unit {
                    id: 0,
                    name: "АМОГУС".to_owned(),
                    pos: grid_pos(10 * i, 10 * j),
                    width: 5,
                    height: 6,
                    ports: vec![
                        Port{
                            inner_cell:grid_pos(0, 3),
                            align:ConnectionAlign::LEFT,
                            name: "vld".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 4),
                            align:ConnectionAlign::LEFT,
                            name: "data1".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 5),
                            align:ConnectionAlign::LEFT,
                            name: "data2".to_owned(),
                        },

                        Port{
                            inner_cell:grid_pos(4, 1),
                            align:ConnectionAlign::RIGHT,
                            name: "vld".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(4, 2),
                            align:ConnectionAlign::RIGHT,
                            name: "data1".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(4, 3),
                            align:ConnectionAlign::RIGHT,
                            name: "data2".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(2, 0),
                            align:ConnectionAlign::TOP,
                            name: "info".to_owned(),
                        }
                ]};
                db.add_component_with_unknown_id(Component::Unit(unit));
            }
        }

        Self {
            state : FieldState {
                scale: scale,
                grid_size: Self::BASE_GRID_SIZE * scale,
                offset: Vec2::default(),
                rect : Rect { min: Pos2::default(), max: Pos2::default() },
                label_font: FontId::monospace((Self::BASE_GRID_SIZE * scale * 0.5).min(Self::MAX_FONT_SIZE)),
                label_visible: Self::BASE_GRID_SIZE * scale * 0.5 >= Self::MIN_DISPLAY_TEXT_SIZE,
                cursor_pos: None,
            },
            grid_type : GridType::Cells,
            grid_db: db,
            external_drag_resp: DragComponentResponse::None,
            connection_builder: ConnectionBuilder::new()
        }

    }

    fn display_grid(&self, ui: &mut egui::Ui, response: &Response) {
        let delta_x = if self.state.offset.x >= 0.0 {self.state.offset.x % self.state.grid_size} else {self.state.grid_size - (self.state.offset.x.abs()% self.state.grid_size)} ;
        let delta_y = if self.state.offset.y >= 0.0 {self.state.offset.y % self.state.grid_size} else {self.state.grid_size - (self.state.offset.y.abs()% self.state.grid_size)} ;

        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255,255,255,10));
        let mut shapes = vec![];

        match self.grid_type {
            GridType::Cells => {
                if Self::GRID_MIN_SCALE < self.state.scale {
                    for i in 0..((self.state.rect.width() - delta_x) / self.state.grid_size) as i32 + 1 {
                        let x = self.state.rect.left() + delta_x + i as f32 * self.state.grid_size;
                        shapes.push(Shape::line_segment([pos2(x, self.state.rect.top()), pos2(x, self.state.rect.bottom())], stroke));
                    }

                    for j in 0..((self.state.rect.height() - delta_y) / self.state.grid_size) as i32 + 1 {
                        let y = self.state.rect.top() + delta_y + j as f32 * self.state.grid_size;
                        shapes.push(Shape::line_segment([pos2(self.state.rect.left(), y), pos2(self.state.rect.right(), y)], stroke));
                    }
                }
            }
            GridType::Points => {
                if Self::POINT_MIN_SCALE < self.state.scale {
                    let vertical_lines = ((self.state.rect.width() - delta_x) / self.state.grid_size) as i32 + 1;
                    let horizontal_lines = ((self.state.rect.height() - delta_y) / self.state.grid_size) as i32 + 1;

                    for i in 0..vertical_lines {
                        for j in 0..horizontal_lines {
                            let x = self.state.rect.left() + delta_x + i as f32 * self.state.grid_size;
                            let y = self.state.rect.top() + delta_y + j as f32 * self.state.grid_size;
                            shapes.push(Shape::circle_filled(pos2(x, y), 1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
                        }
                    }
                }
            }
        }

        if response.hovered() {
            if let Some(pos) = response.hover_pos() {
                let grid_cell_pos = self.state.screen_to_grid(pos);
                shapes.push(filled_cells(&self.state, &grid_cell_pos, 1, 1, Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
            }
        }

        shapes.push(Shape::rect_stroke(self.state.rect, 0.0, Stroke::new(0.1, Color32::WHITE), StrokeKind::Outside));

        ui.painter().with_clip_rect(self.state.rect).extend(shapes);
    }


    // Update state of field
    fn refresh(&mut self, ui: &mut egui::Ui, response: &Response) {
        if response.dragged() {
            self.state.offset += response.drag_delta();
        }

        if response.hovered() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let new_scale = (self.state.scale * zoom_delta).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let zoom_factor = new_scale / self.state.scale;

            if let Some(hover_pos) = response.hover_pos() {
                self.state.offset = (self.state.offset - hover_pos.to_vec2()) * zoom_factor + hover_pos.to_vec2();
            }

            self.state.scale = new_scale;
            if zoom_delta != 1.0 {
                self.state.grid_size = Self::BASE_GRID_SIZE * self.state.scale;
                let label_text_size = (self.state.grid_size * 0.5).min(Self::MAX_FONT_SIZE);
                self.state.label_visible = label_text_size > Self::MIN_DISPLAY_TEXT_SIZE;
                self.state.label_font = FontId::monospace(label_text_size);
            }
        }
        self.state.cursor_pos = response.hover_pos();
    }

    fn handle_drag_resp(&mut self, painter: &Painter) {
        match std::mem::take(&mut self.external_drag_resp) {
            DragComponentResponse::Dragged{dim, pos}  => {

                let p0 = self.state.screen_to_grid(pos);
                let mut result = vec![];
                for x in 0..dim.0 {
                    for y in 0..dim.1 {
                        let cell = p0 + grid_pos(x, y);
                        if self.grid_db.is_free_cell(cell)  {
                            result.push(filled_cells(&self.state,
                                &cell, 1, 1, Color32::from_rgba_unmultiplied(255, 255, 255, 25)));
                        } else {
                            result.extend(blocked_cell(&self.state, &cell));
                        }
                    }
                }
                painter.extend(result);
            }
            DragComponentResponse::Released { pos, mut component } => {
                component.set_pos(self.state.screen_to_grid(pos));
                let dim = component.get_dimension();
                let p0 = component.get_position();
                for x in 0..dim.0 {
                    for y in 0..dim.1 {
                        if !self.grid_db.is_free_cell(p0 + grid_pos(x, y)) {
                            return;
                        }
                    }
                }
                self.grid_db.add_component_with_unknown_id(component);
            }
            _ => {}
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.state.rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(self.state.rect, Sense::drag().union(Sense::all()));
        self.refresh(ui, &response);
        self.display_grid(ui, &response);
        let grid_rect = grid_rect(0, self.state.screen_to_grid(self.state.rect.min),  self.state.screen_to_grid(self.state.rect.max));
        self.grid_db.get_visible_components(&grid_rect).iter().for_each(|u| { // Return iter
            u.display(&self.state, &ui.painter().with_clip_rect(self.state.rect));
        });
        let painter = ui.painter().with_clip_rect(self.state.rect);
        let net_segments = self.grid_db.get_visible_net_segments(&grid_rect);
        painter.extend(net_segments.iter().map(|segment| {Shape::Mesh(Arc::new(segment.get_mesh(&self.grid_db, &self.state)))}));

        self.connection_builder.update(&mut self.grid_db, &self.state, &response, &painter);
        self.connection_builder.draw(&self.grid_db, &self.state, &ui.painter().with_clip_rect(self.state.rect));
        self.handle_drag_resp(&ui.painter().with_clip_rect(self.state.rect));

        if let Some(seg) = self.grid_db.get_hovered_segment(&self.state) {
            seg.highlight(&self.state, &painter);
        }
    }

    pub fn set_external_drag_resp(&mut self, resp:DragComponentResponse) {
        self.external_drag_resp = resp;
    }

}


enum ConnectionBuilderState {
    IDLE,
    ACTIVE
}

struct ConnectionBuilder {
    state : ConnectionBuilderState,
    point : Option<GridBDConnectionPoint>,
    anchors: Vec<GridPos>
}


fn simplify_path(path: Vec<GridPos>) -> Vec<GridPos> {
    let mut cleaned = Vec::new();
    if path.is_empty() {
        return cleaned;
    }

    cleaned.push(path[0]);
    for &point in &path[1..] {
        if point != *cleaned.last().unwrap() {
            cleaned.push(point);
        }
    }

    if cleaned.len() < 3 {
        return cleaned;
    }

    let mut simplified = Vec::new();
    simplified.push(cleaned[0]);

    for i in 1..cleaned.len() - 1 {
        let prev = cleaned[i - 1];
        let curr = cleaned[i];
        let next = cleaned[i + 1];

        let same_x = prev.x == curr.x && curr.x == next.x;
        let same_y = prev.y == curr.y && curr.y == next.y;

        if !(same_x || same_y) {
            simplified.push(curr);
        }
    }

    simplified.push(*cleaned.last().unwrap());
    simplified
}

impl ConnectionBuilder {
    fn generate_full_path_by_anchors(&self, bd:&GridBD, target: &GridBDConnectionPoint) -> Option<Vec<GridPos>> {
        if let Some(cp) = &self.point {
            if let Some((comp1, p1))= bd.get_component_and_connection(cp) {
                let mut result = vec![p1.get_pos(comp1) +  p1.get_grid_connection_offset()];
                self.anchors.iter().for_each(|a| {
                    result.extend(bd.find_net_path(result.last().unwrap().clone(), a.clone())); // !!!
                    result.push(a.clone());
                });
                let (target_comp, target_con) = bd.get_component_and_connection(target).unwrap();
                let target_pos = target_con.get_pos(target_comp) + target_con.get_grid_connection_offset();
                result.extend(bd.find_net_path(result.last().unwrap().clone(), target_pos.clone())); // !!!
                result.push(target_pos);
                return Some(simplify_path(result));
            }
        }
        None
    }

    fn new() -> Self {
        Self {
            state: ConnectionBuilderState::IDLE,
            point: None,
            anchors: vec![]
        }
    }

    fn update(&mut self, bd: &mut GridBD, state: &FieldState, response: &Response, painter: &egui::Painter) {
        if let Some(con) = bd.get_hovered_connection(&state) {
            let comp =  bd.get_component(&con.component_id).unwrap();
            let connection = comp.get_connection(con.connection_id).unwrap();
            connection.highlight(&state, &comp.get_position(),  painter); // TODO: Move to draw
            if response.clicked() {
                self.toggle(bd, &state, con);
            }
        } else if response.clicked() {
            if let Some(pos) = state.cursor_pos {
                self.add_anchor(state.screen_to_grid(pos));
            }
        }
    }

    fn toggle(&mut self, bd: &mut GridBD, state: &FieldState, point: GridBDConnectionPoint) {
        match self.state {
            ConnectionBuilderState::IDLE => {
                self.point = Some(point);
                self.state = ConnectionBuilderState::ACTIVE;
            }
            ConnectionBuilderState::ACTIVE => {
                let old_point = self.point.clone().unwrap();
                self.state = ConnectionBuilderState::IDLE;
                if let Some(points) = self.generate_full_path_by_anchors(bd, &point) {
                    let mut net = Net {
                        id: bd.nets.len(), // fixme
                        start_point: old_point,
                        end_point: point,
                        points: points
                    };
                    bd.add_net(net);
                }
                self.point = None;
                self.anchors.clear();
            }
        }
    }

    fn add_anchor(&mut self, cell: GridPos) {
        match self.state {
            ConnectionBuilderState::ACTIVE => self.anchors.push(cell),
            ConnectionBuilderState::IDLE => {}
        }
    }

    fn draw(& self, bd: &GridBD, state: &FieldState, painter: &egui::Painter)  {
        if let Some(point) = &self.point {
            //painter.line_segment(points, stroke)
            if let Some(comp) = bd.get_component(&point.component_id) {
                self.anchors.iter().for_each(|a| {
                    let shape = filled_cells(state, a, 1, 1, Color32::RED);
                    painter.add(shape);
                });
                if let Some(con) = comp.get_connection(point.connection_id) {
                    let p1 = con.center(&comp.get_position(), state);
                    let p1_1_grid = con.get_pos(comp) + con.get_grid_connection_offset();
                    let mut points = vec![p1, state.grid_to_screen(&p1_1_grid) + vec2(0.5 * state.grid_size, 0.5 * state.grid_size)];
                    let mut last_grid_p = p1_1_grid;
                    self.anchors.iter().for_each(|a| {
                        let path = bd.find_net_path(last_grid_p.clone(), a.clone());
                        points.extend(path.iter().map(|t|{state.grid_to_screen(t) + vec2(0.5 * state.grid_size, 0.5 * state.grid_size)}));
                        points.push(state.grid_to_screen(a) + vec2(0.5 * state.grid_size, 0.5 * state.grid_size));
                        last_grid_p = a.clone();
                    });
                    if let Some(p2) = state.cursor_pos {
                        points.extend(bd.find_net_path(state.screen_to_grid(points.last().unwrap().clone()), state.screen_to_grid(p2)).iter().map(|g|{state.grid_to_screen(&g) + vec2(state.grid_size*0.5, state.grid_size*0.5)}));
                        points.push(p2);
                    } else {

                    }
                    for i in 1..points.len() {
                        if points[i-1] != points[i] { // fixme
                            painter.circle_filled(points[i], state.grid_size * 0.15, Color32::DARK_GRAY);
                            painter.line_segment([points[i-1], points[i]], Stroke::new(state.grid_size * 0.3, Color32::DARK_GRAY));
                        }
                    }

                }
            }
        }
    }
}


enum ComponentManagerState {
    IDLE,
    ACTIVE,
    SELECTED,
    DRAGGED
}

struct ComponentManager {
    state : ComponentManagerState,
    selected_components_ids : Vec<Id>
}

impl ComponentManager {
    fn new() -> Self {
        Self {
            state: ComponentManagerState::IDLE,
            selected_components_ids: vec![]
        }
    }

    fn refresh() {

    }

    fn draw() {

    }
}
