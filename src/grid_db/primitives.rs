use std::f32::consts::PI;

use egui::{Color32, Painter, Pos2, Shape, Stroke, pos2, vec2};
use serde::{Deserialize, Serialize};

use crate::{field::FieldState, grid_db::get_concave_polygon_shape};

use super::{ComponentAction, GridPos, Id, grid_pos};

#[derive(Clone, Serialize, Deserialize)]
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
        self.typ
            .get_lines(state, &self.pos)
            .iter()
            .for_each(|segment| {
                let rotated = self.apply_rotation(segment.to_vec(), state);
                painter.line_segment([rotated[0], rotated[1]], stroke);
            });

        // Получаем точки полигона и применяем вращение
        let (polygon_type, raw_points) =
            self.typ.get_polygon_points_raw(state, stroke_w, &self.pos);
        let points: Vec<Pos2> = self.apply_rotation(raw_points, state);

        match polygon_type {
            PolygonType::Concave => painter.add(get_concave_polygon_shape(
                points,
                Color32::GRAY,
                Color32::DARK_GRAY,
                stroke_w,
            )),
            PolygonType::Convex => painter.add(Shape::convex_polygon(points, fill_color, stroke)),
        };

        let radius = state.grid_size * Self::CONNECTION_SCALE;
        // Рисуем соединения (как раньше)
        (0..=self.typ.get_connections_number()).for_each(|i| {
            painter.circle_filled(
                self.apply_rotation(
                    vec![self.typ.get_connection_position_raw(i, state, &self.pos)],
                    state,
                )[0],
                radius,
                Color32::DARK_GRAY,
            );
        });
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub enum PrimitiveType {
    And(usize),
    Or(usize),
}

impl PrimitiveType {
    pub fn get_connections_number(&self) -> usize {
        match self {
            Self::And(n_inputs) => *n_inputs + 1,
            Self::Or(n_inputs) => *n_inputs + 1,
        }
    }

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

    fn get_and_gate_polygon_points_raw(
        state: &FieldState,
        stroke_w: f32,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> Vec<Pos2> {
        let height = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        } * state.grid_size;
        let radius_x = state.grid_size - stroke_w / 2.0;
        let radius_y = height as f32 / 2.0 - stroke_w / 2.0;
        let pos = state.grid_to_screen(primitive_pos);
        let center = pos + vec2(2.0 * state.grid_size, height / 2.0);

        let n_points = 30;
        let mut points = (0..=n_points)
            .map(|i| {
                let angle = -PI / 2.0 + PI * (i as f32 / n_points as f32);
                let x = center.x + radius_x * angle.cos();
                let y = center.y + radius_y * angle.sin();
                Pos2::new(x, y)
            })
            .collect::<Vec<_>>();
        points.insert(0, pos + vec2(stroke_w / 2.0, stroke_w / 2.0));
        points.insert(0, pos + vec2(stroke_w / 2.0, height - stroke_w / 2.0));
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

    fn get_or_gate_polygon_points_raw(
        state: &FieldState,
        stroke_w: f32,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> Vec<Pos2> {
        let grid_size = state.grid_size;
        let height_factor = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let height = height_factor * grid_size;

        let pos = state.grid_to_screen(primitive_pos);
        let center_y = pos.y + height / 2.0;

        // Configurable parameters
        let tip_x_factor = 3.0; // Tip position multiplier
        let curve_strength = 0.0; // Right curve bend strength (0.0-1.0)
        let n_curve_points = 30; // Number of points per curve segment
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

    //
    // *** Common ***
    //

    fn get_dimension_raw(&self) -> (i32, i32) {
        match self {
            Self::And(n_inputs) => Self::get_and_gate_dimension_raw(*n_inputs),
            Self::Or(n_inputs) => Self::get_or_gate_dimension_raw(*n_inputs),
        }
    }

    fn get_dock_cell_raw(&self, connection_id: Id, primitive_pos: &GridPos) -> GridPos {
        // TODO: remove Option here
        match self {
            Self::And(n_inputs) => {
                Self::get_and_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
            Self::Or(n_inputs) => {
                Self::get_or_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
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
        }
    }

    fn get_polygon_points_raw(
        &self,
        state: &FieldState,
        stroke_w: f32,
        primitive_pos: &GridPos,
    ) -> (PolygonType, Vec<Pos2>) {
        match self {
            Self::And(n_inputs) => (
                PolygonType::Convex,
                Self::get_and_gate_polygon_points_raw(state, stroke_w, *n_inputs, primitive_pos),
            ),
            Self::Or(n_inputs) => (
                PolygonType::Concave,
                Self::get_or_gate_polygon_points_raw(state, stroke_w, *n_inputs, primitive_pos),
            ),
        }
    }

    fn get_or_gate_lines_raw(
        state: &FieldState,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> Vec<[Pos2; 2]> {
        let mut result = Vec::with_capacity(n_inputs);
        for i in 0..n_inputs {
            let p0: Pos2 =
                Self::get_or_gate_connection_position_raw(i, state, n_inputs, primitive_pos);
            let p1: Pos2 = p0 + vec2(state.grid_size, 0.0);
            result.push([p0, p1]);
        }
        result
    }

    fn get_lines(&self, state: &FieldState, primitive_pos: &GridPos) -> Vec<[Pos2; 2]> {
        match self {
            Self::And(_n_inputs) => vec![],
            Self::Or(n_inputs) => Self::get_or_gate_lines_raw(state, *n_inputs, primitive_pos),
        }
    }
}

enum PolygonType {
    Concave,
    Convex,
}
