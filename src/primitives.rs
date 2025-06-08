use std::{collections::HashMap, f32::consts::FRAC_PI_2, i32, mem::offset_of, ops::Add, usize};

use egui::{emath::Float, epaint::TextShape, scroll_area::State, vec2, Color32, Galley, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};
use rstar::{RTree, RTreeObject, AABB};

use crate::field::{Field, FieldState};  // AABB = Axis-Aligned Bounding Box (прямоугольник)
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

    pub fn display(&self, bd: &GridBD, state: &FieldState, painter: &Painter) {
        let grid_center_vec = vec2(0.5 * state.grid_size, 0.5 * state.grid_size);
        let points: Vec<Pos2> = self.points.iter().map(|p| {state.grid_to_screen(&p) + grid_center_vec}).collect();
        let len = points.len();
        if len > 1 {
            bd.get_component(&self.start_point.component_id).inspect(|start_comp| {
                start_comp.get_connection(self.start_point.connection_id).inspect(|start_con|{
                    bd.get_component(&self.end_point.component_id).inspect(|end_comp| {
                        start_comp.get_connection(self.end_point.connection_id).inspect(|end_con|{
                            let start_point = start_con.center(&start_comp.get_position(), state);
                            let end_point = end_con.center(&end_comp.get_position(), state);
                            let mut extended = vec![start_point];
                            extended.extend(points);
                            extended.push(end_point);
                            painter.with_clip_rect(state.rect).line(extended, Stroke::new(state.grid_size * 0.1, Color32::DARK_GRAY));
                        });
                    });
                });
            });
        }
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
    pub nets: HashMap<usize, Net>
}

impl GridBD {
    pub fn new() -> GridBD {
        Self {
            components: HashMap::new(),
            tree: RTree::new(),
            connections: HashMap::new(),
            nets: HashMap::new()
        }
    }

    pub fn add_component(&mut self, component: Component) {
        let rect: GridRect = component.get_grid_rect();
        component.get_connection_cells().iter().for_each(|c| {self.connections.insert(c.cell, c.to_comp_connection(component.get_id()));});
        self.components.insert(rect.id, component);
        self.tree.insert(rect);
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
        return vec![pos1, grid_pos((pos1.x + pos2.x)/2, pos1.y), grid_pos((pos1.x + pos2.x)/2, pos2.y), pos2]
    }

    pub fn add_net(&mut self, net: Net) {
        self.nets.insert(net.id, net);
    }

    pub fn get_visible_nets(&self, rect: &GridRect) -> Vec<&Net> {
        // fixme
        return self.nets.values().collect();
    }
}


pub enum Component {
    Unit(Unit),
    Net(Net)
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
            _ => panic!("TODO"),
        }
    }

    pub fn get_id(&self) -> usize {
        match self {
            Component::Unit(u) => u.id,
            _ => usize::MAX,
        }
    }

    pub fn  get_grid_rect(&self) -> GridRect {
        match self {
            Component::Unit(u) => {u.get_grid_rect()}
            _ => {GridRect{id: usize::MAX, min: grid_pos(0, 0), max: grid_pos(0, 0)}}
        }
    }

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        match self {
            Component::Unit(u) => {u.display(state, painter)}
            _ => {}
        }
    }

    pub fn get_connection_cells(&self) -> Vec<ConnectionCell> {
        match self {
            Component::Unit(unit) => unit.ports.iter().enumerate().map(|(i, p)| {
                ConnectionCell{cell: grid_pos(unit.pos.x + p.inner_cell.x, unit.pos.y + p.inner_cell.y), inner_id:i}
            }).collect(),
            _ => vec![]
        }
    }

    pub fn get_connection(&self, inner_id: Id) -> Option<Connection> {
        match self {
            Component::Unit(unit) => if let Some(p) = unit.ports.get(inner_id) {Some(Connection::Port(p))} else {None},
            _ => None
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


#[derive(Clone)]
pub struct GridBDConnectionPoint {
    pub component_id:Id,
    pub connection_id:Id
}

