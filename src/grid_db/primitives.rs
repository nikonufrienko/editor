use std::{
    f32::consts::FRAC_PI_2,
    ops::{Add, AddAssign},
};

use egui::{
    Color32, FontId, Mesh, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
    epaint::{TextShape, Vertex},
    vec2,
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

#[derive(Debug)]
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
pub enum Component {
    Unit(Unit),
}

pub struct ConnectionCell {
    pub(crate) cell: GridPos,
    pub(crate) inner_id: Id,
}

impl ConnectionCell {
    pub(crate) fn to_comp_connection(&self, component_id: usize) -> GridBDConnectionPoint {
        GridBDConnectionPoint {
            component_id: component_id,
            connection_id: self.inner_id,
        }
    }
}

impl Component {
    pub fn get_position(&self) -> GridPos {
        match self {
            Component::Unit(u) => u.pos,
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
        }
    }

    pub fn get_connection_cells(&self) -> Vec<ConnectionCell> {
        // TODO: remove it?
        match self {
            Component::Unit(unit) => unit
                .ports
                .iter()
                .enumerate()
                .map(|(i, p)| ConnectionCell {
                    cell: grid_pos(unit.pos.x + p.cell.x, unit.pos.y + p.cell.y),
                    inner_id: i,
                })
                .collect(),
        }
    }

    pub fn get_connection(&self, inner_id: Id) -> Option<Connection> {
        match self {
            Component::Unit(unit) => {
                if let Some(p) = unit.ports.get(inner_id) {
                    Some(Connection::Port(p))
                } else {
                    None
                }
            }
        }
    }

    pub fn set_pos(&mut self, pos: GridPos) {
        match self {
            Component::Unit(unit) => unit.pos = pos,
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
        }
    }

    pub fn is_hovered(&self, state: &FieldState) -> bool {
        if let Some(cursor_pos) = state.cursor_pos {
            let grid_cursor_pos = state.screen_to_grid(cursor_pos);
            let dim = self.get_dimension();
            let min: GridPos = self.get_position();
            let max = min + grid_pos(dim.0, dim.1);
            return min.x <= grid_cursor_pos.x
                && grid_cursor_pos.x <= max.x
                && min.y <= grid_cursor_pos.y
                && grid_cursor_pos.y <= max.y;
        }
        return false;
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

pub enum Connection<'a> {
    Port(&'a Port),
}

impl<'a> Connection<'a> {
    pub fn is_hovered(&self, state: &FieldState, component_pos: &GridPos) -> bool {
        match self {
            Self::Port(p) => p.is_hovered(state, component_pos),
        }
    }

    pub fn highlight(&self, state: &FieldState, comp_pos: &GridPos, painter: &Painter) {
        match self {
            Self::Port(p) => p.highlight(state, comp_pos, painter),
        }
    }

    pub fn center(&self, comp_pos: &GridPos, state: &FieldState) -> Pos2 {
        match self {
            Self::Port(p) => p.center(comp_pos, state),
        }
    }

    pub fn get_connection_align(&self) -> ConnectionAlign {
        match self {
            Self::Port(p) => p.align.clone(),
        }
    }

    pub fn get_grid_connection_offset(&self) -> GridPos {
        match self.get_connection_align() {
            ConnectionAlign::BOTTOM => grid_pos(0, 1),
            ConnectionAlign::TOP => grid_pos(0, -1),
            ConnectionAlign::LEFT => grid_pos(-1, 0),
            ConnectionAlign::RIGHT => grid_pos(1, 0),
        }
    }

    pub fn get_pos(&self, owner: &Component) -> GridPos {
        match self {
            Connection::Port(p) => p.cell + owner.get_position(),
        }
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

        let mut p1 = state.grid_to_screen(&self.pos1) + ofs;
        let mut p2 = state.grid_to_screen(&self.pos2) + ofs;

        if self.is_horizontal() {
            let h_ofs = Vec2::new(w * 0.5, 0.0);
            if self.pos1.x < self.pos2.x {
                if self.con1.is_none() {
                    p1 -= h_ofs;
                }
                if self.con2.is_none() {
                    p2 += h_ofs;
                }
            } else {
                if self.con1.is_none() {
                    p1 += h_ofs;
                }
                if self.con2.is_none() {
                    p2 -= h_ofs;
                }
            }
        }

        let mut pts = vec![p1, p2];

        if let Some(cp) = &self.con1 {
            if let Some((comp, con)) = bd.get_component_and_connection(cp) {
                pts.insert(0, con.center(&comp.get_position(), state));
            }
        }

        if let Some(cp) = &self.con2 {
            if let Some((comp, con)) = bd.get_component_and_connection(cp) {
                pts.push(con.center(&comp.get_position(), state));
            }
        }

        let color = Color32::DARK_GRAY;
        let mut mesh = Mesh::default();

        for i in 0..pts.len().saturating_sub(1) {
            let start = pts[i];
            let end = pts[i + 1];

            let delta = end - start;
            let length = delta.length();
            if length == 0.0 {
                continue;
            }
            let dir = delta / length;
            let perp = Vec2::new(-dir.y, dir.x); // перпендикуляр
            let half = perp * half_w;

            let p1 = start + half;
            let p2 = start - half;
            let p3 = end + half;
            let p4 = end - half;

            let idx_base = mesh.vertices.len() as u32;

            // Добавляем `uv: Pos2::ZERO`, даже если текстуры не используются
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

            // два треугольника на сегмент
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
