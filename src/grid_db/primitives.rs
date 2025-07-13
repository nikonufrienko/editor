use std::{
    cell::{LazyCell, RefCell},
    f32::consts::PI,
    sync::Arc,
};

use egui::{
    Color32, Mesh, Painter, Pos2, Shape, Stroke,
    ahash::{HashMap, HashMapExt},
    emath::TSTransform,
    pos2, vec2,
};
use serde::{Deserialize, Serialize};

use crate::{
    field::{Field, FieldState},
    grid_db::tesselate_polygon,
};

use super::{ComponentAction, GridPos, Id, grid_pos};

const STROKE_SCALE: f32 = 0.1;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum LodLevel {
    Max,
    Mid,
    Min, // Minimal quality
}

#[derive(Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq, Debug)]
pub enum Rotation {
    ROT0,
    ROT90,
    ROT180,
    ROT270,
}

impl Rotation {
    pub fn rotated_up(&self) -> Rotation {
        match self {
            Rotation::ROT0 => Rotation::ROT90,
            Rotation::ROT90 => Rotation::ROT180,
            Rotation::ROT180 => Rotation::ROT270,
            Rotation::ROT270 => Rotation::ROT0,
        }
    }

    pub fn rotated_down(&self) -> Rotation {
        match self {
            Rotation::ROT90 => Rotation::ROT0,
            Rotation::ROT180 => Rotation::ROT90,
            Rotation::ROT270 => Rotation::ROT180,
            Rotation::ROT0 => Rotation::ROT270,
        }
    }

    fn cos(&self) -> i32 {
        match self {
            Rotation::ROT90 => 0,
            Rotation::ROT180 => -1,
            Rotation::ROT270 => 0,
            Rotation::ROT0 => 1,
        }
    }

    fn sin(&self) -> i32 {
        match self {
            Rotation::ROT90 => 1,
            Rotation::ROT180 => 0,
            Rotation::ROT270 => -1,
            Rotation::ROT0 => 0,
        }
    }

    fn rotate_grid_pos(&self, point: GridPos, center: GridPos) -> GridPos {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let cos_a = self.cos();
        let sin_a = self.sin();
        grid_pos(
            center.x + dx * cos_a - dy * sin_a,
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
pub struct PrimitiveComponent {
    pub typ: PrimitiveType,
    pub pos: GridPos,
    pub rotation: Rotation,
}

impl PrimitiveComponent {
    pub const ACTIONS: &'static [ComponentAction] = &[
        ComponentAction::RotateDown,
        ComponentAction::RotateUp,
        ComponentAction::Remove,
    ];
    const CONNECTION_SCALE: f32 = 0.1;

    pub fn get_dimension(&self) -> (i32, i32) {
        let (w, h) = self.typ.get_dimension_raw();
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
            Rotation::ROT0 => vec2(0.0, 0.0),
            Rotation::ROT90 => vec2(dim.0 as f32, 0.0) * state.grid_size,
            Rotation::ROT180 => vec2(dim.0 as f32, dim.1 as f32) * state.grid_size,
            Rotation::ROT270 => vec2(0.0, dim.1 as f32) * state.grid_size,
        };
        points
            .iter()
            .map(|p| self.rotation.rotate_point(*p, rot_center) + rot_ofs)
            .collect()
    }

    fn apply_rotation_grid_pos(&self, points: Vec<GridPos>) -> Vec<GridPos> {
        let rot_center = self.pos;
        let dim = self.get_dimension();
        let rot_ofs = match self.rotation {
            Rotation::ROT0 => grid_pos(0, 0),
            Rotation::ROT90 => grid_pos(dim.0 - 1, 0),
            Rotation::ROT180 => grid_pos(dim.0 - 1, dim.1 - 1),
            Rotation::ROT270 => grid_pos(0, dim.1 - 1),
        };
        points
            .iter()
            .map(|p| self.rotation.rotate_grid_pos(*p, rot_center) + rot_ofs)
            .collect()
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

    pub fn get_connection_dock_cell(&self, connection_id: Id) -> Option<GridPos> {
        if connection_id >= self.typ.get_connections_number() {
            return None;
        }
        Some(
            self.apply_rotation_grid_pos(vec![
                self.typ.get_dock_cell_raw(connection_id, &self.pos),
            ])[0],
        )
    }

    pub fn get_connection_position(&self, connection_id: Id, state: &FieldState) -> Option<Pos2> {
        if connection_id >= self.typ.get_connections_number() {
            return None;
        }
        Some(
            self.apply_rotation(
                vec![
                    self.typ
                        .get_connection_position_raw(connection_id, state, &self.pos),
                ],
                state,
            )[0],
        )
    }

    pub fn display(&self, state: &FieldState, painter: &Painter) {
        let stroke_w = 1.0 * state.scale;
        let fill_color = Color32::GRAY;
        let stroke_color = Color32::DARK_GRAY;
        let stroke = Stroke {
            color: stroke_color,
            width: stroke_w,
        };

        let lod_level = state.lod_level();
        // Draw lines:
        if state.scale > Field::LOD_LEVEL_MIN_SCALE {
            self.typ
                .get_lines(state, &self.pos)
                .iter()
                .for_each(|segment| {
                    let rotated = self.apply_rotation(segment.to_vec(), state);
                    painter.line(rotated, stroke);
                });
        }
        let mut shape = Shape::Mesh(get_cached_mesh(self.typ, self.rotation, lod_level));
        shape.transform(TSTransform {
            scaling: state.grid_size,
            translation: state.grid_to_screen(&self.pos).to_vec2(),
        });
        painter.add(shape);

        // Draw connections:
        if state.scale > Field::LOD_LEVEL_MIN_SCALE {
            let radius = state.grid_size * Self::CONNECTION_SCALE;
            (0..self.typ.get_connections_number()).for_each(|i| {
                if self.typ.is_inverted_connection(i) {
                    painter.circle(
                        self.apply_rotation(
                            vec![self.typ.get_connection_position_raw(i, state, &self.pos)],
                            state,
                        )[0],
                        radius * 2.0,
                        fill_color,
                        stroke,
                    );
                } else {
                    painter.circle_filled(
                        self.apply_rotation(
                            vec![self.typ.get_connection_position_raw(i, state, &self.pos)],
                            state,
                        )[0],
                        radius,
                        stroke_color,
                    );
                }
            });
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum PrimitiveType {
    And(usize),
    Or(usize),
    Not,
    Input,
    Output,
}

impl PrimitiveType {
    //
    // *** And gate ***
    //
    fn get_and_gate_dimension_raw(n_inputs: usize) -> (i32, i32) {
        if n_inputs % 2 == 0 {
            (3, (2 * n_inputs - 1) as i32)
        } else {
            (3, n_inputs as i32)
        }
    }

    fn get_and_gate_dock_cell_raw(
        connection_id: Id,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> GridPos {
        if connection_id < n_inputs as Id {
            if n_inputs % 2 == 0 {
                *primitive_pos + grid_pos(-1, 2 * connection_id as i32)
            } else {
                *primitive_pos + grid_pos(-1, connection_id as i32)
            }
        } else {
            let raw_dim = Self::get_and_gate_dimension_raw(n_inputs);
            *primitive_pos + grid_pos(raw_dim.0, raw_dim.1 / 2)
        }
    }

    fn get_and_gate_connection_position_raw(
        connection_id: Id,
        state: &FieldState,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> Pos2 {
        let screen_pos = state.grid_to_screen(&primitive_pos);
        let (w, h) = Self::get_and_gate_dimension_raw(n_inputs);
        if connection_id < n_inputs {
            if n_inputs % 2 == 0 {
                screen_pos + vec2(0.0, ((2 * connection_id) as f32 + 0.5) * state.grid_size)
            } else {
                screen_pos + vec2(0.0, (connection_id as f32 + 0.5) * state.grid_size)
            }
        } else {
            screen_pos + vec2(w as f32 * state.grid_size, h as f32 * state.grid_size / 2.0)
        }
    }

    fn get_and_gate_polygon_points_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Pos2> {
        let stroke_w = STROKE_SCALE;
        let height = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let radius_x = 1.0 - stroke_w / 2.0;
        let radius_y = height as f32 / 2.0 - stroke_w / 2.0;
        let center = vec2(2.0, height / 2.0);

        let n_points = match lod_level {
            LodLevel::Max => 30,
            LodLevel::Mid => 5,
            LodLevel::Min => 2,
        }; // Number of points per curve segment
        let mut points = (0..=n_points)
            .map(|i| {
                let angle = -PI / 2.0 + PI * (i as f32 / n_points as f32);
                let x = center.x + radius_x * angle.cos();
                let y = center.y + radius_y * angle.sin();
                Pos2::new(x, y)
            })
            .collect::<Vec<_>>();
        points.insert(0, pos2(stroke_w / 2.0, stroke_w / 2.0));
        points.insert(0, pos2(stroke_w / 2.0, height - stroke_w / 2.0));
        points
    }

    //
    // *** Or gate ***
    //
    fn get_or_gate_dimension_raw(n_inputs: usize) -> (i32, i32) {
        return Self::get_and_gate_dimension_raw(n_inputs);
    }

    fn get_or_gate_dock_cell_raw(
        connection_id: Id,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> GridPos {
        Self::get_and_gate_dock_cell_raw(connection_id, n_inputs, primitive_pos)
    }

    fn get_or_gate_connection_position_raw(
        connection_id: Id,
        state: &FieldState,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> Pos2 {
        Self::get_and_gate_connection_position_raw(connection_id, state, n_inputs, primitive_pos)
    }

    fn get_or_gate_polygon_points_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Pos2> {
        let grid_size = 1.0;
        let pos = pos2(0.0, 0.0);
        let stroke_w = STROKE_SCALE;
        let grid_size = grid_size;
        let height_factor = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let height = height_factor * grid_size;

        let center_y = pos.y + height / 2.0;

        // Configurable parameters
        let tip_x_factor = 3.0; // Tip position multiplier
        let curve_strength = 0.0; // Right curve bend strength (0.0-1.0)
        let n_curve_points = match lod_level {
            LodLevel::Max => 30,
            LodLevel::Mid => 5,
            LodLevel::Min => 2,
        }; // Number of points per curve segment
        let left_curve_strength = 1.0; // Left curve concavity strength

        // Key points
        let top_point = pos2(pos.x + stroke_w * 0.5, pos.y + stroke_w * 0.5);
        let bottom_point = pos2(pos.x + stroke_w * 0.5, pos.y + height - stroke_w * 0.5);
        let left_control = pos2(pos.x + grid_size * left_curve_strength, center_y);
        let tip_point = pos2(pos.x + tip_x_factor * grid_size, center_y);
        let middle_x = pos.x + 2.2 * grid_size;

        let mut points = Vec::new();

        // Left concave curve (single quadratic Bezier from top to bottom)
        for i in 0..=n_curve_points {
            let t = i as f32 / n_curve_points as f32;
            // Quadratic Bezier formula: P0 = top_point, P1 = left_control, P2 = bottom_point
            let x = (1.0 - t).powi(2) * top_point.x
                + 2.0 * (1.0 - t) * t * left_control.x
                + t.powi(2) * bottom_point.x;
            let y = (1.0 - t).powi(2) * top_point.y
                + 2.0 * (1.0 - t) * t * left_control.y
                + t.powi(2) * bottom_point.y;
            points.push(pos2(x, y));
        }

        // Calculate control points for right curves
        let bottom_control = pos2(
            middle_x,
            bottom_point.y + (tip_point.y - bottom_point.y) * curve_strength,
        );

        let top_control = pos2(
            middle_x,
            top_point.y + (tip_point.y - top_point.y) * curve_strength,
        );

        // Bottom right curve (from bottom point to tip)
        for i in 1..=n_curve_points {
            let t = i as f32 / n_curve_points as f32;
            let x = (1.0 - t).powi(2) * bottom_point.x
                + 2.0 * (1.0 - t) * t * bottom_control.x
                + t.powi(2) * tip_point.x;
            let y = (1.0 - t).powi(2) * bottom_point.y
                + 2.0 * (1.0 - t) * t * bottom_control.y
                + t.powi(2) * tip_point.y;
            points.push(pos2(x, y));
        }

        // Top right curve (from tip to top point)
        for i in 1..=n_curve_points {
            let t = i as f32 / n_curve_points as f32;
            let x = (1.0 - t).powi(2) * tip_point.x
                + 2.0 * (1.0 - t) * t * top_control.x
                + t.powi(2) * top_point.x;
            let y = (1.0 - t).powi(2) * tip_point.y
                + 2.0 * (1.0 - t) * t * top_control.y
                + t.powi(2) * top_point.y;
            points.push(pos2(x, y));
        }

        points
    }

    fn get_or_gate_lines_raw(
        state: &FieldState,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> Vec<Vec<Pos2>> {
        let mut result = Vec::with_capacity(n_inputs);
        for i in 0..n_inputs {
            let p0: Pos2 =
                Self::get_or_gate_connection_position_raw(i, state, n_inputs, primitive_pos);
            let p1: Pos2 = p0 + vec2(state.grid_size, 0.0);
            result.push(vec![p0, p1]);
        }
        result
    }

    //
    // *** Input ***
    //
    fn get_input_dock_cell_raw(primitive_pos: &GridPos) -> GridPos {
        return *primitive_pos + grid_pos(2, 0);
    }

    fn get_input_polygon_points_raw() -> Vec<Pos2> {
        let pos = pos2(0.0, 0.0);
        let grid_size = 1.0;
        let stroke_w = STROKE_SCALE;

        let p0 = pos + vec2(stroke_w * 0.5, stroke_w * 0.5);
        let p1 = pos + vec2(grid_size * 1.5 - stroke_w * 0.5, stroke_w * 0.5);
        let p2 = pos + vec2(2.0 * grid_size - stroke_w * 0.5, 0.5 * grid_size);
        let p3 = pos + vec2(grid_size * 1.5 - stroke_w * 0.5, grid_size - stroke_w * 0.5);
        let p4 = pos + vec2(stroke_w * 0.5, grid_size - stroke_w * 0.5);
        let p5 = pos + vec2(stroke_w * 0.5 + grid_size * 0.5, grid_size * 0.5);

        return vec![p0, p1, p2, p3, p4, p5];
    }

    fn get_input_connection_position_raw(
        _connection_id: Id,
        state: &FieldState,
        primitive_pos: &GridPos,
    ) -> Pos2 {
        state.grid_to_screen(primitive_pos) + vec2(2.0 * state.grid_size, 0.5 * state.grid_size)
    }

    //
    // *** Output ***
    //
    fn get_output_dock_cell_raw(primitive_pos: &GridPos) -> GridPos {
        return *primitive_pos + grid_pos(-1, 0);
    }

    fn get_output_polygon_points_raw() -> Vec<Pos2> {
        Self::get_input_polygon_points_raw()
    }

    fn get_output_connection_position_raw(
        _connection_id: Id,
        state: &FieldState,
        primitive_pos: &GridPos,
    ) -> Pos2 {
        state.grid_to_screen(primitive_pos) + vec2(0.0, 0.5 * state.grid_size)
    }

    fn get_output_lines_raw(state: &FieldState, primitive_pos: &GridPos) -> Vec<Vec<Pos2>> {
        vec![vec![
            state.grid_to_screen(primitive_pos) + vec2(0.0, 0.5 * state.grid_size),
            state.grid_to_screen(primitive_pos)
                + vec2(0.5 * state.grid_size, 0.5 * state.grid_size),
        ]]
    }

    //
    // *** Not ***
    //
    fn get_not_dock_cell_raw(connection_id: Id, primitive_pos: &GridPos) -> GridPos {
        if connection_id == 0 {
            *primitive_pos + grid_pos(-1, 1)
        } else {
            *primitive_pos + grid_pos(3, 1)
        }
    }

    fn get_not_polygon_points_raw() -> Vec<Pos2> {
        let pos = pos2(0.0, 0.0);
        let stroke_w = STROKE_SCALE;
        let grid_size = 1.0;
        let p0 = pos + vec2(stroke_w * 0.5, stroke_w * 0.5);
        let p1 = pos + vec2(3.0 * grid_size - stroke_w * 0.5, grid_size * 1.5);
        let p2 = pos + vec2(stroke_w * 0.5, 3.0 * grid_size - stroke_w * 0.5);
        return vec![p0, p1, p2];
    }

    fn get_not_connection_position_raw(
        connection_id: Id,
        state: &FieldState,
        primitive_pos: &GridPos,
    ) -> Pos2 {
        if connection_id == 0 {
            state.grid_to_screen(primitive_pos) + vec2(0.0, state.grid_size + 0.5 * state.grid_size)
        } else {
            state.grid_to_screen(primitive_pos)
                + vec2(
                    3.0 * state.grid_size,
                    state.grid_size + 0.5 * state.grid_size,
                )
        }
    }

    //
    // *** Common ***
    //
    pub fn get_connections_number(&self) -> usize {
        match self {
            Self::And(n_inputs) => *n_inputs + 1,
            Self::Or(n_inputs) => *n_inputs + 1,
            Self::Not => 2,
            Self::Input => 1,
            Self::Output => 1,
        }
    }

    fn get_dimension_raw(&self) -> (i32, i32) {
        match self {
            Self::And(n_inputs) => Self::get_and_gate_dimension_raw(*n_inputs),
            Self::Or(n_inputs) => Self::get_or_gate_dimension_raw(*n_inputs),
            Self::Not => (3, 3),
            Self::Input => (2, 1),
            Self::Output => (2, 1),
        }
    }

    fn get_dock_cell_raw(&self, connection_id: Id, primitive_pos: &GridPos) -> GridPos {
        match self {
            Self::And(n_inputs) => {
                Self::get_and_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
            Self::Or(n_inputs) => {
                Self::get_or_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
            Self::Not => Self::get_not_dock_cell_raw(connection_id, primitive_pos),
            Self::Input => Self::get_input_dock_cell_raw(primitive_pos),
            Self::Output => Self::get_output_dock_cell_raw(primitive_pos),
        }
    }

    fn get_connection_position_raw(
        &self,
        connection_id: Id,
        state: &FieldState,
        primitive_pos: &GridPos,
    ) -> Pos2 {
        match self {
            Self::And(n_inputs) => Self::get_and_gate_connection_position_raw(
                connection_id,
                state,
                *n_inputs,
                primitive_pos,
            ),
            Self::Or(n_inputs) => Self::get_or_gate_connection_position_raw(
                connection_id,
                state,
                *n_inputs,
                primitive_pos,
            ),
            Self::Not => Self::get_not_connection_position_raw(connection_id, state, primitive_pos),
            Self::Input => {
                Self::get_input_connection_position_raw(connection_id, state, primitive_pos)
            }
            Self::Output => {
                Self::get_output_connection_position_raw(connection_id, state, primitive_pos)
            }
        }
    }

    fn get_polygon_points_raw(&self, lod_level: LodLevel) -> Vec<Pos2> {
        match self {
            Self::And(n_inputs) => Self::get_and_gate_polygon_points_raw(*n_inputs, lod_level),
            Self::Or(n_inputs) => Self::get_or_gate_polygon_points_raw(*n_inputs, lod_level),
            Self::Input => Self::get_input_polygon_points_raw(),
            Self::Output => Self::get_output_polygon_points_raw(),
            Self::Not => Self::get_not_polygon_points_raw(),
        }
    }

    fn get_lines(&self, state: &FieldState, primitive_pos: &GridPos) -> Vec<Vec<Pos2>> {
        match self {
            Self::Or(n_inputs) => Self::get_or_gate_lines_raw(state, *n_inputs, primitive_pos),
            Self::Output => Self::get_output_lines_raw(state, primitive_pos),
            _ => vec![],
        }
    }

    fn is_inverted_connection(&self, connection_id: Id) -> bool {
        match self {
            Self::Not => connection_id == 1,
            _ => false,
        }
    }
}

thread_local! {
    static CACHE: LazyCell<RefCell<HashMap<(PrimitiveType, Rotation, LodLevel), Arc<Mesh>>>> =
        LazyCell::new(|| RefCell::new(HashMap::new()));
}

fn apply_rotation_for_points(points: Vec<Pos2>, rotation: Rotation, dim: (i32, i32)) -> Vec<Pos2> {
    let rot_ofs = match rotation {
        Rotation::ROT0 => vec2(0.0, 0.0),
        Rotation::ROT90 => vec2(dim.0 as f32, 0.0),
        Rotation::ROT180 => vec2(dim.0 as f32, dim.1 as f32),
        Rotation::ROT270 => vec2(0.0, dim.1 as f32),
    };
    points
        .iter()
        .map(|p| rotation.rotate_point(*p, pos2(0.0, 0.0)) + rot_ofs)
        .collect()
}

fn get_cached_mesh(typ: PrimitiveType, rotation: Rotation, lod_level: LodLevel) -> Arc<Mesh> {
    CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        if let Some(result) = map.get(&(typ, rotation, lod_level)) {
            return result.clone();
        }
        let points = typ.get_polygon_points_raw(lod_level);
        let rotated_points = apply_rotation_for_points(points, rotation, typ.get_dimension_raw());
        let mesh = tesselate_polygon(
            rotated_points,
            Color32::GRAY,
                lod_level == LodLevel::Max,
            Color32::DARK_GRAY,
            STROKE_SCALE,
        );
        let arc = Arc::new(mesh);
        let arc_cloned = arc.clone();
        map.insert((typ.clone(), rotation, lod_level), arc);
        return arc_cloned;
    })
}
