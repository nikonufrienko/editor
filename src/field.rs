use egui::{
    Color32, CursorIcon, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, pos2, vec2,
};
use std::sync::Arc;

use crate::{
    grid_db::{
        Component, ComponentAction, GridBD, GridBDConnectionPoint, GridPos, Id, Net,
        RotationDirection, grid_pos, grid_rect,
    },
    preview::DragComponentResponse,
};

#[derive(PartialEq)]
pub enum GridType {
    Dots,
    Cells,
    None,
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
    pub grid_db: GridBD,
    connection_builder: ConnectionBuilder,
    external_drag_resp: DragComponentResponse,
    drag_manager: DragManager,
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
    pub const LOD_LEVEL0_SCALE: f32 = 0.5;

    pub fn new() -> Self {
        let scale = (Self::MAX_SCALE / 40.0).max(Self::MIN_SCALE);
        let db = GridBD::new();

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
            },
            grid_type: GridType::Cells,
            grid_db: db,
            external_drag_resp: DragComponentResponse::None,
            connection_builder: ConnectionBuilder::new(),
            drag_manager: DragManager::new(),
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

        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 10));
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
                            shapes.push(Shape::circle_filled(
                                pos2(x, y),
                                1.0,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 50),
                            ));
                        }
                    }
                }
            }
            GridType::None => {}
        }

        shapes.push(Shape::rect_stroke(
            self.state.rect,
            0.0,
            Stroke::new(0.1, Color32::WHITE),
            StrokeKind::Outside,
        ));

        ui.painter().with_clip_rect(self.state.rect).extend(shapes);
    }

    // Update state of field
    fn refresh(&mut self, ui: &mut egui::Ui, response: &Response, allocated_rect: Rect) {
        let delta_vec = allocated_rect.min - self.state.rect.min;
        self.state.offset -= delta_vec;
        self.state.rect = allocated_rect;
        if response.hovered() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let new_scale = (self.state.scale * zoom_delta).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let zoom_factor = new_scale / self.state.scale;

            if let Some(hover_pos) = response.hover_pos() {
                let local_pos = hover_pos - self.state.rect.min;
                self.state.offset = (self.state.offset - local_pos) * zoom_factor + local_pos;
            }

            self.state.scale = new_scale;
            if zoom_delta != 1.0 {
                self.state.grid_size = Self::BASE_GRID_SIZE * self.state.scale;
                let label_text_size = (self.state.grid_size * 0.5).min(Self::MAX_FONT_SIZE);
                self.state.label_visible = label_text_size > Self::MIN_DISPLAY_TEXT_SIZE;
                self.state.label_font = FontId::monospace(label_text_size);
            }
            if !self
                .drag_manager
                .refresh(&mut self.grid_db, &self.state, response, ui)
            {
                if response.dragged() {
                    self.state.offset += response.drag_delta();
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
                }
            }
        }
        self.state.cursor_pos = response.hover_pos();
    }

    fn handle_drag_resp(&mut self, painter: &Painter) {
        match std::mem::take(&mut self.external_drag_resp) {
            DragComponentResponse::Dragged { dim, pos } => {
                draw_component_drag_preview(&self.grid_db, &self.state, dim, painter, pos, None);
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
                self.grid_db.push_component(component);
            }
            _ => {}
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let allocated_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(self.state.rect, Sense::drag().union(Sense::all()));
        self.refresh(ui, &response, allocated_rect);
        self.display_grid(ui);
        let grid_rect = grid_rect(
            0,
            self.state.screen_to_grid(self.state.rect.min),
            self.state.screen_to_grid(self.state.rect.max),
        );
        self.grid_db
            .get_visible_components(&grid_rect)
            .iter()
            .for_each(|u| {
                u.display(&self.state, &ui.painter().with_clip_rect(self.state.rect));
            });
        let painter = ui.painter().with_clip_rect(self.state.rect);
        let net_segments = self.grid_db.get_visible_net_segments(&grid_rect);
        painter.extend(
            net_segments
                .iter()
                .map(|segment| Shape::Mesh(Arc::new(segment.get_mesh(&self.grid_db, &self.state)))),
        );

        self.connection_builder
            .update(&mut self.grid_db, &self.state, &response, &painter);
        self.connection_builder.draw(
            &self.grid_db,
            &self.state,
            &ui.painter().with_clip_rect(self.state.rect),
        );
        self.handle_drag_resp(&ui.painter().with_clip_rect(self.state.rect));
        self.drag_manager
            .draw_preview(&self.grid_db, &self.state, &painter, ui);
    }

    pub fn set_external_drag_resp(&mut self, resp: DragComponentResponse) {
        self.external_drag_resp = resp;
    }
}

enum ConnectionBuilderState {
    IDLE,
    ACTIVE,
}

struct ConnectionBuilder {
    state: ConnectionBuilderState,
    point: Option<GridBDConnectionPoint>,
    anchors: Vec<GridPos>,
}

fn simplify_path(path: Vec<GridPos>) -> Vec<GridPos> {
    let mut cleaned = Vec::with_capacity(path.len());
    cleaned.push(path[0]);
    for (i, &point) in path.iter().enumerate().skip(1) {
        if point != *cleaned.last().unwrap() || i == path.len() - 1 {
            cleaned.push(point);
        }
    }

    if cleaned.len() <= 2 {
        return cleaned;
    }

    let mut i = 1;
    while i < cleaned.len().saturating_sub(1) {
        let prev = cleaned[i - 1];
        let curr = cleaned[i];
        let next = cleaned[i + 1];

        let same_x = prev.x == curr.x && curr.x == next.x;
        let same_y = prev.y == curr.y && curr.y == next.y;

        if same_x || same_y {
            cleaned.remove(i);
        } else {
            i += 1;
        }
    }

    cleaned
}

impl ConnectionBuilder {
    fn generate_full_path_by_anchors(
        &self,
        bd: &GridBD,
        target: &GridBDConnectionPoint,
    ) -> Option<Vec<GridPos>> {
        let cp = &self.point?;
        let comp1 = bd.get_component(&cp.component_id)?;
        let mut result = vec![comp1.get_connection_dock_cell(cp.connection_id).unwrap()];
        self.anchors.iter().for_each(|a| {
            result.extend(bd.find_net_path(result.last().unwrap().clone(), a.clone())); // !!!
            result.push(a.clone());
        });
        let target_comp = bd.get_component(&target.component_id).unwrap();
        let target_pos = target_comp
            .get_connection_dock_cell(target.connection_id)
            .unwrap();
        result.extend(bd.find_net_path(result.last().unwrap().clone(), target_pos.clone())); // !!!
        result.push(target_pos);
        return Some(simplify_path(result));
    }

    fn new() -> Self {
        Self {
            state: ConnectionBuilderState::IDLE,
            point: None,
            anchors: vec![],
        }
    }

    fn update(
        &mut self,
        bd: &mut GridBD,
        state: &FieldState,
        response: &Response,
        painter: &egui::Painter,
    ) {
        if let Some(con) = bd.get_hovered_connection(&state) {
            let comp = bd.get_component(&con.component_id).unwrap();
            comp.highlight_connection(con.connection_id, state, painter);
            if response.clicked() {
                self.toggle(bd, con);
            }
        } else if response.clicked() {
            if let Some(pos) = state.cursor_pos {
                self.add_anchor(state.screen_to_grid(pos));
            }
        }
    }

    fn toggle(&mut self, bd: &mut GridBD, point: GridBDConnectionPoint) {
        match self.state {
            ConnectionBuilderState::IDLE => {
                self.point = Some(point);
                self.state = ConnectionBuilderState::ACTIVE;
            }
            ConnectionBuilderState::ACTIVE => {
                let old_point = self.point.clone().unwrap();
                self.state = ConnectionBuilderState::IDLE;
                if let Some(points) = self.generate_full_path_by_anchors(bd, &point) {
                    bd.add_net(Net {
                        start_point: old_point,
                        end_point: point,
                        points: points,
                    });
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

    fn draw_anchors(&self, state: &FieldState, painter: &egui::Painter) {
        self.anchors.iter().for_each(|a| {
            let r1 = Rect::from_min_size(
                state.grid_to_screen(a),
                vec2(state.grid_size, state.grid_size),
            )
            .scale_from_center(0.8);

            let r2 = r1.scale_from_center(0.5);
            let stroke = Stroke::new(state.grid_size * 0.1, Color32::GRAY);
            painter.line_segment([r1.left_top(), r2.left_top()], stroke);
            painter.line_segment([r1.left_bottom(), r2.left_bottom()], stroke);
            painter.line_segment([r1.right_top(), r2.right_top()], stroke);
            painter.line_segment([r1.right_bottom(), r2.right_bottom()], stroke);
        });
    }

    fn draw(&self, bd: &GridBD, state: &FieldState, painter: &egui::Painter) {
        if let Some(point) = &self.point {
            if let Some(comp) = bd.get_component(&point.component_id) {
                self.draw_anchors(state, painter);
                let p1 = comp
                    .get_connection_position(point.connection_id, state)
                    .unwrap();
                let p1_1_grid = comp.get_connection_dock_cell(point.connection_id).unwrap();
                let mut points = vec![
                    p1,
                    state.grid_to_screen(&p1_1_grid)
                        + vec2(0.5 * state.grid_size, 0.5 * state.grid_size),
                ];
                let mut last_grid_p = p1_1_grid;
                self.anchors.iter().for_each(|a| {
                    let path = bd.find_net_path(last_grid_p.clone(), a.clone());
                    points.extend(path.iter().map(|t| {
                        state.grid_to_screen(t) + vec2(0.5 * state.grid_size, 0.5 * state.grid_size)
                    }));
                    points.push(
                        state.grid_to_screen(a)
                            + vec2(0.5 * state.grid_size, 0.5 * state.grid_size),
                    );
                    last_grid_p = a.clone();
                });
                if let Some(p2) = state.cursor_pos {
                    points.extend(
                        bd.find_net_path(
                            state.screen_to_grid(points.last().unwrap().clone()),
                            state.screen_to_grid(p2),
                        )
                        .iter()
                        .map(|g| {
                            state.grid_to_screen(&g)
                                + vec2(state.grid_size * 0.5, state.grid_size * 0.5)
                        }),
                    );
                    points.push(p2);
                } else {
                }
                for i in 1..points.len() {
                    if points[i - 1] != points[i] {
                        // fixme
                        painter.circle_filled(
                            points[i],
                            state.grid_size * 0.15,
                            Color32::DARK_GRAY,
                        );
                        painter.line_segment(
                            [points[i - 1], points[i]],
                            Stroke::new(state.grid_size * 0.3, Color32::DARK_GRAY),
                        );
                    }
                }
            }
        }
    }
}

// INTERACTION MANAGER???
#[derive(PartialEq)]
enum DragState {
    Idle,
    NetDragged { net_id: Id, segment_id: Id },
    ComponentSelected(Id),
    ComponentDragged { id: Id, grab_ofs: Vec2 },
}

struct DragManager {
    state: DragState,
    drag_delta: Vec2,
}

fn draw_component_drag_preview(
    bd: &GridBD,
    state: &FieldState,
    dim: (i32, i32),
    painter: &Painter,
    pos: Pos2,
    component_id: Option<Id>,
) {
    let p0 = state.screen_to_grid(pos);
    let mut result = vec![];
    for x in 0..dim.0 {
        for y in 0..dim.1 {
            let cell = p0 + grid_pos(x, y);
            let available = if let Some(id) = component_id {
                bd.is_available_cell(cell, id)
            } else {
                bd.is_free_cell(cell)
            };
            if available {
                result.push(filled_cells(
                    state,
                    &cell,
                    1,
                    1,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 25),
                ));
            } else {
                result.extend(blocked_cell(state, &cell));
            }
        }
    }
    painter.extend(result);
}

impl DragManager {
    fn new() -> Self {
        Self {
            state: DragState::Idle,
            drag_delta: vec2(0.0, 0.0),
        }
    }

    fn move_net_segment(&self, cursor_grid_pos: &GridPos, bd: &mut GridBD) {
        match self.state {
            DragState::NetDragged { net_id, segment_id } => {
                let GridPos { x, y } = cursor_grid_pos;
                let mut net = bd.remove_net(&net_id).unwrap();
                let p1 = net.points[segment_id];
                let p2 = net.points[segment_id + 1];
                if p1.y == p2.y {
                    net.points[segment_id] = grid_pos(p1.x, *y);
                    net.points[segment_id + 1] = grid_pos(p2.x, *y);
                } else {
                    net.points[segment_id] = grid_pos(*x, p1.y);
                    net.points[segment_id + 1] = grid_pos(*x, p2.y);
                }
                if net.points.len() > 0 && (segment_id == net.points.len() - 2) {
                    net.points.push(p2);
                }
                if segment_id == 0 {
                    net.points.insert(0, p1);
                }
                net.points = simplify_path(net.points);
                bd.add_net(net);
            }
            _ => {}
        }
    }

    fn move_net_connection_point(
        comp_id: Id,
        net_id: Id,
        bd: &mut GridBD,
        delta_x: i32,
        delta_y: i32,
    ) {
        let mut net = bd.remove_net(&net_id).unwrap();
        let pts_len = net.points.len();

        if pts_len >= 2 {
            if comp_id == net.start_point.component_id && comp_id == net.end_point.component_id {
                // Move all points if component is connected to both ends
                for i in 0..net.points.len() {
                    net.points[i] = net.points[i] + grid_pos(delta_x, delta_y);
                }
            } else if comp_id == net.start_point.component_id {
                // Handle component connected to start of net
                if net.points[0].y == net.points[1].y {
                    // horizontal segment
                    if net.points.len() >= 4 {
                        // Has another vertical segment that can be moved
                        net.points[0] += grid_pos(delta_x, delta_y);
                        net.points[1] += grid_pos(delta_x, delta_y);
                        net.points[2] += grid_pos(delta_x, 0);
                    } else {
                        net.points[0].x += delta_x;
                        if delta_y != 0 {
                            net.points.insert(0, net.points[0] + grid_pos(0, delta_y));
                        }
                    }
                } else {
                    // vertical segment
                    if net.points.len() >= 4 {
                        // Has another horizontal segment that can be moved
                        net.points[0] += grid_pos(delta_x, delta_y);
                        net.points[1] += grid_pos(delta_x, delta_y);
                        net.points[2] += grid_pos(0, delta_y);
                    } else {
                        net.points[0].y += delta_y; // Fixed: change Y instead of X
                        if delta_x != 0 {
                            net.points.insert(0, net.points[0] + grid_pos(delta_x, 0));
                        }
                    }
                }
            } else if comp_id == net.end_point.component_id {
                // Handle component connected to end of net
                if net.points[pts_len - 1].y == net.points[pts_len - 2].y {
                    // horizontal segment
                    if net.points.len() >= 4 {
                        net.points[pts_len - 1] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 2] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 3] += grid_pos(delta_x, 0);
                    } else {
                        net.points[pts_len - 1].x += delta_x;
                        if delta_y != 0 {
                            net.points
                                .push(net.points[pts_len - 1] + grid_pos(0, delta_y));
                        }
                    }
                } else {
                    // vertical segment
                    if net.points.len() >= 4 {
                        net.points[pts_len - 1] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 2] += grid_pos(delta_x, delta_y);
                        net.points[pts_len - 3] += grid_pos(0, delta_y);
                    } else {
                        net.points[pts_len - 1].y += delta_y;
                        if delta_x != 0 {
                            net.points
                                .push(net.points[pts_len - 1] + grid_pos(delta_x, 0));
                        }
                    }
                }
            }
        }

        net.points = simplify_path(net.points);
        bd.add_net(net);
    }

    fn move_component(&self, comp_id: Id, bd: &mut GridBD, new_pos: GridPos) {
        let comp = bd.get_component(&comp_id).unwrap();

        if bd.is_available_location(new_pos, comp.get_dimension(), comp_id) {
            let old_pos = comp.get_position();
            let delta_y = new_pos.y - old_pos.y;
            let delta_x = new_pos.x - old_pos.x;

            for net_id in bd.get_connected_nets(&comp_id) {
                Self::move_net_connection_point(comp_id, net_id, bd, delta_x, delta_y);
            }

            let mut comp = bd.remove_component(&comp_id).unwrap();
            comp.set_pos(new_pos);
            bd.insert_component(comp_id, comp);
        }
    }

    fn rotate_component(&self, comp_id: Id, bd: &mut GridBD, dir: RotationDirection) {
        let comp = bd.get_component(&comp_id).unwrap().clone();
        let mut rotated_comp = comp.clone();
        rotated_comp.rotate(dir);

        if bd.is_available_location(
            rotated_comp.get_position(),
            rotated_comp.get_dimension(),
            comp_id,
        ) {
            let nets_ids: Vec<Id> = bd
                .get_connected_nets(&comp_id)
                .iter()
                .map(|it| *it)
                .collect();
            let connections_ids: Vec<Id> = nets_ids
                .iter()
                .map(|it| {
                    let net = bd.nets.get(it).unwrap();
                    if net.end_point.component_id == comp_id {
                        net.end_point.connection_id
                    } else {
                        net.start_point.connection_id
                    }
                })
                .collect();

            for (i, net_id) in nets_ids.iter().enumerate() {
                let old_pos = comp.get_connection_dock_cell(connections_ids[i]).unwrap();
                let new_pos = rotated_comp
                    .get_connection_dock_cell(connections_ids[i])
                    .unwrap();
                let delta_y = new_pos.y - old_pos.y;
                let delta_x = new_pos.x - old_pos.x;
                Self::move_net_connection_point(comp_id, *net_id, bd, delta_x, delta_y);
            }

            bd.remove_component(&comp_id).unwrap();
            bd.insert_component(comp_id, rotated_comp);
        }
    }

    /// Returns false if no drag action performed
    fn refresh(
        &mut self,
        bd: &mut GridBD,
        state: &FieldState,
        response: &Response,
        ui: &egui::Ui,
    ) -> bool {
        match self.state {
            DragState::NetDragged { net_id, segment_id } => {
                if let Some(hover_pos) = state.cursor_pos {
                    let segment = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id, net_id)
                        .unwrap();
                    if segment.is_horizontal() {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    } else {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    }
                    if response.is_pointer_button_down_on() {
                        self.drag_delta += response.drag_delta();
                        return true;
                    } else {
                        self.drag_delta = vec2(0.0, 0.0);
                        self.move_net_segment(&state.screen_to_grid(hover_pos), bd);
                        self.state = DragState::Idle
                    }
                }
            }
            DragState::Idle => {
                if let Some(segment) = bd.get_hovered_segment(state) {
                    if segment.is_horizontal() {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                    } else {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    }
                    if response.is_pointer_button_down_on() {
                        // Do no use dragged() or drag_started()
                        self.drag_delta += response.drag_delta();
                        self.state = DragState::NetDragged {
                            net_id: segment.net_id,
                            segment_id: segment.inner_id,
                        };
                        return true;
                    }
                } else if let Some(id) = bd.get_hovered_component_id(state) {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                    if response.clicked() {
                        self.state = DragState::ComponentSelected(*id);
                        return true;
                    }
                }
            }
            DragState::ComponentSelected(id) => {
                let comp = bd.get_component(&id).unwrap();
                let action = Self::get_action(comp, state);
                if response.clicked() && action != ComponentAction::None {
                    match action {
                        ComponentAction::RotateUp => {
                            self.rotate_component(id, bd, RotationDirection::Up);
                            self.state = DragState::Idle;
                        }
                        ComponentAction::RotateDown => {
                            self.rotate_component(id, bd, RotationDirection::Down);
                            self.state = DragState::Idle;
                        }
                        ComponentAction::Remove => {
                            bd.remove_component_with_connected_nets(&id);
                            self.state = DragState::Idle;
                            return true;
                        }
                        _ => {}
                    }
                    return true;
                } else if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                    bd.remove_component_with_connected_nets(&id);
                    self.state = DragState::Idle;
                    return true;
                } else if comp.is_hovered(state) {
                    ui.ctx().output_mut(|o| o.cursor_icon = CursorIcon::Grab);
                    if response.dragged() {
                        if let Some(hovepos) = response.hover_pos() {
                            self.state = DragState::ComponentDragged {
                                id,
                                grab_ofs: hovepos.to_vec2()
                                    - state.grid_to_screen(&comp.get_position()).to_vec2(),
                            };
                        }
                    }
                    return true;
                } else if response.clicked() {
                    self.state = DragState::Idle;
                }
            } // TODO
            DragState::ComponentDragged { id, grab_ofs } => {
                if response.dragged() {
                    ui.ctx()
                        .output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
                } else {
                    if let Some(pos) = state.cursor_pos {
                        self.move_component(id, bd, state.screen_to_grid(pos - grab_ofs));
                    }
                    self.state = DragState::Idle;
                }
                return true;
            }
        }
        false
    }

    fn draw_preview(&mut self, bd: &GridBD, state: &FieldState, painter: &Painter, ui: &Ui) {
        match self.state {
            DragState::NetDragged { net_id, segment_id } => {
                let ofs = vec2(0.5, 0.5) * state.grid_size;
                if let Some(pos) = state.cursor_pos {
                    let GridPos { x, y } = state.screen_to_grid(pos);
                    let segment = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id, net_id)
                        .unwrap();
                    let (p1, p2) = if segment.is_horizontal() {
                        (
                            state.grid_to_screen(&grid_pos(segment.pos1.x, y)),
                            state.grid_to_screen(&grid_pos(segment.pos2.x, y)),
                        )
                    } else {
                        (
                            state.grid_to_screen(&grid_pos(x, segment.pos1.y)),
                            state.grid_to_screen(&grid_pos(x, segment.pos2.y)),
                        )
                    };
                    let mut pts = vec![p1 + ofs, p2 + ofs];
                    if let Some(next_segment) = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id + 1, net_id)
                    {
                        pts.push(state.grid_to_screen(&next_segment.pos2) + ofs);
                    } else {
                        pts.push(state.grid_to_screen(&segment.pos2) + ofs);
                    }
                    if let Some(prev_segment) = bd
                        .nets
                        .get(&net_id)
                        .unwrap()
                        .get_segment(segment_id.wrapping_sub(1), net_id)
                    {
                        pts.insert(0, state.grid_to_screen(&prev_segment.pos1) + ofs);
                    } else {
                        pts.insert(0, state.grid_to_screen(&segment.pos1) + ofs);
                    }
                    painter.line(
                        pts,
                        Stroke::new(
                            state.grid_size * 0.1,
                            Color32::from_rgba_unmultiplied(100, 100, 0, 100),
                        ),
                    );
                }
            }
            DragState::Idle => {
                if let Some(seg) = bd.get_hovered_segment(state) {
                    seg.highlight(state, &painter);
                }
            }
            DragState::ComponentSelected(id) => {
                if let Some(comp) = bd.get_component(&id) {
                    let rect = comp.get_grid_rect(id);
                    painter.rect_stroke(
                        Rect::from_min_max(
                            state.grid_to_screen(&rect.min),
                            state.grid_to_screen(&(rect.max + grid_pos(1, 1))),
                        ),
                        state.grid_size * 0.1,
                        Stroke::new(
                            state.grid_size * 0.15,
                            Color32::from_rgba_unmultiplied(100, 100, 0, 100),
                        ),
                        StrokeKind::Outside,
                    );
                    Self::draw_actions_panel(comp, state, ui, painter);
                }
            }
            DragState::ComponentDragged { id, grab_ofs } => {
                if let Some(pos) = state.cursor_pos {
                    draw_component_drag_preview(
                        bd,
                        state,
                        bd.get_component(&id).unwrap().get_dimension(),
                        painter,
                        pos - grab_ofs,
                        Some(id),
                    );
                }
            }
        }
    }

    fn get_action(comp: &Component, state: &FieldState) -> ComponentAction {
        if let Some(cursor_pos) = state.cursor_pos {
            let actions = comp.get_available_actions();
            for (i, rect) in ComponentAction::actions_grid(comp, state, actions.len())
                .iter()
                .enumerate()
            {
                if rect.contains(cursor_pos) {
                    return actions[i];
                }
            }
        }
        ComponentAction::None
    }

    fn draw_actions_panel(comp: &Component, state: &FieldState, ui: &egui::Ui, painter: &Painter) {
        let actions = comp.get_available_actions();
        if !actions.is_empty() {
            let visuals = &ui.style().visuals;
            let rect = ComponentAction::actions_rect(comp, state, actions.len());
            let r = rect.height() * 0.1;
            painter.add(visuals.popup_shadow.as_shape(rect, r));
            painter.rect(
                rect,
                r,
                visuals.panel_fill,
                visuals.window_stroke(),
                StrokeKind::Outside,
            );
            let grid = ComponentAction::actions_grid(comp, state, actions.len());
            actions.iter().enumerate().for_each(|(i, act)| {
                let rect = grid[i];
                let selected = if let Some(cursor_pos) = state.cursor_pos {
                    rect.contains(cursor_pos)
                } else {
                    false
                };
                act.draw(&rect, painter, selected, visuals);
            });
        }
    }
}
