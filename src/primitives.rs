use std::{collections::HashMap, f32::consts::FRAC_PI_2, i32, ops::Add, usize};

use eframe::egui_glow::painter;
use egui::{emath::align, epaint::{TextShape, Vertex}, scroll_area::State, vec2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use rstar::{RTree, RTreeObject, AABB};

use crate::field::{self, Field, FieldState};  // AABB = Axis-Aligned Bounding Box (прямоугольник)
type Point = [i32; 2];  // Точка (x, y)

pub type Id = usize;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct GridPos {
    pub x : i32,
    pub y : i32
}

impl GridPos {
    fn to_point(&self) -> [i32; 2] {
        return [self.x, self.y];
    }
}

impl Add for GridPos {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        grid_pos(self.x + rhs.x, self.y + rhs.y)
    }
}

pub fn grid_pos(x:i32, y:i32) -> GridPos {
    GridPos {x,y}
}

#[derive(Clone)]
pub enum ConnectionAlign {LEFT, RIGHT, TOP, BOTTOM} // TODO: add custom

impl ConnectionAlign {
    fn grid_offset(&self) -> Vec2 {
        match self {
            Self::LEFT      =>  {vec2(0.0, 0.5)}
            Self::RIGHT     =>  {vec2(1.0, 0.5)}
            Self::TOP       =>  {vec2(0.5, 0.0)}
            Self::BOTTOM    =>  {vec2(0.5, 1.0)}
        }
    }

    fn rotation_angle(&self) -> f32 {
        match self {
            Self::LEFT      =>  {0.0}
            Self::RIGHT     =>  {0.0}
            Self::TOP       =>  {FRAC_PI_2}
            Self::BOTTOM    =>  {-FRAC_PI_2}
        }
    }
}


#[derive(Clone)]
pub struct Port { // Connection
    pub inner_cell: GridPos,
    pub align: ConnectionAlign,
    pub name: String
}

pub struct GridRect {
    pub id : usize,
    pub min : GridPos,
    pub max : GridPos
}

impl PartialEq for GridRect {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
}

pub fn grid_rect(id:usize, min: GridPos, max: GridPos) -> GridRect {
    return GridRect {
        id,
        min,
        max
    };
}

#[derive(Clone)]
pub struct Unit {
    pub id: Id, // TODO: remove it, return new id on adding to BD??
    pub name: String,
    pub pos: GridPos,
    pub width: i32,
    pub height: i32,
    pub ports: Vec<Port>
}

pub struct Net {
    pub id: Id, // TODO: remove it, return new id on adding to BD
    pub start_point: GridBDConnectionPoint,
    pub end_point: GridBDConnectionPoint,
    pub points: Vec<GridPos>
}

impl Net {
    fn get_grid_rect(&self, id: Id) -> GridRect { // TODO: remove it
        let mut x_min:i32 = i32::MAX;
        let mut y_min:i32 = i32::MAX;
        let mut x_max:i32 = i32::MIN;
        let mut y_max:i32 = i32::MIN;
        for p in &self.points {
            if p.x < x_min {
                x_min = p.x;
            }
            if p.y < y_min {
                y_min = p.y
            }
            if p.x > x_max {
                x_max = p.x;
            }
            if p.y > y_max {
                y_max = p.y
            }
        }
        return grid_rect(id, grid_pos(x_min, y_min), grid_pos(x_max, y_max));
    }

    fn port_align_to_vec2(&self, state: &FieldState,  align:&ConnectionAlign) -> Vec2 {
        match align {
            ConnectionAlign::LEFT => {vec2(0.5 * state.grid_size, 0.0)},
            ConnectionAlign::RIGHT => {vec2(-0.5 * state.grid_size, 0.0)},
            ConnectionAlign::TOP => {vec2(0.0, -0.5 * state.grid_size)},
            ConnectionAlign::BOTTOM => {vec2(0.0, 0.5 * state.grid_size)},
        }
    }

    fn get_segments(&self) -> Vec<NetSegment> { // TODO: return iterator?
        let mut result = vec![];
        for i in 0..self.points.len()-1 {
            result.push(NetSegment::new(i, self.id, self.points[i], self.points[i+1],
            (i == 0).then_some(self.start_point),
            (i == self.points.len()-2).then_some(self.end_point)));
        }
        result
    }
}

impl RTreeObject for GridRect {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(self.min.to_point(), self.max.to_point())
    }
}


pub struct GridBD {
    components: HashMap<usize, Component>,
    tree : RTree<GridRect>,

    // TODO: use it
    connections: HashMap<GridPos, ComponentConnection>,
    pub nets: HashMap<usize, Net>,
    net_tree : RTree<NetSegment>
}

impl GridBD {
    pub fn new() -> GridBD {
        Self {
            components: HashMap::new(),
            tree: RTree::new(),
            connections: HashMap::new(),
            nets: HashMap::new(),
            net_tree: RTree::new()
        }
    }

    pub fn add_component(&mut self, component: Component) {
        let rect: GridRect = component.get_grid_rect();
        component.get_connection_cells().iter().for_each(|c| {self.connections.insert(c.cell, c.to_comp_connection(component.get_id()));});
        self.components.insert(rect.id, component);
        self.tree.insert(rect);
    }

    pub fn add_component_with_unknown_id(&mut self, component: Component) {
        let mut component = component;
        component.set_id(self.components.len());
        self.add_component(component);
    }

    pub fn remove_component(&mut self, id:usize) {
        if let Some(component) = self.components.get(&id) {
            component.get_connection_cells().iter().for_each(|c| {
                if let Some(connection) = self.connections.get(&c.cell) {
                    if connection.component_id == component.get_id() {
                        self.connections.remove(&c.cell);
                    }
                }
                self.connections.insert(c.cell, c.to_comp_connection(component.get_id()));
            });
            self.tree.remove(&component.get_grid_rect());
            self.components.remove(&id);
        }
    }

    pub fn get_hovered_connection(&self, state: &FieldState) -> Option<GridBDConnectionPoint> {
        if let Some(cursor_pos) = state.cursor_pos {
            let grid_hoverpos = state.screen_to_grid(cursor_pos);
            // TODO: Simplify it (HOW??)
            for i in 0..3 {
                for j in 0..3 {
                    if let Some(connection) = self.connections.get(&grid_pos(grid_hoverpos.x + i - 1, grid_hoverpos.y + j - 1)) {
                        if let Some(component) = self.components.get(&connection.component_id) {
                            if let Some(con) = component.get_connection(connection.inner_id) {
                                if con.is_hovered(state, &component.get_grid_rect().min) {
                                    return Some(GridBDConnectionPoint { component_id: connection.component_id, connection_id: connection.inner_id });
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_visible_components(&self, rect: &GridRect) -> Vec<&Component> {
        self.tree.locate_in_envelope_intersecting(&rect.envelope()).map(|rect| {self.components.get(&rect.id).unwrap()}).collect()
    }

    pub fn get_component(&self, id: &Id) -> Option<&Component> {
        return self.components.get(&id);
    }

    pub fn find_net_path(&self, pos1:GridPos, pos2:GridPos) -> Vec<GridPos> {
        return vec![grid_pos((pos1.x + pos2.x)/2, pos1.y), grid_pos((pos1.x + pos2.x)/2, pos2.y)]
    }

    pub fn add_net(&mut self, net: Net) {
        for segment in net.get_segments() {
            self.net_tree.insert(segment);
        }
        self.nets.insert(net.id, net);
    }

    pub fn get_visible_nets(&self, rect: &GridRect) -> Vec<&Net> {
        // fixme
        return self.nets.values().collect();
    }

    pub fn get_component_and_connection(&self, cp: &GridBDConnectionPoint) -> Option<(&Component, Connection)> {
        if let Some(comp) = self.components.get(&cp.component_id) {
            if let Some(con) = comp.get_connection(cp.connection_id) {
                return Some((comp, con));
            }
        }
        None
    }

    pub fn get_visible_net_segments(&self, rect: &GridRect) -> Vec<&NetSegment> {
        self.net_tree.locate_in_envelope_intersecting(&rect.envelope()).collect()
    }
}


#[derive(Clone)]
pub enum Component {
    Unit(Unit)
}

struct ComponentConnection {
    component_id: Id,
    inner_id: Id
}

struct ConnectionCell {
    cell: GridPos,
    inner_id: Id
}

impl ConnectionCell {
    fn to_comp_connection(&self, component_id:usize) -> ComponentConnection {
        ComponentConnection {
            component_id: component_id,
            inner_id : self.inner_id
        }
    }
}

impl Component {
    pub fn get_position(&self) -> GridPos {
        match self {
            Component::Unit(u) => u.pos,
        }
    }

    pub fn get_id(&self) -> usize {
        match self {
            Component::Unit(u) => u.id,
        }
    }

    pub fn  get_grid_rect(&self) -> GridRect {
        match self {
            Component::Unit(u) => u.get_grid_rect(),
        }
    }

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        match self {
            Component::Unit(u) => u.display(state, painter),
        }
    }

    fn get_connection_cells(&self) -> Vec<ConnectionCell> {
        match self {
            Component::Unit(unit) => unit.ports.iter().enumerate().map(|(i, p)| {
                ConnectionCell{cell: grid_pos(unit.pos.x + p.inner_cell.x, unit.pos.y + p.inner_cell.y), inner_id:i}
            }).collect(),
        }
    }

    pub fn get_connection(&self, inner_id: Id) -> Option<Connection> {
        match self {
            Component::Unit(unit) => if let Some(p) = unit.ports.get(inner_id) {Some(Connection::Port(p))} else {None},
            _ => None
        }
    }

    pub fn draw_preview(&self, rect: &Rect, painter: &Painter) {
        let grid_rect = self.get_grid_rect();
        let w = grid_rect.max.x - grid_rect.min.x + 2;
        let h = grid_rect.max.y - grid_rect.min.y + 2;
        let x_grid_size = rect.width() / w as f32;
        let y_grid_size = rect.height() / h as f32;
        let grid_size = x_grid_size.min(y_grid_size);
        let scale = grid_size / Field::BASE_GRID_SIZE;
        let state = FieldState {
            scale: grid_size / Field::BASE_GRID_SIZE,
            offset: Vec2::default(),
            grid_size: grid_size,
            rect: rect.clone(), // ?? TODO make it as Option
            label_font: FontId::monospace((Field::BASE_GRID_SIZE * scale * 0.5).min(Field::MAX_FONT_SIZE)),
            label_visible: true,
            cursor_pos: None,
        };
        self.display(&state, painter);
    }

    pub fn get_dimension(&self) -> (i32, i32) {
        let grid_rect = self.get_grid_rect();
        let w = grid_rect.max.x - grid_rect.min.x;
        let h = grid_rect.max.y - grid_rect.min.y;
        return (w, h);
    }

    pub fn set_id(&mut self, id:Id) {
        match self {
            Component::Unit(unit) => unit.id = id
        }
    }

    pub fn set_pos(&mut self, pos: GridPos) {
        match self {
            Component::Unit(unit) => unit.pos = pos
        }
    }
}

impl Unit {
    fn get_grid_rect(&self) -> GridRect {
        grid_rect(self.id, self.pos, grid_pos(self.pos.x + self.width, self.pos.y + self.height))
    }

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        // TODO: Add LOD level
        // 1. Display unit with ports and labels
        // 2. Display display only Rectangle
        let rect = Rect::from_min_size(state.grid_to_screen(&self.pos), vec2(state.grid_size * self.width as f32, state.grid_size * self.height as f32));
        painter.rect_filled(rect, 0.5 * state.scale, Color32::GRAY);

        if state.scale > Field::LED_LEVEL0_SCALE {
            painter.rect_stroke(rect, 0.5 * state.scale, Stroke::new(1.0 * state.scale, Color32::DARK_GRAY),StrokeKind::Outside);
            for port in &self.ports {
                port.display(&self.pos, state, &painter);
            }
        }
    }
}


impl Port {
    const PORT_SCALE:f32 = 0.1;

    pub fn center(&self, unit_pos:&GridPos, state: &FieldState) -> Pos2 {
        state.grid_to_screen(&GridPos { x: unit_pos.x + self.inner_cell.x, y: unit_pos.y + self.inner_cell.y }) + self.align.grid_offset() * state.grid_size
    }

    pub fn display(&self, unit_pos:&GridPos, state: &FieldState, painter:&Painter) {
        let angle= self.align.rotation_angle();
        let pos = self.center(unit_pos, state);
        painter.circle_filled(pos, state.grid_size * Self::PORT_SCALE, Color32::GRAY);
        painter.circle_stroke(pos, state.grid_size * Self::PORT_SCALE, Stroke::new(1.0 * state.scale, Color32::DARK_GRAY));
        if state.label_visible {
            let galley = painter.fonts(|fonts| {
                fonts.layout_no_wrap(
                    self.name.clone(),
                    state.label_font.clone(),
                    Color32::WHITE,
                )
            });
            let label_rect = galley.rect;

            let mut text_pos = state.grid_to_screen(&GridPos { x: unit_pos.x + self.inner_cell.x, y: unit_pos.y + self.inner_cell.y });
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
            painter.add(
                TextShape::new(text_pos, galley, Color32::WHITE).with_angle(angle)
            );
        }
    }

    pub fn is_hovered(&self, state: &FieldState, unit_pos: &GridPos) -> bool {
        if let Some(cursor_pos) = state.cursor_pos {
            let d = self.center(unit_pos, state).distance(cursor_pos);
            // println!("center:{}, cursor:{}, {}, {}", self.center(unit_pos, state), cursor_pos, self.name, d);
            d <= state.grid_size * Self::PORT_SCALE * 2.0
        }
        else {
            false
        }
    }

    pub fn highlight(&self, state: &FieldState, unit_pos: &GridPos, painter:&Painter) {
        let p = self.center(unit_pos, state);
        painter.circle_filled(p, state.grid_size * Self::PORT_SCALE * 3.0, Color32::from_rgba_unmultiplied(100, 100, 0, 100));
    }
}


pub enum Connection<'a> {
    Port(&'a Port)
}

impl<'a> Connection <'a> {
    pub fn is_hovered(&self, state: &FieldState, component_pos: &GridPos) -> bool {
        match self {
            Self::Port(p) => p.is_hovered(state, component_pos)
        }
    }

    pub fn highlight(&self, state: &FieldState, comp_pos: &GridPos, painter:&Painter) {
        match self {
            Self::Port(p) => p.highlight(state, comp_pos, painter)
        }
    }

    pub fn center(&self, comp_pos:&GridPos, state: &FieldState) -> Pos2 {
        match self {
            Self::Port(p) => p.center(comp_pos, state)
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

    pub fn get_pos(&self, owner :&Component) -> GridPos {
        match self {
            Connection::Port(p) => p.inner_cell + owner.get_position(),
        }
    }
}


#[derive(Clone, Copy)]
pub struct GridBDConnectionPoint {
    pub component_id:Id,
    pub connection_id:Id
}

pub struct NetSegment {
    inner_id: Id,  // ID of segment in net
    net_id: Id,    // ID of net
    pos1: GridPos,
    pos2: GridPos,
    con1: Option<GridBDConnectionPoint>,  // if segment
    con2: Option<GridBDConnectionPoint>,  // Second position
}

impl NetSegment {
    pub fn new(inner_id: Id, net_id: Id, pos1: GridPos, pos2: GridPos,
               con1: Option<GridBDConnectionPoint>, con2: Option<GridBDConnectionPoint>) -> Self {
        Self {
            inner_id,
            net_id,
            pos1,
            pos2,
            con1,
            con2
        }
    }

    fn is_horizontal(&self) -> bool {
        self.pos1.y == self.pos2.y
    }

    pub fn get_mesh(&self, bd: &GridBD, state: &FieldState) -> Mesh {
        let w = (state.grid_size * 0.1).max(0.5);
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
            mesh.vertices.push(Vertex { pos: p1, uv: Pos2::ZERO, color });
            mesh.vertices.push(Vertex { pos: p2, uv: Pos2::ZERO, color });
            mesh.vertices.push(Vertex { pos: p3, uv: Pos2::ZERO, color });
            mesh.vertices.push(Vertex { pos: p4, uv: Pos2::ZERO, color });

            // два треугольника на сегмент
            mesh.indices.extend_from_slice(&[
                idx_base, idx_base + 1, idx_base + 2,
                idx_base + 2, idx_base + 1, idx_base + 3,
            ]);
        }

        mesh
    }
}

impl RTreeObject for NetSegment {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(self.pos1.to_point(), self.pos2.to_point())
    }
}

impl PartialEq for NetSegment {
    fn eq(&self, other: &Self) -> bool {
        other.inner_id == self.inner_id && self.net_id == other.net_id
    }
}
