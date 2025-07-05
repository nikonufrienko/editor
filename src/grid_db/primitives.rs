use std::{ f32::consts::{FRAC_PI_2, PI}, ops::{Add, AddAssign}
};

use egui::{
    epaint::{PathShape, PathStroke, TextShape, Vertex}, vec2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    field::{Field, FieldState},
    grid_db::{GridBD, GridBDConnectionPoint, GridRect, Id, grid_rect},
};

#[serde_as]
#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
#[serde(from = "[i32; 2]", into = "[i32; 2]")]
pub struct GridPos {
    #[serde_as(as = "FromInto<i32>")]
    pub x: i32,
    #[serde_as(as = "FromInto<i32>")]
    pub y: i32,
}

impl From<[i32; 2]> for GridPos {
    fn from(arr: [i32; 2]) -> Self {
        GridPos {
            x: arr[0],
            y: arr[1],
        }
    }
}

impl Into<[i32; 2]> for GridPos {
    fn into(self) -> [i32; 2] {
        [self.x, self.y]
    }
}

impl GridPos {
    pub(crate) fn to_point(&self) -> [i32; 2] {
        return [self.x, self.y];
    }
}

impl Add for GridPos {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        grid_pos(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for GridPos {
    fn add_assign(&mut self, rhs: Self) {
        *self = grid_pos(self.x + rhs.x, self.y + rhs.y);
    }
}

pub fn grid_pos(x: i32, y: i32) -> GridPos {
    GridPos { x, y }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ConnectionAlign {
    LEFT,
    RIGHT,
    TOP,
    BOTTOM,
} // TODO: add custom

impl ConnectionAlign {
    pub(crate) fn grid_offset(&self) -> Vec2 {
        match self {
            Self::LEFT => vec2(0.0, 0.5),
            Self::RIGHT => vec2(1.0, 0.5),
            Self::TOP => vec2(0.5, 0.0),
            Self::BOTTOM => vec2(0.5, 1.0),
        }
    }

    pub(crate) fn rotation_angle(&self) -> f32 {
        match self {
            Self::LEFT => 0.0,
            Self::RIGHT => 0.0,
            Self::TOP => FRAC_PI_2,
            Self::BOTTOM => -FRAC_PI_2,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Port {
    // Connection
    pub cell: GridPos,
    pub align: ConnectionAlign,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Net {
    pub start_point: GridBDConnectionPoint,
    pub end_point: GridBDConnectionPoint,
    pub points: Vec<GridPos>,
}

impl Net {
    pub fn get_segments(&self, net_id: Id) -> Vec<NetSegment> {
        // TODO: return iterator?
        let mut result = vec![];
        for i in 0..self.points.len() - 1 {
            result.push(NetSegment::new(
                i,
                net_id,
                self.points[i],
                self.points[i + 1],
                (i == 0).then_some(self.start_point),
                (i == self.points.len() - 2).then_some(self.end_point),
            ));
        }
        result
    }

    pub fn get_segment(&self, segment_id: Id, net_id: Id) -> Option<NetSegment> {
        Some(NetSegment::new(
            segment_id,
            net_id,
            *self.points.get(segment_id)?,
            *self.points.get(segment_id + 1)?,
            (segment_id == 0).then_some(self.start_point),
            (segment_id == self.points.len() - 2).then_some(self.end_point),
        ))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Unit {
    pub name: String,
    pub pos: GridPos,
    pub width: i32,
    pub height: i32,
    pub ports: Vec<Port>,
}

impl Unit {
    const ACTIONS:&'static [ComponentAction] = &[ComponentAction::Remove];

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        // TODO: Add LOD level
        // 1. Display unit with ports and labels
        // 2. Display display only Rectangle
        let rect = Rect::from_min_size(
            state.grid_to_screen(&self.pos),
            vec2(
                state.grid_size * self.width as f32,
                state.grid_size * self.height as f32,
            ),
        );
        painter.rect_filled(rect, 0.5 * state.scale, Color32::GRAY);

        if state.scale > Field::LOD_LEVEL0_SCALE {
            painter.rect_stroke(
                rect,
                0.5 * state.scale,
                Stroke::new(1.0 * state.scale, Color32::DARK_GRAY),
                StrokeKind::Outside,
            );
            for port in &self.ports {
                port.display(&self.pos, state, &painter);
            }
        }
    }
}




#[derive(Clone, Serialize, Deserialize)]
pub enum Rotation {
    ROT0, ROT90, ROT180, ROT270
}

impl Rotation {
    fn rotated_up(&self) -> Rotation {
        match self {
            Rotation::ROT0      => Rotation::ROT90,
            Rotation::ROT90     => Rotation::ROT180,
            Rotation::ROT180    => Rotation::ROT270,
            Rotation::ROT270    => Rotation::ROT0,
        }
    }

    fn rotated_down(&self) -> Rotation {
        match self {
            Rotation::ROT90     => Rotation::ROT0,
            Rotation::ROT180    => Rotation::ROT90,
            Rotation::ROT270    => Rotation::ROT180,
            Rotation::ROT0      => Rotation::ROT270,
        }
    }

    fn cos(&self) -> i32 {
        match self {
            Rotation::ROT90     => 0,
            Rotation::ROT180    => -1,
            Rotation::ROT270    => 0,
            Rotation::ROT0      => 1,
        }
    }

    fn sin(&self) -> i32 {
        match self {
            Rotation::ROT90     => 1,
            Rotation::ROT180    => 0,
            Rotation::ROT270    => -1,
            Rotation::ROT0      => 0,
        }
    }

    fn rotate_grid_pos(&self, point: GridPos, center: GridPos) -> GridPos{
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let cos_a = self.cos();
        let sin_a = self.sin();
        grid_pos(
            center.x + dx * cos_a - dy  * sin_a,
            center.y + dx * sin_a + dy * cos_a,
        )
    }

    fn rotate_point(&self, point: Pos2, center: Pos2) -> Pos2 {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let cos_a = self.cos() as f32;
        let sin_a = self.sin() as f32;
        Pos2::new(
            center.x + dx * cos_a - dy * sin_a,
            center.y + dx * sin_a + dy * cos_a,
        )
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub struct AndGate {
    pub n_inputs: u32,
    pub pos : GridPos,
    pub rotation : Rotation
}

impl AndGate {
    const ACTIONS:&'static [ComponentAction] = &[ComponentAction::RotateDown, ComponentAction::RotateUp, ComponentAction::Remove];
    const CONNECTION_SCALE:f32 = 0.1;

    fn get_dimension_raw(&self) -> (i32, i32) {
        if self.n_inputs % 2 == 0 {
            (2, (2 * self.n_inputs - 1) as i32)
        } else {
            (2, self.n_inputs as i32)
        }
    }

    fn get_dimension(&self) -> (i32, i32) {
        let (w, h) = self.get_dimension_raw();
        match self.rotation {
            Rotation::ROT0 => (w, h),
            Rotation::ROT90 => (h, w),
            Rotation::ROT180 => (w, h),
            Rotation::ROT270 => (h, w),
        }
    }

    fn apply_rotation(&self, points: Vec<Pos2>, state: &FieldState) -> Vec<Pos2> {
        let rot_center = state.grid_to_screen(&self.pos);
        let dim = self.get_dimension();
        let rot_ofs = match self.rotation {
            Rotation::ROT0   => vec2(0.0, 0.0),
            Rotation::ROT90  => vec2(dim.0 as f32, 0.0)  * state.grid_size,
            Rotation::ROT180 => vec2(dim.0 as f32, dim.1 as f32) * state.grid_size,
            Rotation::ROT270 => vec2(0.0, dim.1 as f32) * state.grid_size,
        };
        points.iter().map(|p| {self.rotation.rotate_point(*p, rot_center) + rot_ofs}).collect()
    }

    fn apply_rotation_grid_pos(&self, points: Vec<GridPos>) -> Vec<GridPos> {
        let rot_center = self.pos;
        let dim = self.get_dimension();
        let rot_ofs = match self.rotation {
            Rotation::ROT0   => grid_pos(0, 0),
            Rotation::ROT90  => grid_pos(dim.0-1, 0),
            Rotation::ROT180 => grid_pos(dim.0-1, dim.1-1),
            Rotation::ROT270 => grid_pos(0, dim.1-1),
        };
        points.iter().map(|p| {self.rotation.rotate_grid_pos(*p, rot_center) + rot_ofs}).collect()
    }

    fn get_connection_dock_cell(&self, connection_id: Id) -> Option<GridPos> {
        let raw_dim = self.get_dimension_raw();
        if connection_id < self.n_inputs as Id + 1 {
            let pos = if connection_id < self.n_inputs as Id {
                if self.n_inputs % 2 == 0 {
                    self.pos + grid_pos(-1, 2 * connection_id as i32)
                } else {
                    self.pos + grid_pos(-1, connection_id as i32)
                }
            } else {
                self.pos + grid_pos(raw_dim.0, raw_dim.1 / 2)
            };
            Some(self.apply_rotation_grid_pos(vec![pos])[0])
        } else {
            None
        }
    }

    fn get_connection_position(&self, connection_id: Id, state: &FieldState) -> Option<Pos2> {
        let screen_pos = state.grid_to_screen(&self.pos);
        let (w, h) = self.get_dimension_raw();
        if connection_id < self.n_inputs as Id + 1 {
            let pos = if connection_id < self.n_inputs as Id {
                if self.n_inputs % 2 == 0 {
                    screen_pos + vec2(0.0, ((2 * connection_id) as f32 + 0.5) * state.grid_size)
                } else {
                    screen_pos + vec2(0.0, (connection_id as f32 + 0.5) * state.grid_size)
                }
            } else {
                screen_pos + vec2(w as f32 * state.grid_size, h as f32 * state.grid_size / 2.0)
            };
            Some(self.apply_rotation(vec![pos], state)[0])
        } else {
            None
        }
    }

    pub fn is_connection_hovered(&self, connection_id: Id, state: &FieldState) -> bool {
        if let Some(cursor_pos) = state.cursor_pos {
            if let Some(con_pos) = self.get_connection_position(connection_id, state) {
                let d = con_pos.distance(cursor_pos);
                d <= state.grid_size * Self::CONNECTION_SCALE * 2.0
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn highlight_connection(&self, connection_id: Id, state: &FieldState, painter: &Painter) {
        if let Some(p) = self.get_connection_position(connection_id, state) {
            painter.circle_filled(
                p,
                state.grid_size * Self::CONNECTION_SCALE * 3.0,
                Color32::from_rgba_unmultiplied(100, 100, 0, 100),
            );
        }
    }

    fn display(&self, state: &FieldState, painter: &Painter) {
        let stroke_w = 1.0 * state.scale;
        let height = if self.n_inputs % 2 == 0 {
            (2 * self.n_inputs - 1) as f32
        } else {
            self.n_inputs as f32
        } * state.grid_size;
        let radius_x = state.grid_size - stroke_w / 2.0;
        let radius_y = height as f32 / 2.0 - stroke_w / 2.0;
        let pos = state.grid_to_screen(&self.pos);
        let center = pos + vec2(state.grid_size, height / 2.0);

        let n_points = 30;
        let mut points = (0..=n_points).map(|i| {
            let angle = -PI / 2.0 + PI * (i as f32 / n_points as f32);
            let x = center.x + radius_x * angle.cos();
            let y = center.y + radius_y * angle.sin();
            Pos2::new(x, y)
        }).collect::<Vec<_>>();
        points.insert(0, pos + vec2(stroke_w / 2.0, stroke_w / 2.0));
        points.insert(0, pos + vec2(stroke_w / 2.0, height - stroke_w / 2.0) );

        let polygon = Shape::convex_polygon(
            self.apply_rotation(points, state),
            Color32::GRAY,
            Stroke::new(stroke_w, Color32::DARK_GRAY),
        );
        painter.add(polygon);

        let inc = if self.n_inputs % 2 == 0 {2} else {1};
        let mut port_points: Vec<Pos2> = (0..self.n_inputs).map(|i|{pos + vec2(0.0, (i * inc) as f32 * state.grid_size + state.grid_size/2.0)}).collect();
        port_points.push(pos + vec2(self.get_dimension_raw().0 as f32 * state.grid_size, height / 2.0));
        let port_points = self.apply_rotation(port_points, state);
        port_points.iter().for_each(|p| {
            painter.circle_filled(p.clone(), state.grid_size * Self::CONNECTION_SCALE, Color32::DARK_GRAY);
        });
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub enum Component {
    Unit(Unit),
    AndGate(AndGate),
}

impl Component {
    pub fn get_position(&self) -> GridPos {
        match self {
            Component::Unit(u) => u.pos,
            Component::AndGate(g) => g.pos
        }
    }

    pub fn get_grid_rect(&self, id: Id) -> GridRect {
        let dim = self.get_dimension();
        grid_rect(
            id,
            self.get_position(),
            self.get_position() + grid_pos(dim.0 - 1, dim.1 - 1),
        )
    }

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        match self {
            Component::Unit(u) => u.display(state, painter),
            Component::AndGate(g) => g.display(state, painter)
        }
    }

    pub fn get_connection_dock_cells(&self) -> Vec<GridPos> {
        match self {
            Component::Unit(unit) => unit
                .ports
                .iter()
                .enumerate()
                .map(|(_i, p)| p.get_dock_cell(&unit.pos))
                .collect(),
            Component::AndGate(g) => (0..=g.n_inputs as Id)
                .map(|i| g.get_connection_dock_cell(i).unwrap()).collect()
        }
    }

    pub fn set_pos(&mut self, pos: GridPos) {
        match self {
            Component::Unit(unit) => unit.pos = pos,
            Component::AndGate(g) => g.pos = pos,
        }
    }

    pub fn draw_preview(&self, rect: &Rect, painter: &Painter) {
        let (mut w, mut h) = self.get_dimension();
        w += 2;
        h += 2;
        let x_grid_size = rect.width() / w as f32;
        let y_grid_size = rect.height() / h as f32;
        let grid_size = x_grid_size.min(y_grid_size);
        let scale = grid_size / Field::BASE_GRID_SIZE;
        let state = FieldState {
            scale: grid_size / Field::BASE_GRID_SIZE,
            offset: Vec2::default(),
            grid_size: grid_size,
            rect: rect.clone(), // ?? TODO make it as Option
            label_font: FontId::monospace(
                (Field::BASE_GRID_SIZE * scale * 0.5).min(Field::MAX_FONT_SIZE),
            ),
            label_visible: true,
            cursor_pos: None,
        };
        self.display(&state, painter);
    }

    pub fn get_dimension(&self) -> (i32, i32) {
        match self {
            Component::Unit(u) => (u.width, u.height),
            Component::AndGate(g) => g.get_dimension()
        }
    }

    pub fn is_hovered(&self, state: &FieldState) -> bool {
        if let Some(cursor_pos) = state.cursor_pos {
            let grid_cursor_pos = state.screen_to_grid(cursor_pos);
            let dim = self.get_dimension();
            let min: GridPos = self.get_position();
            let max = min + grid_pos(dim.0 - 1, dim.1 - 1);
            return min.x <= grid_cursor_pos.x
                && grid_cursor_pos.x <= max.x
                && min.y <= grid_cursor_pos.y
                && grid_cursor_pos.y <= max.y;
        }
        return false;
    }

    pub fn rotate(&mut self, dir: RotationDirection) {
        match self {
            Self::AndGate(g) => {
                match dir {
                    RotationDirection::Down => g.rotation = g.rotation.rotated_down(),
                    RotationDirection::Up   => g.rotation = g.rotation.rotated_up(),
                }
            },
            _ => {}
        }
    }

    pub fn get_available_actions(&self) -> &'static [ComponentAction] {
        match self {
            Self::AndGate(_g) => AndGate::ACTIONS,
            Self::Unit(_u) => Unit::ACTIONS,
        }
    }

    pub fn highlight_connection(&self, connection_id: Id, state: &FieldState, painter: &Painter) {
        match self {
            Component::Unit(unit) => {
                if let Some(p) = unit.ports.get(connection_id) {
                    p.highlight(state, &unit.pos, painter);
                }
            },
            Component::AndGate(g) => {g.highlight_connection(connection_id, state, painter);}, // TODO
        }
    }

    pub fn get_connection_position(&self, connection_id: Id, state: &FieldState) -> Option<Pos2> {
        match self {
            Component::Unit(unit) => {
                let p= unit.ports.get(connection_id)?;
                Some(p.center(&unit.pos, state))
            },
            Component::AndGate(g) => g.get_connection_position(connection_id, state),
        }
    }

    pub fn get_connection_dock_cell(&self, connection_id: Id) -> Option<GridPos> {
        match self {
            Component::Unit(unit) => {
                let p= unit.ports.get(connection_id)?;
                Some(p.get_dock_cell(&unit.pos))
            },
            Component::AndGate(g) => g.get_connection_dock_cell(connection_id),
        }
    }

    pub fn is_connection_hovered(&self, connection_id: Id, state: &FieldState) -> bool{
        match self {
            Component::Unit(unit) => unit.ports.get(connection_id).is_some_and(|p| {p.is_hovered(state, &unit.pos)}),
            Component::AndGate(g) => g.is_connection_hovered(connection_id, state),
        }
    }
}

impl Port {
    const PORT_SCALE: f32 = 0.1;

    pub fn center(&self, unit_pos: &GridPos, state: &FieldState) -> Pos2 {
        state.grid_to_screen(&GridPos {
            x: unit_pos.x + self.cell.x,
            y: unit_pos.y + self.cell.y,
        }) + self.align.grid_offset() * state.grid_size
    }

    pub fn get_dock_cell(&self, unit_pos: &GridPos) -> GridPos {
        self.cell + unit_pos.clone() +
        match self.align {
            ConnectionAlign::BOTTOM => grid_pos(0, 1),
            ConnectionAlign::TOP => grid_pos(0, -1),
            ConnectionAlign::LEFT => grid_pos(-1, 0),
            ConnectionAlign::RIGHT => grid_pos(1, 0),
        }
    }

    pub fn display(&self, unit_pos: &GridPos, state: &FieldState, painter: &Painter) {
        let angle = self.align.rotation_angle();
        let pos = self.center(unit_pos, state);
        painter.circle_filled(pos, state.grid_size * Self::PORT_SCALE, Color32::GRAY);
        painter.circle_stroke(
            pos,
            state.grid_size * Self::PORT_SCALE,
            Stroke::new(1.0 * state.scale, Color32::DARK_GRAY),
        );
        if state.label_visible {
            let galley = painter.fonts(|fonts| {
                fonts.layout_no_wrap(self.name.clone(), state.label_font.clone(), Color32::WHITE)
            });
            let label_rect = galley.rect;

            let mut text_pos = state.grid_to_screen(&GridPos {
                x: unit_pos.x + self.cell.x,
                y: unit_pos.y + self.cell.y,
            });
            match self.align {
                ConnectionAlign::LEFT => {
                    text_pos.y += state.grid_size / 2.0 - label_rect.height() / 2.0;
                    text_pos.x += state.grid_size * 0.5;
                    // TODO:
                }
                ConnectionAlign::RIGHT => {
                    text_pos.y += state.grid_size / 2.0 - label_rect.height() / 2.0;
                    text_pos.x -= label_rect.width() - state.grid_size * 0.5;
                }
                ConnectionAlign::TOP => {
                    text_pos.x += (state.grid_size + label_rect.width() / 2.0) / 2.0;
                    text_pos.y += state.grid_size * 0.5;
                }
                ConnectionAlign::BOTTOM => {}
            }
            painter.add(TextShape::new(text_pos, galley, Color32::WHITE).with_angle(angle));
        }
    }

    pub fn is_hovered(&self, state: &FieldState, unit_pos: &GridPos) -> bool {
        if let Some(cursor_pos) = state.cursor_pos {
            let d = self.center(unit_pos, state).distance(cursor_pos);
            d <= state.grid_size * Self::PORT_SCALE * 2.0
        } else {
            false
        }
    }

    pub fn highlight(&self, state: &FieldState, unit_pos: &GridPos, painter: &Painter) {
        let p = self.center(unit_pos, state);
        painter.circle_filled(
            p,
            state.grid_size * Self::PORT_SCALE * 3.0,
            Color32::from_rgba_unmultiplied(100, 100, 0, 100),
        );
    }
}

pub struct NetSegment {
    pub inner_id: Id, // ID of segment in net
    pub net_id: Id,   // ID of net
    pub pos1: GridPos,
    pub pos2: GridPos,
    con1: Option<GridBDConnectionPoint>, // if segment
    con2: Option<GridBDConnectionPoint>, // Second position
}

impl NetSegment {
    pub fn new(
        inner_id: Id,
        net_id: Id,
        pos1: GridPos,
        pos2: GridPos,
        con1: Option<GridBDConnectionPoint>,
        con2: Option<GridBDConnectionPoint>,
    ) -> Self {
        Self {
            inner_id,
            net_id,
            pos1,
            pos2,
            con1,
            con2,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        self.pos1.y == self.pos2.y
    }

    pub fn get_mesh(&self, bd: &GridBD, state: &FieldState) -> Mesh {
        let w = (state.grid_size * 0.1).max(1.0);
        let half_w = w * 0.5;
        let ofs = Vec2::new(0.5 * state.grid_size, 0.5 * state.grid_size);

        let p1 = state.grid_to_screen(&self.pos1) + ofs;
        let p2 = state.grid_to_screen(&self.pos2) + ofs;

        let mut pts = vec![p1, p2];

        if let Some(cp) = &self.con1 {
            if let Some(comp) = bd.get_component(&cp.component_id) {
                pts.insert(0, comp.get_connection_position(cp.connection_id, state).unwrap());
            }
        }

        if let Some(cp) = &self.con2 {
            if let Some(comp) = bd.get_component(&cp.component_id) {
                pts.push(comp.get_connection_position(cp.connection_id, state).unwrap());
            }
        }

        let color = Color32::DARK_GRAY;
        let mut mesh = Mesh::default();

        for i in 0..pts.len() - 1 {
            let start = pts[i];
            let end = pts[i + 1];

            let delta = end - start;
            let length = delta.length();
            if length == 0.0 {
                continue;
            }
            let dir = delta / length;
            let perp = Vec2::new(-dir.y, dir.x);
            let half = perp * half_w;

            let p1 = start + half - dir * half_w;
            let p2 = start - half - dir * half_w;
            let p3 = end + half + dir * half_w;
            let p4 = end - half + dir * half_w;

            let idx_base = mesh.vertices.len() as u32;

            mesh.vertices.push(Vertex {
                pos: p1,
                uv: Pos2::ZERO,
                color,
            });
            mesh.vertices.push(Vertex {
                pos: p2,
                uv: Pos2::ZERO,
                color,
            });
            mesh.vertices.push(Vertex {
                pos: p3,
                uv: Pos2::ZERO,
                color,
            });
            mesh.vertices.push(Vertex {
                pos: p4,
                uv: Pos2::ZERO,
                color,
            });

            mesh.indices.extend_from_slice(&[
                idx_base,
                idx_base + 1,
                idx_base + 2,
                idx_base + 2,
                idx_base + 1,
                idx_base + 3,
            ]);
        }

        mesh
    }


    pub fn is_hovered(&self, state: &FieldState) -> bool {
        let ofs = Vec2::new(0.5 * state.grid_size, 0.5 * state.grid_size);
        let Pos2 { x: ax, y: ay } = state.grid_to_screen(&self.pos1) + ofs;
        let Pos2 { x: bx, y: by } = state.grid_to_screen(&self.pos2) + ofs;
        if let Some(Pos2 { x: px, y: py }) = state.cursor_pos {
            if if self.is_horizontal() {
                ax.min(bx) > px || px > ax.max(bx)
            } else {
                ay.min(by) > py || py > ay.max(by)
            } {
                return false;
            }
            let abx = bx - ax;
            let aby = by - ay;
            let apx = px - ax;
            let apy = py - ay;
            let cross = abx * apy - aby * apx;
            let length_ab_sq = abx.powi(2) + aby.powi(2);
            return ((cross * cross) / length_ab_sq) < (state.grid_size * 0.3).powi(2);
        }
        false
    }

    pub fn highlight(&self, state: &FieldState, painter: &Painter) {
        let ofs = Vec2::new(0.5 * state.grid_size, 0.5 * state.grid_size);

        let p1 = state.grid_to_screen(&self.pos1) + ofs;
        let p2 = state.grid_to_screen(&self.pos2) + ofs;

        painter.line_segment(
            [p1, p2],
            Stroke::new(
                (state.grid_size * 0.3).max(1.0),
                Color32::from_rgba_unmultiplied(100, 100, 0, 100),
            ),
        );
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ComponentAction {RotateUp, RotateDown, Remove, None}

impl ComponentAction {
    fn draw_rotation_arrow(
        painter: &Painter,
        center: Pos2,
        radius: f32,
        clockwise: bool,
        stroke: Stroke,
    ) {
        let sweep_angle = if clockwise { 1.7 * PI } else { -1.7 * PI };
        let start_angle = if clockwise {0.0} else  {PI };
        let eps =if clockwise { 0.2 } else { -0.2 };

        let num_segments = 30;
        let mut points = Vec::with_capacity(num_segments + 1);

        for i in 0..=num_segments {
            let t = i as f32 / num_segments as f32;
            let angle = start_angle + t * sweep_angle;
            points.push(center + radius * Vec2::angled(angle));
        }

        painter.add(Shape::Path(PathShape {
            points,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke::new(stroke.width, stroke.color),
        }));

        let r_vec = radius * Vec2::angled(start_angle + sweep_angle - eps);
        let p1 = center + r_vec + r_vec.normalized() * stroke.width;
        let p2 = center + r_vec - r_vec.normalized() * stroke.width;
        let p3 = if clockwise {
            ((p2 - p1).rot90() + (p1.to_vec2() + p2.to_vec2()) / 2.0).to_pos2()
        } else {
            ((p1 - p2).rot90() + (p1.to_vec2() + p2.to_vec2()) / 2.0).to_pos2()
        };

        painter.add(Shape::convex_polygon(
            vec![p1, p2, p3],
            stroke.color,
            Stroke::NONE,
        ));
    }

    pub fn actions_grid(comp: &Component, state: &FieldState, n_actions: usize) -> Vec<Rect> {
        let (w, _h) = comp.get_dimension();
        let size = 50.0;
        let pos = state.grid_to_screen(&comp.get_position()) + vec2(w as f32 * 0.5 * state.grid_size - n_actions as f32 * 0.5 * size, -size * 1.2);
        (0..n_actions).map(|i| {
            Rect::from_min_size( pos + vec2(size * i as f32, 0.0), vec2(size, size))
        }).collect()
    }

    pub fn actions_rect(comp: &Component, state: &FieldState, n_actions: usize) -> Rect {
        let (w, _h) = comp.get_dimension();
        let size = 50.0;
        let pos = state.grid_to_screen(&comp.get_position()) + vec2(w as f32 * 0.5 * state.grid_size - n_actions as f32 * 0.5 * size, -size * 1.2);
        Rect::from_min_size(pos, vec2(size * n_actions as f32, size))
    }

    pub fn draw(&self, rect:&Rect, painter: &egui::Painter, selected: bool, visuals: &egui::Visuals) {
        let stroke = if selected {
            Stroke::new(rect.height() / 8.0, visuals.strong_text_color(),)
        } else {
            Stroke::new(rect.height() / 8.0, visuals.text_color())
        };
        match self {
            Self::RotateDown => {
                Self::draw_rotation_arrow(&painter, rect.center(), rect.height() * 0.3, false, stroke);
            }
            Self::RotateUp => {
                Self::draw_rotation_arrow(&painter, rect.center(), rect.height() * 0.3, true, stroke);
            }
            Self::Remove => {
                let scaled = rect.scale_from_center(0.6);
                painter.line_segment([scaled.left_top(), scaled.right_bottom()], stroke);
                painter.line_segment([scaled.left_bottom(), scaled.right_top()], stroke);
            }
            _ => {}
        }
    }
}

pub enum RotationDirection {Up, Down}
