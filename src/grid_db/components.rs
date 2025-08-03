use std::{
    f32::consts::PI,
    ops::{Add, AddAssign},
    vec,
};

use egui::{epaint::{PathShape, PathStroke}, pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Theme, Vec2
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    field::{Field, FieldState, SVG_DUMMY_STATE},
    grid_db::{
        ComponentColor, GridBD, GridBDConnectionPoint, GridRect, Id, LodLevel, PrimitiveType,
        Rotation, STROKE_SCALE, TextField, grid_rect, mesh_line, show_text_with_debounce,
        svg_circle_filled, svg_line, svg_rect, svg_single_line_text,
    },
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

trait TextAlignment {
    fn to_text_rotation(&self) -> Self;
    fn to_text_align2(&self) -> Align2;
}

impl TextAlignment for Rotation {
    fn to_text_rotation(&self) -> Self {
        match self {
            Self::ROT0 => Self::ROT0,
            Self::ROT90 => Self::ROT90,
            Self::ROT180 => Self::ROT0,
            Self::ROT270 => Self::ROT90,
        }
    }

    fn to_text_align2(&self) -> Align2 {
        match self {
            Self::ROT0 => Align2::LEFT_CENTER,
            Self::ROT90 => Align2::LEFT_CENTER,
            Self::ROT180 => Align2::RIGHT_CENTER,
            Self::ROT270 => Align2::RIGHT_CENTER,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Net {
    pub start_point: GridBDConnectionPoint,
    pub end_point: GridBDConnectionPoint,
    pub points: Vec<GridPos>,
}

impl Net {
    pub fn get_segments(&self, net_id: Id) -> Vec<NetSegment> {
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
        scale: f32,
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
        points.push(first_point * scale);

        for i in 0..self.points.len() {
            points.push(
                pos2(
                    (self.points[i].x + offset.x) as f32 + 0.5,
                    (self.points[i].y + offset.y) as f32 + 0.5,
                ) * scale,
            );
        }
        points.push(last_point * scale);
        Some(svg_line(&points, color, width))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Unit {
    pub pos: GridPos,
    pub width: i32,
    pub height: i32,
    pub ports: Vec<Port>,
}

impl Unit {
    const ACTIONS: &'static [ComponentAction] = &[
        ComponentAction::AddPort,
        ComponentAction::EditPort,
        ComponentAction::RemovePort,
        ComponentAction::Remove,
    ];

    pub fn display(&self, state: &FieldState, painter: &Painter, theme: Theme) {
        let fill_color = theme.get_fill_color();
        let rect = Rect::from_min_size(
            state.grid_to_screen(&self.pos) + vec2(0.05, 0.05) * state.grid_size,
            vec2(
                state.grid_size * (self.width as f32 - 0.1),
                state.grid_size * (self.height as f32 - 0.1),
            ),
        );
        painter.rect(
            rect,
            0.5 * state.scale,
            fill_color,
            theme.get_stroke(state),
            StrokeKind::Middle,
        );

        if state.scale > Field::LOD_LEVEL_MIN_SCALE {
            for port in &self.ports {
                port.display(&self.pos, (self.width, self.height), state, &painter, theme);
            }
        }
    }

    fn resize(&mut self, size: (i32, i32)) {
        let mut min_w = 1;
        let mut min_h = 1;
        for Port {
            align,
            offset,
            name: _name,
        } in &self.ports
        {
            if [Rotation::ROT0, Rotation::ROT180].contains(align) && offset + 1 > min_h {
                min_h = *offset + 1;
            }
            if [Rotation::ROT270, Rotation::ROT90].contains(align) && offset + 1 > min_w {
                min_w = *offset + 1;
            }
        }
        (self.width, self.height) = (size.0.max(min_w), size.1.max(min_h));
    }

    fn get_nearest_port_pos(
        &self,
        state: &FieldState,
        used: bool,
    ) -> Option<(Rotation, i32, Option<Id>)> {
        let (w, h) = (self.width, self.height);
        let grid_pos = self.pos;
        let rect = Rect::from_min_size(
            state.grid_to_screen(&grid_pos),
            vec2(w as f32 * state.grid_size, h as f32 * state.grid_size),
        );
        if let Some(cursor_pos) = state.cursor_pos {
            if rect.contains(cursor_pos) {
                let Pos2 { x, y } = cursor_pos;
                let GridPos {
                    x: grid_x,
                    y: grid_y,
                } = state.screen_to_grid(cursor_pos);
                let distances = [
                    x - rect.left(),
                    rect.right() - x,
                    rect.bottom() - y,
                    y - rect.top(),
                ];
                let (rotation, offset) = match distances
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .unwrap()
                    .0
                {
                    0 => (Rotation::ROT0, grid_y - grid_pos.y),
                    1 => (Rotation::ROT180, grid_y - grid_pos.y),
                    2 => (Rotation::ROT270, grid_x - grid_pos.x),
                    3 => (Rotation::ROT90, grid_x - grid_pos.x),
                    _ => panic!(),
                };
                for (id, port) in self.ports.iter().enumerate() {
                    if port.align == rotation && port.offset == offset {
                        if used {
                            return Some((rotation, offset, Some(id)));
                        } else {
                            return None;
                        }
                    }
                }
                if !used {
                    return Some((rotation, offset, None));
                }
            }
        }

        None
    }

    fn to_svg(&self, offset: GridPos, scale: f32, theme: Theme) -> String {
        let pos = self.pos + offset;
        let mut result = String::new();
        result += &svg_rect(
            pos2(pos.x as f32 * scale, pos.y as f32 * scale),
            (self.width as f32 * scale, self.height as f32 * scale),
            STROKE_SCALE * scale,
            theme,
        );
        result += &"\n";
        for port in &self.ports {
            let center: Pos2 =
                (port.center(&self.pos, (self.width, self.height), &SVG_DUMMY_STATE)
                    + vec2(offset.x as f32, offset.y as f32))
                    * scale;
            result += &svg_circle_filled(center, 0.1 * scale, theme.get_stroke_color());
            result += &"\n";
        }
        for p in &self.ports {
            let cell = p.get_cell(&self.pos, (self.width, self.height)) + offset;
            let text_pos =
                pos2(cell.x as f32 * scale, cell.y as f32 * scale) + vec2(0.5, 0.5) * scale;
            result += &svg_single_line_text(
                p.name.clone(),
                text_pos,
                0.5 * scale,
                p.align.to_text_rotation(),
                theme,
                p.align.to_text_align2(),
            );
        }
        result
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Component {
    Unit(Unit),
    Primitive(PrimitiveComponent),
    TextField(TextField),
}

impl Component {
    pub fn get_position(&self) -> GridPos {
        match self {
            Component::Unit(u) => u.pos,
            Component::Primitive(g) => g.pos,
            Component::TextField(f) => f.pos,
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
            Component::TextField(f) => f.display(state, painter),
        }
    }

    pub fn get_connection_dock_cells(&self) -> Vec<GridPos> {
        match self {
            Component::Unit(unit) => unit
                .ports
                .iter()
                .enumerate()
                .map(|(_i, p)| p.get_dock_cell(&unit.pos, (unit.width, unit.height)))
                .collect(),
            Component::Primitive(g) => (0..g.typ.get_connections_number())
                .map(|i| g.get_connection_dock_cell(i).unwrap())
                .collect(),
            _ => vec![],
        }
    }

    pub fn set_pos(&mut self, pos: GridPos) {
        match self {
            Component::Unit(unit) => unit.pos = pos,
            Component::Primitive(g) => g.pos = pos,
            Component::TextField(f) => f.pos = pos,
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
            debounce_scale: 1.0,
            debounce: false,
        };
        self.display(&state, painter, theme);
    }

    pub fn get_dimension(&self) -> (i32, i32) {
        match self {
            Component::Unit(u) => (u.width, u.height),
            Component::Primitive(g) => g.get_dimension(),
            Component::TextField(f) => f.size,
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
            Self::Primitive(p) => p.get_actions(),
            Self::Unit(_u) => Unit::ACTIONS,
            Self::TextField(_f) => TextField::ACTIONS,
        }
    }

    pub fn highlight_connection(&self, connection_id: Id, state: &FieldState, painter: &Painter) {
        match self {
            Component::Unit(unit) => {
                if let Some(p) = unit.ports.get(connection_id) {
                    p.highlight(state, &unit.pos, (unit.width, unit.height), painter);
                }
            }
            Component::Primitive(g) => {
                g.highlight_connection(connection_id, state, painter);
            }
            _ => {}
        }
    }

    pub fn get_connection_position(&self, connection_id: Id, state: &FieldState) -> Option<Pos2> {
        match self {
            Component::Unit(unit) => {
                let p = unit.ports.get(connection_id)?;
                Some(p.center(&unit.pos, (unit.width, unit.height), state))
            }
            Component::Primitive(g) => g.get_connection_position(connection_id, state),
            _ => None,
        }
    }

    pub fn get_connection_dock_cell(&self, connection_id: Id) -> Option<GridPos> {
        match self {
            Component::Unit(unit) => {
                let p = unit.ports.get(connection_id)?;
                Some(p.get_dock_cell(&unit.pos, (unit.width, unit.height)))
            }
            Component::Primitive(g) => g.get_connection_dock_cell(connection_id),
            _ => None,
        }
    }

    pub fn is_connection_hovered(&self, connection_id: Id, state: &FieldState) -> bool {
        match self {
            Component::Unit(unit) => unit
                .ports
                .get(connection_id)
                .is_some_and(|p| p.is_hovered(state, &unit.pos, (unit.width, unit.height))),
            Component::Primitive(g) => g.is_connection_hovered(connection_id, state),
            _ => false,
        }
    }

    pub fn to_svg(&self, offset: GridPos, scale: f32, theme: Theme) -> String {
        match self {
            Component::Primitive(g) => g.get_svg(offset, scale, theme),
            Component::TextField(f) => f.get_svg(offset, scale, theme),
            Component::Unit(u) => u.to_svg(offset, scale, theme),
        }
    }

    /// Should I only check the overlap for this component?
    pub fn is_overlap_only(&self) -> bool {
        match self {
            Component::Primitive(g) => match g.typ {
                PrimitiveType::Point => true,
                _ => false,
            },
            Component::TextField(_f) => true,
            _ => false,
        }
    }

    /// Returns true, if component supports resizing
    pub fn is_resizable(&self) -> bool {
        match self {
            Component::TextField(_f) => true,
            Component::Unit(_u) => true,
            _ => false,
        }
    }

    /// Sets new size for component, if component supports resizing
    pub fn set_size(&mut self, size: (i32, i32)) {
        match self {
            Component::TextField(f) => f.size = size,
            Component::Unit(u) => u.resize(size),
            _ => {}
        }
    }

    pub fn is_single_line_text_edit(&self) -> bool {
        match self {
            Component::Unit(_u) => true,
            _ => false,
        }
    }

    /// Returns immutable reference to the text in a text edit field
    pub fn get_text_edit(&self, id: Id) -> Option<&String> {
        match self {
            Component::TextField(f) => {
                if id == 0 {
                    Some(&f.text)
                } else {
                    None
                }
            }
            Component::Unit(u) => Some(&u.ports.get(id)?.name),
            _ => None,
        }
    }

    /// Returns mutable reference to the text in a text edit field
    pub fn get_text_edit_mut(&mut self, id: Id) -> Option<&mut String> {
        match self {
            Component::TextField(f) => {
                if id == 0 {
                    Some(&mut f.text)
                } else {
                    None
                }
            }
            Component::Unit(u) => Some(&mut u.ports.get_mut(id)?.name),
            _ => None,
        }
    }
    /// Returns mutable reference to the text in a text edit field
    pub fn get_text_edit_rect(&self, id: Id, state: &FieldState) -> Option<Rect> {
        match self {
            Component::TextField(f) => {
                if id == 0 {
                    let (w, h) = f.size;
                    Some(Rect::from_min_size(
                        state.grid_to_screen(&f.pos),
                        state.grid_size * vec2(w as f32, h as f32),
                    ))
                } else {
                    None
                }
            }
            Component::Unit(u) => {
                let port = u.ports.get(id)?;
                let mut pos = state.grid_to_screen(&port.get_cell(&u.pos, (u.width, u.height)));
                let w = state.grid_size * u.width as f32 * 0.5;
                match port.align {
                    Rotation::ROT180 => pos -= vec2(w, 0.0),
                    _ => {}
                }
                return Some(Rect::from_min_size(pos, vec2(w, state.grid_size)));
            }
            _ => None,
        }
    }

    pub fn get_nearest_port_pos(
        &self,
        state: &FieldState,
        used: bool,
    ) -> Option<(Rotation, i32, Option<Id>)> {
        match self {
            Component::Unit(u) => u.get_nearest_port_pos(state, used),
            _ => None,
        }
    }

    pub fn add_port(&mut self, port: Port) {
        match self {
            Component::Unit(u) => u.ports.push(port),
            _ => panic!("Can't add port"),
        }
    }

    pub fn remove_port(&mut self, id: Id) -> Port {
        match self {
            Component::Unit(u) => u.ports.remove(id),
            _ => panic!("Can't remove port"),
        }
    }

    pub fn show_customization_panel(&mut self, ui : &mut egui::Ui, locale: &'static crate::locale::Locale) -> Option<Self> {
        match self {
            Self::Primitive(p) => {
                p.typ.show_customization_panel(ui, locale);
                return None;
            }
            _ => panic!()
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Port {
    // Connection
    pub offset: i32,
    pub align: Rotation,
    pub name: String,
}

impl Port {
    const PORT_SCALE: f32 = 0.1;

    pub fn center(
        &self,
        unit_pos: &GridPos,
        (width, height): (i32, i32),
        state: &FieldState,
    ) -> Pos2 {
        match self.align {
            Rotation::ROT0 => {
                state.grid_to_screen(&grid_pos(unit_pos.x, unit_pos.y + self.offset))
                    + vec2(0.0, 0.5 * state.grid_size)
            }
            Rotation::ROT90 => {
                state.grid_to_screen(&grid_pos(unit_pos.x + self.offset, unit_pos.y))
                    + vec2(0.5 * state.grid_size, 0.0)
            }
            Rotation::ROT180 => {
                state.grid_to_screen(&grid_pos(unit_pos.x + width, unit_pos.y + self.offset))
                    + vec2(0.0, 0.5 * state.grid_size)
            }
            Rotation::ROT270 => {
                state.grid_to_screen(&grid_pos(unit_pos.x + self.offset, unit_pos.y + height))
                    + vec2(0.5 * state.grid_size, 0.0)
            }
        }
    }

    pub fn get_dock_cell(&self, unit_pos: &GridPos, (width, height): (i32, i32)) -> GridPos {
        match self.align {
            Rotation::ROT0 => grid_pos(unit_pos.x - 1, unit_pos.y + self.offset),
            Rotation::ROT90 => grid_pos(unit_pos.x + self.offset, unit_pos.y - 1),
            Rotation::ROT180 => grid_pos(unit_pos.x + width, unit_pos.y + self.offset),
            Rotation::ROT270 => grid_pos(unit_pos.x + self.offset, unit_pos.y + height),
        }
    }

    fn get_cell(&self, unit_pos: &GridPos, (width, height): (i32, i32)) -> GridPos {
        match self.align {
            Rotation::ROT0 => grid_pos(unit_pos.x, unit_pos.y + self.offset),
            Rotation::ROT90 => grid_pos(unit_pos.x + self.offset, unit_pos.y),
            Rotation::ROT180 => grid_pos(unit_pos.x + width - 1, unit_pos.y + self.offset),
            Rotation::ROT270 => grid_pos(unit_pos.x + self.offset, unit_pos.y + height - 1),
        }
    }

    pub fn display(
        &self,
        unit_pos: &GridPos,
        dim: (i32, i32),
        state: &FieldState,
        painter: &Painter,
        theme: Theme,
    ) {
        let stroke_color = theme.get_stroke_color();
        let pos = self.center(unit_pos, dim, state);
        painter.circle_filled(pos, state.grid_size * Self::PORT_SCALE, stroke_color);
        if state.lod_level() == LodLevel::Max {
            let text_pos: Pos2 = state.grid_to_screen(&self.get_cell(unit_pos, dim))
                + vec2(0.5, 0.5) * state.grid_size;
            show_text_with_debounce(
                text_pos,
                self.name.clone(),
                state,
                painter,
                None,
                self.align.to_text_rotation(),
                self.align.to_text_align2(),
            );
        }
    }

    pub fn is_hovered(&self, state: &FieldState, unit_pos: &GridPos, dim: (i32, i32)) -> bool {
        if let Some(cursor_pos) = state.cursor_pos {
            let d = self.center(unit_pos, dim, state).distance(cursor_pos);
            d <= state.grid_size * Self::PORT_SCALE * 2.0
        } else {
            false
        }
    }

    pub fn highlight(
        &self,
        state: &FieldState,
        unit_pos: &GridPos,
        dim: (i32, i32),
        painter: &Painter,
    ) {
        let p = self.center(unit_pos, dim, state);
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
    AddPort,
    RemovePort,
    EditPort,
    EditText,
    Customize,
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

    pub fn draw_connection_icon(center: Pos2, radius: f32, painter: &Painter, stroke: Stroke) {
        let num_segments = 30;
        let mut points = Vec::with_capacity(num_segments + 1);
        let start_angle = PI * 0.25;
        let sweep_angle = 1.5 * PI;
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
        painter.line_segment(
            [
                center - vec2(radius, 0.0),
                center - vec2(radius * 1.75, 0.0),
            ],
            stroke,
        );
        painter.line_segment([center, center + vec2(radius * 1.0, 0.0)], stroke);
        painter.circle_filled(center, stroke.width, stroke.color);
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
            Self::AddPort => {
                painter.text(
                    rect.min + vec2(rect.height() * 0.05, rect.height() * 0.05),
                    Align2::LEFT_TOP,
                    "+",
                    FontId::monospace(rect.height() * 0.5),
                    stroke.color,
                );
                let stroke2 = Stroke {
                    color: stroke.color,
                    width: stroke.width * 0.75,
                };
                Self::draw_connection_icon(
                    rect.center() + vec2(rect.height() * 0.1, rect.height() * 0.1),
                    rect.height() * 0.3 * 0.75,
                    painter,
                    stroke2,
                );
            }
            Self::RemovePort => {
                painter.text(
                    rect.min + vec2(rect.height() * 0.05, rect.height() * 0.05),
                    Align2::LEFT_TOP,
                    "×",
                    FontId::monospace(rect.height() * 0.5),
                    stroke.color,
                );
                let stroke2 = Stroke {
                    color: stroke.color,
                    width: stroke.width * 0.75,
                };
                Self::draw_connection_icon(
                    rect.center() + vec2(rect.height() * 0.1, rect.height() * 0.1),
                    rect.height() * 0.3 * 0.75,
                    painter,
                    stroke2,
                );
            }
            Self::EditPort => {
                painter.text(
                    rect.min + vec2(rect.height() * 0.05, rect.height() * 0.05),
                    Align2::LEFT_TOP,
                    "📝",
                    FontId::monospace(rect.height() * 0.5),
                    stroke.color,
                );
                let stroke2 = Stroke {
                    color: stroke.color,
                    width: stroke.width * 0.75,
                };
                Self::draw_connection_icon(
                    rect.center() + vec2(rect.height() * 0.1, rect.height() * 0.1),
                    rect.height() * 0.3 * 0.75,
                    painter,
                    stroke2,
                );
            }
            Self::EditText => {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "📝",
                    FontId::monospace(rect.height()),
                    stroke.color,
                );
            }
            Self::Customize => {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "⚙",
                    FontId::monospace(rect.height()),
                    stroke.color,
                );
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
pub enum RotationDirection {
    Up,
    Down,
}
