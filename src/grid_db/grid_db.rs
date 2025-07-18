use std::{
    collections::{HashMap, HashSet},
    i32, usize,
};

use egui::{Theme};
use rstar::{AABB, PointDistance, RTree, RTreeObject};
use serde::{Deserialize, Serialize};

use crate::{
    field::FieldState,
    grid_db::{Component, ComponentColor, GridPos, Net, NetSegment, grid_pos},
}; // AABB = Axis-Aligned Bounding Box (прямоугольник)
type Point = [i32; 2]; // Точка (x, y)

pub type Id = usize;

pub struct GridRect {
    pub id: usize,
    pub min: GridPos,
    pub max: GridPos,
}

impl GridRect {
    fn contains(&self, pos: GridPos) -> bool {
        pos.x >= self.min.x && pos.y >= self.min.y && pos.x <= self.max.x && pos.y <= self.max.y
    }
}

impl PartialEq for GridRect {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
}

impl RTreeObject for GridRect {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(self.min.to_point(), self.max.to_point())
    }
}

impl PointDistance for GridRect {
    fn distance_2(&self, point: &Point) -> i32 {
        let x = point[0];
        let y = point[1];

        let dx = if x < self.min.x {
            self.min.x - x
        } else if x > self.max.x {
            x - self.max.x
        } else {
            0
        };

        let dy = if y < self.min.y {
            self.min.y - y
        } else if y > self.max.y {
            y - self.max.y
        } else {
            0
        };

        dx * dx + dy * dy
    }
}

pub fn grid_rect(id: usize, min: GridPos, max: GridPos) -> GridRect {
    return GridRect { id, min, max };
}

#[derive(Default)]
pub struct GridBD {
    components: HashMap<usize, Component>,
    tree: RTree<GridRect>,
    connections: HashMap<GridPos, HashSet<GridBDConnectionPoint>>, // HashSet<GridBDConnectionPoint> --> Vec<GridBDConnectionPoint> ???
    pub nets: HashMap<usize, Net>,
    connected_nets: HashMap<GridBDConnectionPoint, HashSet<Id>>,
    net_tree: RTree<NetSegment>,
    next_component_id: Id,
    next_net_id: Id,
}

impl GridBD {
    pub fn new() -> GridBD {
        Self {
            components: HashMap::new(),
            tree: RTree::new(),
            connections: HashMap::new(),
            nets: HashMap::new(),
            net_tree: RTree::new(),
            connected_nets: HashMap::new(),
            next_component_id: 0,
            next_net_id: 0,
        }
    }

    pub fn insert_component(&mut self, id: Id, component: Component) {
        let rect: GridRect = component.get_grid_rect(id);
        component
            .get_connection_dock_cells()
            .iter()
            .enumerate()
            .for_each(|(i, cell)| {
                if let Some(set) = self.connections.get_mut(cell) {
                    set.insert(GridBDConnectionPoint {
                        component_id: id,
                        connection_id: i,
                    });
                } else {
                    self.connections.insert(
                        *cell,
                        HashSet::from([GridBDConnectionPoint {
                            component_id: id,
                            connection_id: i,
                        }]),
                    );
                }
            });
        self.components.insert(rect.id, component);
        self.tree.insert(rect);
    }

    pub fn push_component(&mut self, component: Component) {
        self.insert_component(self.next_component_id, component);
        self.next_component_id += 1;
    }

    pub fn remove_component(&mut self, id: &Id) -> Option<Component> {
        let component = self.components.get(&id)?;
        for cell in component.get_connection_dock_cells() {
            if let Some(connections_set) = self.connections.get_mut(&cell) {
                if let Some(connection) = connections_set
                    .iter()
                    .find(|it| it.component_id == *id)
                    .cloned()
                {
                    connections_set.remove(&connection);
                    if connections_set.is_empty() {
                        self.connections.remove(&cell);
                    }
                }
            }
            //self.connections.insert(*cell, GridBDConnectionPoint {component_id: *id, connection_id: i}); // ?????????
        }
        self.tree.remove(&component.get_grid_rect(*id));

        return self.components.remove(&id);
    }

    pub fn get_hovered_connection(&self, state: &FieldState) -> Option<GridBDConnectionPoint> {
        if let Some(cursor_pos) = state.cursor_pos {
            let grid_hoverpos = state.screen_to_grid(cursor_pos);
            // TODO: Simplify it (HOW??)
            for i in 0..3 {
                for j in 0..3 {
                    if let Some(connections) = self
                        .connections
                        .get(&grid_pos(grid_hoverpos.x + i - 1, grid_hoverpos.y + j - 1))
                    {
                        for connection in connections {
                            if let Some(component) = self.components.get(&connection.component_id) {
                                if component.is_connection_hovered(connection.connection_id, state)
                                {
                                    return Some(connection.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_hovered_component_id(&self, state: &FieldState) -> Option<&Id> {
        let cell = state.screen_to_grid(state.cursor_pos?);
        if let Some(rect) = self
            .tree
            .locate_in_envelope_intersecting(&cell.to_point().envelope())
            .next()
        {
            return Some(&rect.id);
        }
        return None;
    }

    pub fn get_visible_components(&self, rect: &GridRect) -> Vec<&Component> {
        self.tree
            .locate_in_envelope_intersecting(&rect.envelope())
            .map(|rect| self.components.get(&rect.id).unwrap())
            .collect()
    }

    pub fn get_component(&self, id: &Id) -> Option<&Component> {
        return self.components.get(&id);
    }

    pub fn get_component_mut(&mut self, id: &Id) -> Option<&mut Component> {
        return self.components.get_mut(&id);
    }

    pub fn find_net_path(&self, pos1: GridPos, pos2: GridPos) -> Vec<GridPos> {
        return vec![
            grid_pos((pos1.x + pos2.x) / 2, pos1.y),
            grid_pos((pos1.x + pos2.x) / 2, pos2.y),
        ];
    }

    pub fn add_net(&mut self, net: Net) {
        let net_id = self.next_net_id;
        self.next_net_id += 1;
        for segment in net.get_segments(net_id) {
            self.net_tree.insert(segment);
        }
        for p in [net.start_point, net.end_point] {
            if let Some(nets) = self.connected_nets.get_mut(&p) {
                nets.insert(net_id);
            } else {
                let mut set = HashSet::new();
                set.insert(net_id);
                self.connected_nets.insert(p, set);
            }
        }
        self.nets.insert(net_id, net);
    }

    pub fn remove_net(&mut self, id: &Id) -> Option<Net> {
        if let Some(net) = self.nets.get(id) {
            for segment in net.get_segments(*id) {
                self.net_tree.remove(&segment);
            }
            for p in [net.start_point, net.end_point] {
                if let Some(nets) = self.connected_nets.get_mut(&p) {
                    nets.remove(id);
                }
            }
            return self.nets.remove(id);
        }
        None
    }

    pub fn remove_component_with_connected_nets(&mut self, component_id: &Id) {
        for net_id in self.get_connected_nets(component_id) {
            self.remove_net(&net_id);
        }
        self.remove_component(component_id);
    }

    pub fn get_hovered_segment(&self, state: &FieldState) -> Option<&NetSegment> {
        let cell = state.screen_to_grid(state.cursor_pos?);
        let segments = self
            .net_tree
            .locate_in_envelope_intersecting(&cell.to_point().envelope());
        for s in segments {
            if s.is_hovered(state) {
                return Some(s);
            }
        }
        return None;
    }

    pub fn get_visible_net_segments(&self, rect: &GridRect) -> Vec<&NetSegment> {
        self.net_tree
            .locate_in_envelope_intersecting(&rect.envelope())
            .collect()
    }

    /// Is cell free to place a new component
    pub fn is_free_cell(&self, cell: GridPos, overlap_only: bool) -> bool {
        for nearest in self.tree.locate_within_distance(cell.to_point(), 2) {
            if overlap_only || self.get_component(&nearest.id).unwrap().is_overlap_only() {
                if nearest.contains(cell) {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }

    /// Is cell available for moving an existing component
    pub fn is_available_cell(&self, cell: GridPos, component_id: Id) -> bool {
        for nearest in self.tree.locate_within_distance(cell.to_point(), 2) {
            if nearest.id != component_id {
                if self.get_component(&component_id).unwrap().is_overlap_only()
                    || self.get_component(&nearest.id).unwrap().is_overlap_only()
                {
                    // Check only overlap
                    if nearest.contains(cell) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn get_connected_nets(&self, component_id: &Id) -> HashSet<Id> {
        let mut result = HashSet::new();
        if let Some(comp) = self.get_component(component_id) {
            comp.get_connection_dock_cells()
                .iter()
                .enumerate()
                .for_each(|(inner_id, _cell)| {
                    // TODO: simplify it
                    if let Some(set) = self.connected_nets.get(&&GridBDConnectionPoint {
                        component_id: *component_id,
                        connection_id: inner_id,
                    }) {
                        result.extend(set);
                    }
                });
        }
        result
    }

    pub fn is_available_location(&self, p: GridPos, dim: (i32, i32), component_id: Id) -> bool {
        for x in 0..dim.0 {
            for y in 0..dim.1 {
                if !self.is_available_cell(p + grid_pos(x, y), component_id) {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn dump_to_json(&self) -> Option<String> {
        serde_json::to_string_pretty(&GridBdDump {
            components: self.components.clone(),
            nets: self.nets.clone(),
        })
        .ok()
    }

    pub fn dump_to_svg(&self, theme: Theme) -> String {
        let [c_min_x, c_min_y, c_max_x, c_max_y];
        if self.components.values().len() >= 1 {
            let c_bbox = self.tree.root().envelope();
            [c_min_x, c_min_y] = c_bbox.lower();
            [c_max_x, c_max_y] = c_bbox.upper();
        } else {
            [c_min_x, c_min_y, c_max_x, c_max_y] = [0, 0, 0, 0];
        }

        let [n_min_x, n_min_y, n_max_x, n_max_y];
        if self.nets.values().len() >= 1 {
            let n_bbox = self.net_tree.root().envelope();
            [n_min_x, n_min_y] = n_bbox.lower();
            [n_max_x, n_max_y] = n_bbox.upper();
        } else {
            [n_min_x, n_min_y, n_max_x, n_max_y] = [0, 0, 0, 0];
        }

        let min_x = c_min_x.min(n_min_x);
        let min_y = c_min_y.min(n_min_y);
        let max_x = c_max_x.max(n_max_x);
        let max_y = c_max_y.max(n_max_y);

        let backgound = theme.get_bg_color().to_hex();

        // Fixme:
        let w = max_x - min_x + 2;
        let h: i32 = max_y - min_y + 2;
        let offset = grid_pos(-min_x, -min_y);
        let body = self
            .components
            .values()
            .map(|comp| comp.to_svg(offset, theme))
            .chain(self.nets.values().map(|net| {
                net.to_svg(theme.get_stroke_color(), 0.1, offset, &self)
                    .unwrap_or_default()
            }))
            .collect::<Vec<String>>()
            .join("\n");

        format!(
            "<svg viewBox=\"0 0 {w} {h}\" xmlns=\"http://www.w3.org/2000/svg\" style=\"background-color: {backgound}\">\n{body}\n</svg>"
        )
    }

    pub fn load_from_json(json: String) -> Result<Self, serde_json::Error> {
        let dump: GridBdDump = serde_json::from_str(&json)?;
        let mut result = Self::new();
        if let Some(max_id) = dump.components.keys().max() {
            result.next_component_id = max_id + 1;
        }

        for (id, component) in dump.components {
            result.insert_component(id, component);
        }
        for (_i, net) in dump.nets {
            result.add_net(net);
        }
        // Fixme: need load with same id???
        Ok(result)
    }
}

#[derive(Serialize, Deserialize)]
struct GridBdDump {
    components: HashMap<Id, Component>,
    nets: HashMap<Id, Net>,
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct GridBDConnectionPoint {
    pub component_id: Id,
    pub connection_id: Id,
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
