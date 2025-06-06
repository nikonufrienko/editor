use std::{collections::HashMap, f32::consts::FRAC_PI_2, i32, mem::offset_of, usize};

use egui::{ahash::{HashMap, HashMapExt}, emath::Float, epaint::TextShape, vec2, Color32, Galley, Painter, Rect, Stroke, StrokeKind, Vec2};
use rstar::{RTree, RTreeObject, AABB};

use crate::field::Field;  // AABB = Axis-Aligned Bounding Box (прямоугольник)
type Point = [i32; 2];  // Точка (x, y)

#[derive(Clone, Copy)]
pub struct GridPos {
    pub x : i32,
    pub y : i32
}

impl GridPos {
    fn to_point(&self) -> [i32; 2] {
        return [self.x, self.y];
    }
}

pub fn grid_pos(x:i32, y:i32) -> GridPos {
    GridPos {x,y}
}

pub enum PortAlign {LEFT, RIGHT, TOP, BOTTOM}

impl PortAlign {
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

pub struct Port {
    pub inner_cell: GridPos,
    pub align: PortAlign,
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
    pub id: usize,
    pub name: String,
    pub pos: GridPos,
    pub width: i32,
    pub height: i32,
    pub ports: Vec<Port>
}

pub struct Net {
    pub aligns: (PortAlign, PortAlign),
    pub points: Vec<GridPos>
}

impl Net {
    fn get_grid_rect(&self, id: usize) -> GridRect {
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
}

impl RTreeObject for GridRect {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(self.min.to_point(), self.max.to_point())
    }
}


pub struct GridDB {
    components: HashMap<usize, Component>,
    tree : RTree<GridRect>,

    // TODO: use it
    connections: HashMap<usize, Port>,
    nets: HashMap<usize, Port>
}

impl GridDB {
    pub fn new() -> GridDB {
        Self {
            components: HashMap::new(),
            tree: RTree::new(),
            connections: HashMap::new(),
            nets: HashMap::new()
        }
    }

    pub fn add_component(&mut self, component: Component) {
        let rect: GridRect = component.get_grid_rect();
        self.components.insert(rect.id, component);
        self.tree.insert(rect);
    }

    pub fn remove_component(&mut self, id:usize) {
        if let Some(unit) = self.components.get(&id) {
            self.tree.remove(&unit.get_grid_rect());
            self.components.remove(&id);
        }
    }

    pub fn get_visible_components(&self, rect:GridRect) -> Vec<&Component> {
        self.tree.locate_in_envelope_intersecting(&rect.envelope()).map(|rect| {self.components.get(&rect.id).unwrap()}).collect()
    }
}


pub enum Component {
    Unit(Unit),
    Net(Net)
}

impl Component {
    pub fn  get_grid_rect(&self) -> GridRect {
        match self {
            Component::Unit(u) => {u.get_grid_rect()}
            _ => {GridRect{id: usize::MAX, min: grid_pos(0, 0), max: grid_pos(0, 0)}}
        }
    }
    pub fn display(&self, field: &Field, ui:&mut egui::Ui) {
        match self {
            Component::Unit(u) => {u.display(field, ui)}
            _ => {}
        }
    }
}

impl Unit {
    fn get_grid_rect(&self) -> GridRect {
        grid_rect(self.id, self.pos, grid_pos(self.pos.x + self.width, self.pos.y + self.height))
    }

    pub fn display(&self, field: &Field, ui:&mut egui::Ui) {
        // TODO: Add LOD level
        // 1. Display unit with ports and labels
        // 2. Display display only Rectangle
        let rect = Rect::from_min_size(field.grid_to_screen(self.pos), vec2(field.grid_size * self.width as f32, field.grid_size * self.height as f32));
        let painter = ui.painter().with_clip_rect(field.rect);
        painter.rect_filled(rect, 0.5 * field.scale, Color32::GRAY);

        if field.scale > Field::LED_LEVEL0_SCALE {
            painter.rect_stroke(rect, 0.5 * field.scale, Stroke::new(1.0 * field.scale, Color32::DARK_GRAY),StrokeKind::Outside);
            for port in &self.ports {
                port.display(self.pos, field, &painter);
            }
        }
    }
}

impl Port {
    pub fn display(&self, unit_pos:GridPos, field: &Field, painter:&Painter) {
        let mut pos = field.grid_to_screen(GridPos { x: unit_pos.x + self.inner_cell.x, y: unit_pos.y + self.inner_cell.y });
        let mut text_pos = pos.clone();
        let angle= self.align.rotation_angle();
        pos += self.align.grid_offset() * field.grid_size;
        painter.circle_filled(pos, field.grid_size/6.0, Color32::GRAY);
        painter.circle_stroke(pos, field.grid_size/6.0, Stroke::new(1.0 * field.scale, Color32::DARK_GRAY));
        if field.label_visible {
            let galley = painter.fonts(|fonts| {
                fonts.layout_no_wrap(
                    self.name.clone(),
                    field.label_font.clone(),
                    Color32::WHITE,
                )
            });
            let label_rect = galley.rect;

            match self.align {
                PortAlign::LEFT => {
                    text_pos.y += field.grid_size / 2.0 - label_rect.height() / 2.0;
                    text_pos.x += field.grid_size * 0.5;
                    // TODO:
                }
                PortAlign::RIGHT => {
                    text_pos.y += field.grid_size / 2.0 - label_rect.height() / 2.0;
                    text_pos.x -= label_rect.width() - field.grid_size * 0.5;

                }
                PortAlign::TOP => {
                    text_pos.x += (field.grid_size + label_rect.width() / 2.0) / 2.0;
                    text_pos.y += field.grid_size * 0.5;

                }
                PortAlign::BOTTOM => {}
            }
            painter.add(
                TextShape::new(text_pos, galley, Color32::WHITE).with_angle(angle)
            );
        }
    }
}

