use std::{
    f32::consts::{FRAC_PI_2, PI},
    ops::{Add, AddAssign},
};

use egui::{
    epaint::{PathShape, PathStroke, TextShape}, pos2, vec2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Theme, Vec2
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    field::{Field, FieldState, SVG_DUMMY_STATE},
    grid_db::{grid_rect, mesh_line, svg_line, ComponentColor, GridBD, GridBDConnectionPoint, GridRect, Id, PrimitiveType},
};

use super::PrimitiveComponent;

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

    pub fn to_svg(
        &self,
        color: Color32,
        width: f32,
        offset: GridPos,
        bd: &GridBD,
    ) -> Option<String> {
        if self.points.is_empty() {
            return Some(String::new());
        }
        let offset_vec2 = vec2(offset.x as f32, offset.y as f32);
        let first_point = bd
            .get_component(&self.start_point.component_id)?
            .get_connection_position(self.start_point.connection_id, &SVG_DUMMY_STATE)?
            + offset_vec2;
        let last_point = bd
            .get_component(&self.end_point.component_id)?
            .get_connection_position(self.end_point.connection_id, &SVG_DUMMY_STATE)?
            + offset_vec2;
        let mut points = Vec::with_capacity(self.points.len() + 2);
        points.push(first_point);

        for i in 0..self.points.len() {
            points.push(pos2(
                (self.points[i].x + offset.x) as f32 + 0.5,
                (self.points[i].y + offset.y) as f32 + 0.5,
            ));
        }
        points.push(last_point);
        Some(svg_line(&points, color, width))
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
    const ACTIONS: &'static [ComponentAction] = &[ComponentAction::Remove];

    pub fn display(&self, state: &FieldState, painter: &Painter, theme: Theme) {
        let fill_color = theme.get_fill_color();
        let stroke_color = theme.get_stroke_color();
        let rect = Rect::from_min_size(
            state.grid_to_screen(&self.pos),
            vec2(
                state.grid_size * self.width as f32,
                state.grid_size * self.height as f32,
            ),
        );
        painter.rect_filled(rect, 0.5 * state.scale, fill_color);

        if state.scale > Field::LOD_LEVEL_MIN_SCALE {
            painter.rect_stroke(
                rect,
                0.5 * state.scale,
                Stroke::new(1.0 * state.scale, stroke_color),
                StrokeKind::Outside,
            );
            for port in &self.ports {
                port.display(&self.pos, state, &painter, theme);
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Component {
    Unit(Unit),
    Primitive(PrimitiveComponent),
}

impl Component {
    pub fn get_position(&self) -> GridPos {
        match self {
            Component::Unit(u) => u.pos,
            Component::Primitive(g) => g.pos,
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

    pub fn display(&self, state: &FieldState, painter: &Painter, theme: Theme) {
        match self {
            Component::Unit(u) => u.display(state, painter, theme),
            Component::Primitive(g) => g.display(state, painter, theme),
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
            Component::Primitive(g) => (0..g.typ.get_connections_number())
                .map(|i| g.get_connection_dock_cell(i).unwrap())
                .collect(),
        }
    }

    pub fn set_pos(&mut self, pos: GridPos) {
        match self {
            Component::Unit(unit) => unit.pos = pos,
            Component::Primitive(g) => g.pos = pos,
        }
    }

    pub fn draw_preview(&self, rect: &Rect, painter: &Painter, theme: Theme) {
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
        self.display(&state, painter, theme);
    }

    pub fn get_dimension(&self) -> (i32, i32) {
        match self {
            Component::Unit(u) => (u.width, u.height),
            Component::Primitive(g) => g.get_dimension(),
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
            Self::Primitive(g) => match dir {
                RotationDirection::Down => g.rotation = g.rotation.rotated_down(),
                RotationDirection::Up => g.rotation = g.rotation.rotated_up(),
            },
            _ => {}
        }
    }

    pub fn get_available_actions(&self) -> &'static [ComponentAction] {
        match self {
            Self::Primitive(_g) => PrimitiveComponent::ACTIONS,
            Self::Unit(_u) => Unit::ACTIONS,
        }
    }

    pub fn highlight_connection(&self, connection_id: Id, state: &FieldState, painter: &Painter) {
        match self {
            Component::Unit(unit) => {
                if let Some(p) = unit.ports.get(connection_id) {
                    p.highlight(state, &unit.pos, painter);
                }
            }
            Component::Primitive(g) => {
                g.highlight_connection(connection_id, state, painter);
            } // TODO
        }
    }

    pub fn get_connection_position(&self, connection_id: Id, state: &FieldState) -> Option<Pos2> {
        match self {
            Component::Unit(unit) => {
                let p = unit.ports.get(connection_id)?;
                Some(p.center(&unit.pos, state))
            }
            Component::Primitive(g) => g.get_connection_position(connection_id, state),
        }
    }

    pub fn get_connection_dock_cell(&self, connection_id: Id) -> Option<GridPos> {
        match self {
            Component::Unit(unit) => {
                let p = unit.ports.get(connection_id)?;
                Some(p.get_dock_cell(&unit.pos))
            }
            Component::Primitive(g) => g.get_connection_dock_cell(connection_id),
        }
    }

    pub fn is_connection_hovered(&self, connection_id: Id, state: &FieldState) -> bool {
        match self {
            Component::Unit(unit) => unit
                .ports
                .get(connection_id)
                .is_some_and(|p| p.is_hovered(state, &unit.pos)),
            Component::Primitive(g) => g.is_connection_hovered(connection_id, state),
        }
    }

    pub fn to_svg(&self, offset: GridPos, theme: Theme) -> String {
        match self {
            Component::Primitive(g) => g.get_svg(offset, theme),
            _ => "".into(), // TODO: fixme
        }
    }

    /// Should I only check the overlap for this component?
    pub fn is_overlap_only(&self) -> bool {
        match self {
            Component::Primitive(g) => match g.typ {
                PrimitiveType::Point => true,
                _ => false
            },
            _ => false
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
        self.cell
            + unit_pos.clone()
            + match self.align {
                ConnectionAlign::BOTTOM => grid_pos(0, 1),
                ConnectionAlign::TOP => grid_pos(0, -1),
                ConnectionAlign::LEFT => grid_pos(-1, 0),
                ConnectionAlign::RIGHT => grid_pos(1, 0),
            }
    }

    pub fn display(&self, unit_pos: &GridPos, state: &FieldState, painter: &Painter, theme: Theme) {
        let fill_color = theme.get_fill_color();
        let stroke_color = theme.get_stroke_color();
        let text_color = theme.get_text_color();
        let angle = self.align.rotation_angle();
        let pos = self.center(unit_pos, state);
        painter.circle_filled(pos, state.grid_size * Self::PORT_SCALE, fill_color);
        painter.circle_stroke(
            pos,
            state.grid_size * Self::PORT_SCALE,
            Stroke::new(1.0 * state.scale, stroke_color),
        );
        if state.label_visible {
            let galley = painter.fonts(|fonts| {
                fonts.layout_no_wrap(self.name.clone(), state.label_font.clone(), text_color)
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
            painter.add(TextShape::new(text_pos, galley, text_color).with_angle(angle));
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

    pub fn get_mesh(&self, bd: &GridBD, state: &FieldState, theme: Theme) -> Mesh {
        let w = (state.grid_size * 0.1).max(1.0);
        let ofs = Vec2::new(0.5 * state.grid_size, 0.5 * state.grid_size);
        let color = theme.get_stroke_color();

        let p1 = state.grid_to_screen(&self.pos1) + ofs;
        let p2 = state.grid_to_screen(&self.pos2) + ofs;

        let mut pts = vec![p1, p2];

        if let Some(cp) = &self.con1 {
            if let Some(comp) = bd.get_component(&cp.component_id) {
                pts.insert(
                    0,
                    comp.get_connection_position(cp.connection_id, state)
                        .unwrap(),
                );
            }
        }

        if let Some(cp) = &self.con2 {
            if let Some(comp) = bd.get_component(&cp.component_id) {
                pts.push(
                    comp.get_connection_position(cp.connection_id, state)
                        .unwrap(),
                );
            }
        }

        mesh_line(pts, w, color)
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
pub enum ComponentAction {
    RotateUp,
    RotateDown,
    Remove,
    None,
}

impl ComponentAction {
    fn draw_rotation_arrow(
        painter: &Painter,
        center: Pos2,
        radius: f32,
        clockwise: bool,
        stroke: Stroke,
    ) {
        let sweep_angle = if clockwise { 1.7 * PI } else { -1.7 * PI };
        let start_angle = if clockwise { 0.0 } else { PI };
        let eps = if clockwise { 0.2 } else { -0.2 };

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
        let pos = state.grid_to_screen(&comp.get_position())
            + vec2(
                w as f32 * 0.5 * state.grid_size - n_actions as f32 * 0.5 * size,
                -size * 1.2,
            );
        (0..n_actions)
            .map(|i| Rect::from_min_size(pos + vec2(size * i as f32, 0.0), vec2(size, size)))
            .collect()
    }

    pub fn actions_rect(comp: &Component, state: &FieldState, n_actions: usize) -> Rect {
        let (w, _h) = comp.get_dimension();
        let size = 50.0;
        let pos = state.grid_to_screen(&comp.get_position())
            + vec2(
                w as f32 * 0.5 * state.grid_size - n_actions as f32 * 0.5 * size,
                -size * 1.2,
            );
        Rect::from_min_size(pos, vec2(size * n_actions as f32, size))
    }

    pub fn draw(
        &self,
        rect: &Rect,
        painter: &egui::Painter,
        selected: bool,
        visuals: &egui::Visuals,
    ) {
        let stroke = if selected {
            Stroke::new(rect.height() / 8.0, visuals.strong_text_color())
        } else {
            Stroke::new(rect.height() / 8.0, visuals.text_color())
        };
        match self {
            Self::RotateDown => {
                Self::draw_rotation_arrow(
                    &painter,
                    rect.center(),
                    rect.height() * 0.3,
                    false,
                    stroke,
                );
            }
            Self::RotateUp => {
                Self::draw_rotation_arrow(
                    &painter,
                    rect.center(),
                    rect.height() * 0.3,
                    true,
                    stroke,
                );
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

pub enum RotationDirection {
    Up,
    Down,
}
