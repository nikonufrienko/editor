use std::f32::consts::TAU;
use std::{
    cell::{LazyCell, RefCell},
    f32::consts::PI,
    sync::Arc,
    vec,
};

use egui::Theme;
use egui::{
    Color32, Mesh, Painter, Pos2, Shape, Stroke,
    ahash::{HashMap, HashMapExt},
    emath::TSTransform,
    pos2, vec2,
};
use serde::{Deserialize, Serialize};

use crate::grid_db::{ComponentColor, STROKE_SCALE};
use crate::{
    field::{Field, FieldState, SVG_DUMMY_STATE},
    grid_db::{svg_circle_filled, svg_line, svg_polygon, tesselate_polygon},
};

use super::{ComponentAction, GridPos, Id, grid_pos};

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

    pub fn get_rotated_dim(&self, (w, h): (i32, i32)) -> (i32, i32) {
        match self {
            Rotation::ROT0 => (w, h),
            Rotation::ROT90 => (h, w),
            Rotation::ROT180 => (w, h),
            Rotation::ROT270 => (h, w),
        }
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

    fn apply_rotation(&self, point: Pos2, state: &FieldState) -> Pos2 {
        let rot_center = state.grid_to_screen(&self.pos);
        let dim = self.get_dimension();
        let rot_ofs = match self.rotation {
            Rotation::ROT0 => vec2(0.0, 0.0),
            Rotation::ROT90 => vec2(dim.0 as f32, 0.0) * state.grid_size,
            Rotation::ROT180 => vec2(dim.0 as f32, dim.1 as f32) * state.grid_size,
            Rotation::ROT270 => vec2(0.0, dim.1 as f32) * state.grid_size,
        };
        self.rotation.rotate_point(point, rot_center) + rot_ofs
    }

    fn apply_rotation_for_points(&self, points: &mut Vec<Pos2>, state: &FieldState) {
        let rot_center = state.grid_to_screen(&self.pos);
        let dim = self.get_dimension();
        let rot_ofs = match self.rotation {
            Rotation::ROT0 => vec2(0.0, 0.0),
            Rotation::ROT90 => vec2(dim.0 as f32, 0.0) * state.grid_size,
            Rotation::ROT180 => vec2(dim.0 as f32, dim.1 as f32) * state.grid_size,
            Rotation::ROT270 => vec2(0.0, dim.1 as f32) * state.grid_size,
        };
        for point in points {
            *point = self.rotation.rotate_point(*point, rot_center) + rot_ofs;
        }
    }

    fn apply_rotation_grid_pos(&self, point: GridPos) -> GridPos {
        let rot_center = self.pos;
        let dim = self.get_dimension();
        let rot_ofs = match self.rotation {
            Rotation::ROT0 => grid_pos(0, 0),
            Rotation::ROT90 => grid_pos(dim.0 - 1, 0),
            Rotation::ROT180 => grid_pos(dim.0 - 1, dim.1 - 1),
            Rotation::ROT270 => grid_pos(0, dim.1 - 1),
        };
        self.rotation.rotate_grid_pos(point, rot_center) + rot_ofs
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
        Some(self.apply_rotation_grid_pos(self.typ.get_dock_cell_raw(connection_id, &self.pos)))
    }

    pub fn get_connection_position(&self, connection_id: Id, state: &FieldState) -> Option<Pos2> {
        if connection_id >= self.typ.get_connections_number() {
            return None;
        }
        Some(self.apply_rotation(
            self.typ.get_connection_position_raw(connection_id) * state.grid_size
                + state.grid_to_screen(&self.pos).to_vec2(),
            state,
        ))
    }

    pub fn display(&self, state: &FieldState, painter: &Painter, theme: Theme) {
        let stroke_w = 1.0 * state.scale;
        let _fill_color = theme.get_fill_color();
        let stroke_color = theme.get_stroke_color();
        let stroke = Stroke {
            color: stroke_color,
            width: stroke_w,
        };
        let lod_level = state.lod_level();
        let screen_pos = state.grid_to_screen(&self.pos).to_vec2();
        // Draw lines:
        if state.scale > Field::LOD_LEVEL_MIN_SCALE {
            for line in self.typ.get_lines(lod_level) {
                let mut line = line;
                for p in &mut line {
                    *p = *p * state.grid_size + screen_pos;
                }
                self.apply_rotation_for_points(&mut line, state);
                painter.line(line, stroke);
            }
        }
        for mesh in get_cached_meshes(self.typ, self.rotation, lod_level, theme) {
            let mut shape = Shape::Mesh(mesh);
            shape.transform(TSTransform {
                scaling: state.grid_size,
                translation: screen_pos,
            });
            painter.add(shape);
        }

        // Draw connections:
        if state.scale > Field::LOD_LEVEL_MIN_SCALE {
            let radius = state.grid_size * Self::CONNECTION_SCALE;
            (0..self.typ.get_connections_number()).for_each(|i| {
                painter.circle_filled(
                    self.apply_rotation(
                        self.typ.get_connection_position_raw(i) * state.grid_size + screen_pos,
                        state,
                    ),
                    radius,
                    stroke_color,
                );
            });
        }
    }

    pub fn get_svg(&self, offset: GridPos, scale: f32, theme: Theme) -> String {
        // FIXME:
        let fill_color = theme.get_fill_color();
        let stroke_color = theme.get_stroke_color();
        let pos: GridPos = self.pos + offset;
        let raw_offset = vec2(pos.x as f32, pos.y as f32);
        let offset_vec2 = vec2(offset.x as f32, offset.y as f32);
        let pos_vec2 = vec2(self.pos.x as f32, self.pos.y as f32);
        let stroke_w = STROKE_SCALE * scale;

        // Lines
        let mut result = String::new();
        let raw_lines = self.typ.get_lines(LodLevel::Max);
        for raw_line in raw_lines {
            let mut raw_line = raw_line;
            self.apply_rotation_for_points(&mut raw_line, &SVG_DUMMY_STATE);
            for p in &mut raw_line {
                *p = (*p + raw_offset) * scale;
            }
            result.push_str(&(svg_line(&raw_line, stroke_color, stroke_w) + &"\n"));
        }

        // Ports:
        let radius = Self::CONNECTION_SCALE * scale;
        (0..self.typ.get_connections_number()).for_each(|i| {
            result.push_str(
                &(svg_circle_filled(
                    (self.apply_rotation(
                        self.typ.get_connection_position_raw(i) + pos_vec2,
                        &SVG_DUMMY_STATE,
                    ) + offset_vec2)
                        * scale,
                    radius,
                    stroke_color,
                ) + &"\n"),
            );
        });

        // Polygons:
        let mut polygons_points = self.typ.get_polygons_points_raw(LodLevel::Max);
        for points in &mut polygons_points {
            apply_rotation_for_raw_points(points, self.rotation, self.typ.get_dimension_raw());
            for p in &mut *points {
                *p = (*p + raw_offset) * scale;
            }
            result.push_str(&(svg_polygon(&points, fill_color, stroke_color, stroke_w) + &"\n"));
        }

        result
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum PrimitiveType {
    // Logic gates:
    And(usize),
    Or(usize),
    Xor(usize),
    Nand(usize),
    Not,
    Point,

    // Muxes:
    Mux(usize),

    // Logic gates:
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

    fn get_and_gate_connection_position_raw(connection_id: Id, n_inputs: usize) -> Pos2 {
        let (w, h) = Self::get_and_gate_dimension_raw(n_inputs);
        if connection_id < n_inputs {
            if n_inputs % 2 == 0 {
                pos2(0.0, (2 * connection_id) as f32 + 0.5)
            } else {
                pos2(0.0, connection_id as f32 + 0.5)
            }
        } else {
            pos2(w as f32, h as f32 / 2.0)
        }
    }

    fn get_and_gate_shape_points(
        stroke_w: f32,
        radius_x: f32,
        radius_y: f32,
        center: Pos2,
        height: f32,
        lod_level: LodLevel,
    ) -> Vec<Pos2> {
        let n_points = match lod_level {
            LodLevel::Max => 30,
            LodLevel::Mid => 8,
            LodLevel::Min => 4,
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

    fn get_and_gate_polygon_points_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Pos2> {
        let stroke_w = STROKE_SCALE;
        let height = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let radius_x = 1.0 - stroke_w / 2.0;
        let radius_y = height as f32 / 2.0 - stroke_w / 2.0;
        let center = pos2(2.0, height / 2.0);
        Self::get_and_gate_shape_points(stroke_w, radius_x, radius_y, center, height, lod_level)
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

    fn get_or_gate_connection_position_raw(connection_id: Id, n_inputs: usize) -> Pos2 {
        Self::get_and_gate_connection_position_raw(connection_id, n_inputs)
    }

    fn get_or_left_curve(
        top_point: Pos2,
        bottom_point: Pos2,
        left_control: Pos2,
        n_curve_points: usize,
    ) -> Vec<Pos2> {
        let mut points = Vec::with_capacity(n_curve_points);
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
        points
    }

    fn get_or_gate_shape_points(
        top_point: Pos2,
        bottom_point: Pos2,
        left_control: Pos2,
        tip_point: Pos2,
        middle_x: f32,
        n_curve_points: usize,
    ) -> Vec<Pos2> {
        let mut points = Vec::new();
        // Configurable parameters
        let curve_strength = 0.0; // Right curve bend strength (0.0-1.0)

        points.extend(Self::get_or_left_curve(
            top_point,
            bottom_point,
            left_control,
            n_curve_points,
        ));

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

    fn get_or_gate_polygon_points_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Pos2> {
        let grid_size = 1.0;
        let pos = pos2(0.0, 0.0);
        let stroke_w = STROKE_SCALE;
        let height_factor = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let height = height_factor * grid_size;

        // Configurable parameters
        let center_y = pos.y + height / 2.0;
        let tip_x_factor = 3.0; // Tip position multiplier
        let left_curve_strength = 1.0; // Left curve concavity strength
        let n_curve_points = match lod_level {
            LodLevel::Max => 30,
            LodLevel::Mid => 5,
            LodLevel::Min => 2,
        }; // Number of points per curve segment

        // Key points
        let top_point = pos2(pos.x + stroke_w * 0.5, pos.y + stroke_w * 0.5);
        let bottom_point = pos2(pos.x + stroke_w * 0.5, pos.y + height - stroke_w * 0.5);
        let left_control = pos2(pos.x + grid_size * left_curve_strength, center_y);
        let tip_point = pos2(pos.x + tip_x_factor * grid_size, center_y);
        let middle_x = pos.x + 2.2 * grid_size;

        Self::get_or_gate_shape_points(
            top_point,
            bottom_point,
            left_control,
            tip_point,
            middle_x,
            n_curve_points,
        )
    }

    fn get_or_gate_lines_raw(n_inputs: usize) -> Vec<Vec<Pos2>> {
        let mut result = Vec::with_capacity(n_inputs);

        let grid_size = 1.0;
        let pos = pos2(0.0, 0.0);
        let stroke_w = STROKE_SCALE;
        let height_factor = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let height = height_factor * grid_size;

        let top_point = pos2(pos.x + stroke_w * 0.5, pos.y + stroke_w * 0.5);
        let bottom_point = pos2(pos.x + stroke_w * 0.5, pos.y + height - stroke_w * 0.5);
        let left_curve_strength = 1.0;
        let left_control = pos2(
            pos.x + grid_size * left_curve_strength,
            pos.y + height / 2.0,
        );

        let min_y = top_point.y;
        let max_y = bottom_point.y;
        let y_range = max_y - min_y;

        for i in 0..n_inputs {
            let p0 = Self::get_or_gate_connection_position_raw(i, n_inputs);
            let y = p0.y;

            let t = if y_range.abs() < f32::EPSILON {
                0.5
            } else {
                ((y - min_y) / y_range).clamp(0.0, 1.0)
            };

            let x = (1.0 - t).powi(2) * top_point.x
                + 2.0 * (1.0 - t) * t * left_control.x
                + t.powi(2) * bottom_point.x;

            result.push(vec![p0, pos2(x, y)]);
        }

        result
    }

    //
    // *** Xor gate ***
    //
    fn get_xor_gate_dimension_raw(n_inputs: usize) -> (i32, i32) {
        return Self::get_and_gate_dimension_raw(n_inputs);
    }

    fn get_xor_gate_dock_cell_raw(
        connection_id: Id,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> GridPos {
        Self::get_and_gate_dock_cell_raw(connection_id, n_inputs, primitive_pos)
    }

    fn get_xor_gate_connection_position_raw(connection_id: Id, n_inputs: usize) -> Pos2 {
        Self::get_and_gate_connection_position_raw(connection_id, n_inputs)
    }

    fn get_xor_gate_polygon_points_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Pos2> {
        let grid_size = 1.0;
        let pos = pos2(0.0, 0.0);
        let stroke_w = STROKE_SCALE;
        let height_factor = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let height = height_factor * grid_size;

        let center_y = pos.y + height / 2.0;

        // Configurable parameters
        let tip_x_factor = 3.0; // Tip position multiplier
        let n_curve_points = match lod_level {
            LodLevel::Max => 30,
            LodLevel::Mid => 5,
            LodLevel::Min => 2,
        }; // Number of points per curve segment
        let left_curve_strength = 1.0; // Left curve concavity strength

        // Key points
        let top_point = pos2(
            pos.x + 0.25 * grid_size + stroke_w * 0.5,
            pos.y + stroke_w * 0.5,
        );
        let bottom_point = pos2(
            pos.x + 0.25 * grid_size + stroke_w * 0.5,
            pos.y + height - stroke_w * 0.5,
        );
        let left_control = pos2(
            pos.x + 0.25 * grid_size + grid_size * left_curve_strength,
            center_y,
        );
        let tip_point = pos2(pos.x + tip_x_factor * grid_size, center_y);
        let middle_x = pos.x + 2.2 * grid_size;

        Self::get_or_gate_shape_points(
            top_point,
            bottom_point,
            left_control,
            tip_point,
            middle_x,
            n_curve_points,
        )
    }

    fn get_xor_gate_lines_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        let mut result = Vec::with_capacity(n_inputs + 1);
        result.extend(Self::get_or_gate_lines_raw(n_inputs));
        let grid_size = 1.0;
        let pos = pos2(0.0, 0.0);
        let stroke_w = STROKE_SCALE;
        let height_factor = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let height = height_factor * grid_size;

        let top_point = pos2(pos.x + stroke_w * 0.5, pos.y + stroke_w * 0.5);
        let bottom_point = pos2(pos.x + stroke_w * 0.5, pos.y + height - stroke_w * 0.5);
        let left_curve_strength = 1.0;
        let left_control = pos2(
            pos.x + grid_size * left_curve_strength,
            pos.y + height / 2.0,
        );

        let n_curve_points = match lod_level {
            LodLevel::Max => 30,
            LodLevel::Mid => 5,
            LodLevel::Min => 2,
        }; // Number of points per curve segment
        result.push(Self::get_or_left_curve(
            top_point,
            bottom_point,
            left_control,
            n_curve_points,
        ));

        result
    }

    //
    // *** Nand gate ***
    //
    fn get_nand_gate_dimension_raw(n_inputs: usize) -> (i32, i32) {
        Self::get_and_gate_dimension_raw(n_inputs)
    }

    fn get_nand_gate_dock_cell_raw(
        connection_id: Id,
        n_inputs: usize,
        primitive_pos: &GridPos,
    ) -> GridPos {
        Self::get_and_gate_dock_cell_raw(connection_id, n_inputs, primitive_pos)
    }

    fn get_nand_gate_connection_position_raw(connection_id: Id, n_inputs: usize) -> Pos2 {
        Self::get_and_gate_connection_position_raw(connection_id, n_inputs)
    }

    fn get_circle_points(center: Pos2, radius: f32, lod_level: LodLevel) -> Vec<Pos2> {
        let n_circle_points = match lod_level {
            LodLevel::Max => 40,
            LodLevel::Mid => 6,
            LodLevel::Min => 4,
        };
        let mut circle_points: Vec<Pos2> = Vec::with_capacity(n_circle_points);
        for i in 0..n_circle_points {
            let angle = (i as f32 / n_circle_points as f32) * TAU;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            circle_points.push(Pos2::new(x, y));
        }
        circle_points
    }

    fn get_nand_gate_polygons_points_raw(n_inputs: usize, lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        let stroke_w = STROKE_SCALE;
        let height = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let radius_x = 1.0 - stroke_w / 2.0;
        let radius_y = height as f32 / 2.0 - stroke_w / 2.0;
        let center = pos2(1.5, height / 2.0);
        vec![
            Self::get_and_gate_shape_points(
                stroke_w, radius_x, radius_y, center, height, lod_level,
            ),
            Self::get_circle_points(center + vec2(radius_x, 0.0), 0.25, lod_level),
        ]
    }

    fn get_nand_gate_lines_raw(n_inputs: usize) -> Vec<Vec<Pos2>> {
        let height = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        };
        let stroke_w = STROKE_SCALE;
        let radius_x = 1.0 - stroke_w / 2.0;
        let center = pos2(1.5, height / 2.0);
        return vec![vec![center + vec2(radius_x, 0.0), pos2(3.0, height / 2.0)]];
    }

    //
    // *** Mux ***
    //
    fn get_mux_dimension_raw(n_inputs: usize) -> (i32, i32) {
        let w = if n_inputs > 2 { 2 } else { 1 };
        let h = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as i32
        } else {
            n_inputs as i32
        };
        return (w, h);
    }

    fn get_mux_dock_cell_raw(
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
        } else if connection_id == n_inputs {
            let (w, h) = Self::get_mux_dimension_raw(n_inputs);
            *primitive_pos + grid_pos(w, h / 2)
        } else {
            let (w, h) = Self::get_mux_dimension_raw(n_inputs);
            if w == 1 {
                *primitive_pos + grid_pos(0, h)
            } else {
                *primitive_pos + grid_pos(1, h)
            }
        }
    }

    fn get_mux_connection_position_raw(connection_id: Id, n_inputs: usize) -> Pos2 {
        let (w, h) = Self::get_mux_dimension_raw(n_inputs);
        if connection_id < n_inputs {
            if n_inputs % 2 == 0 {
                pos2(0.0, (2 * connection_id) as f32 + 0.5)
            } else {
                pos2(0.0, connection_id as f32 + 0.5)
            }
        } else if connection_id == n_inputs {
            pos2(w as f32, h as f32 / 2.0)
        } else {
            if w == 1 {
                pos2(0.5, h as f32 - 0.25)
            } else {
                pos2(1.5, h as f32 - 0.75)
            }
        }
    }

    fn get_mux_polygon_points_raw(n_inputs: usize) -> Vec<Pos2> {
        let (w, h) = Self::get_mux_dimension_raw(n_inputs);
        let stroke_ofs = STROKE_SCALE * 0.5;
        return vec![
            pos2(stroke_ofs, stroke_ofs),
            pos2(w as f32 - stroke_ofs, stroke_ofs + 0.5 * w as f32),
            pos2(
                w as f32 - stroke_ofs,
                h as f32 - 0.5 * w as f32 - stroke_ofs,
            ),
            pos2(stroke_ofs, h as f32 - stroke_ofs),
        ];
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

    fn get_input_connection_position_raw(_connection_id: Id) -> Pos2 {
        pos2(2.0, 0.5)
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

    fn get_output_connection_position_raw(_connection_id: Id) -> Pos2 {
        pos2(0.0, 0.5)
    }

    fn get_output_lines_raw() -> Vec<Vec<Pos2>> {
        vec![vec![pos2(0.0, 0.5), pos2(0.5, 0.5)]]
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

    fn get_not_polygons_points_raw(lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        let stroke_w = STROKE_SCALE;
        let grid_size = 1.0;
        let p0 = pos2(
            grid_size * 0.5 + stroke_w * 0.5,
            grid_size * 0.5 + stroke_w * 0.5,
        );
        let p1 = pos2(2.5 * grid_size - stroke_w * 0.5, grid_size * 1.5);
        let p2 = pos2(
            grid_size * 0.5 + stroke_w * 0.5,
            2.5 * grid_size - stroke_w * 0.5,
        );
        return vec![
            vec![p0, p1, p2],
            Self::get_circle_points(p1, grid_size * 0.25, lod_level),
        ];
    }

    fn get_not_connection_position_raw(connection_id: Id) -> Pos2 {
        pos2(if connection_id == 0 { 0.0 } else { 3.0 }, 1.5)
    }

    fn get_not_lines_raw() -> Vec<Vec<Pos2>> {
        let grid_size = 1.0;
        vec![
            vec![
                pos2(0.0, grid_size * 1.5),
                pos2(0.5 * grid_size, grid_size * 1.5),
            ],
            vec![
                pos2(2.5 * grid_size, grid_size * 1.5),
                pos2(3.0 * grid_size, grid_size * 1.5),
            ],
        ]
    }

    //
    // *** Common ***
    //
    pub fn get_connections_number(&self) -> usize {
        match self {
            Self::And(n_inputs) => *n_inputs + 1,
            Self::Or(n_inputs) => *n_inputs + 1,
            Self::Xor(n_inputs) => *n_inputs + 1,
            Self::Nand(n_inputs) => *n_inputs + 1,
            Self::Mux(n_inputs) => *n_inputs + 2,
            Self::Not => 2,
            Self::Input => 1,
            Self::Output => 1,
            Self::Point => 1,
        }
    }

    fn get_dimension_raw(&self) -> (i32, i32) {
        match self {
            Self::And(n_inputs) => Self::get_and_gate_dimension_raw(*n_inputs),
            Self::Or(n_inputs) => Self::get_or_gate_dimension_raw(*n_inputs),
            Self::Xor(n_inputs) => Self::get_xor_gate_dimension_raw(*n_inputs),
            Self::Nand(n_inputs) => Self::get_nand_gate_dimension_raw(*n_inputs),
            Self::Not => (3, 3),
            Self::Input => (2, 1),
            Self::Output => (2, 1),
            Self::Mux(n_inputs) => Self::get_mux_dimension_raw(*n_inputs),
            Self::Point => (1, 1),
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
            Self::Xor(n_inputs) => {
                Self::get_xor_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
            Self::Nand(n_inputs) => {
                Self::get_nand_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
            Self::Mux(n_inputs) => {
                Self::get_mux_dock_cell_raw(connection_id, *n_inputs, primitive_pos)
            }
            Self::Not => Self::get_not_dock_cell_raw(connection_id, primitive_pos),
            Self::Input => Self::get_input_dock_cell_raw(primitive_pos),
            Self::Output => Self::get_output_dock_cell_raw(primitive_pos),
            Self::Point => *primitive_pos,
        }
    }

    fn get_connection_position_raw(&self, connection_id: Id) -> Pos2 {
        match self {
            Self::And(n_inputs) => {
                Self::get_and_gate_connection_position_raw(connection_id, *n_inputs)
            }
            Self::Or(n_inputs) => {
                Self::get_or_gate_connection_position_raw(connection_id, *n_inputs)
            }
            Self::Xor(n_inputs) => {
                Self::get_xor_gate_connection_position_raw(connection_id, *n_inputs)
            }
            Self::Nand(n_inputs) => {
                Self::get_nand_gate_connection_position_raw(connection_id, *n_inputs)
            }
            Self::Not => Self::get_not_connection_position_raw(connection_id),
            Self::Mux(n_inputs) => Self::get_mux_connection_position_raw(connection_id, *n_inputs),
            Self::Input => Self::get_input_connection_position_raw(connection_id),
            Self::Output => Self::get_output_connection_position_raw(connection_id),
            Self::Point => pos2(0.5, 0.5),
        }
    }

    fn get_polygons_points_raw(&self, lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        match self {
            Self::And(n_inputs) => {
                vec![Self::get_and_gate_polygon_points_raw(*n_inputs, lod_level)]
            }
            Self::Or(n_inputs) => vec![Self::get_or_gate_polygon_points_raw(*n_inputs, lod_level)],
            Self::Xor(n_inputs) => {
                vec![Self::get_xor_gate_polygon_points_raw(*n_inputs, lod_level)]
            }
            Self::Nand(n_inputs) => Self::get_nand_gate_polygons_points_raw(*n_inputs, lod_level),
            Self::Mux(n_inputs) => vec![Self::get_mux_polygon_points_raw(*n_inputs)],
            Self::Input => vec![Self::get_input_polygon_points_raw()],
            Self::Output => vec![Self::get_output_polygon_points_raw()],
            Self::Not => Self::get_not_polygons_points_raw(lod_level),
            Self::Point => vec![],
        }
    }

    fn get_lines(&self, lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        match self {
            Self::Or(n_inputs) => Self::get_or_gate_lines_raw(*n_inputs),
            Self::Xor(n_inputs) => Self::get_xor_gate_lines_raw(*n_inputs, lod_level),
            Self::Nand(n_inputs) => Self::get_nand_gate_lines_raw(*n_inputs),
            Self::Output => Self::get_output_lines_raw(),
            Self::Not => Self::get_not_lines_raw(),
            _ => vec![],
        }
    }
}

thread_local! {
    static CACHE: LazyCell<RefCell<HashMap<(PrimitiveType, Rotation, LodLevel, Theme), Vec<Arc<Mesh>>>>> =
        LazyCell::new(|| RefCell::new(HashMap::new()));
}

fn apply_rotation_for_raw_points(points: &mut Vec<Pos2>, rotation: Rotation, raw_dim: (i32, i32)) {
    let dim = rotation.get_rotated_dim(raw_dim);
    let rot_ofs = match rotation {
        Rotation::ROT0 => vec2(0.0, 0.0),
        Rotation::ROT90 => vec2(dim.0 as f32, 0.0),
        Rotation::ROT180 => vec2(dim.0 as f32, dim.1 as f32),
        Rotation::ROT270 => vec2(0.0, dim.1 as f32),
    };
    for point in points {
        *point = rotation.rotate_point(*point, pos2(0.0, 0.0)) + rot_ofs;
    }
}

fn get_cached_meshes(
    typ: PrimitiveType,
    rotation: Rotation,
    lod_level: LodLevel,
    theme: Theme,
) -> Vec<Arc<Mesh>> {
    CACHE.with(|cell| {
        let mut map = cell.borrow_mut();
        if let Some(result) = map.get(&(typ, rotation, lod_level, theme)) {
            return result.clone();
        }
        let mut polygons_points = typ.get_polygons_points_raw(lod_level);
        let mut result = Vec::with_capacity(polygons_points.len());
        for points in &mut polygons_points {
            apply_rotation_for_raw_points(points, rotation, typ.get_dimension_raw());
            let mesh = tesselate_polygon(
                points,
                theme.get_fill_color(),
                lod_level != LodLevel::Min || theme == Theme::Light, // Do not optimize stroke on light theme
                theme.get_stroke_color(),
                STROKE_SCALE,
            );
            let arc = Arc::new(mesh);
            result.push(arc);
        }
        let result_cloned = result.clone();
        map.insert((typ.clone(), rotation, lod_level, theme), result);
        return result_cloned;
    })
}
