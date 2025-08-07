use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::ops::Add;
use std::{
    cell::{LazyCell, RefCell},
    f32::consts::PI,
    sync::Arc,
    vec,
};

use egui::{Align2, RichText, Theme};
use egui::{Color32, Mesh, Painter, Pos2, Shape, Stroke, emath::TSTransform, pos2, vec2};
use serde::{Deserialize, Serialize};

use crate::grid_db::{ComponentColor, STROKE_SCALE, show_text_with_debounce, svg_single_line_text};
use crate::locale::Locale;

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
    pub fn to_radians(&self) -> f32 {
        match self {
            Rotation::ROT0 => 0.0,
            Rotation::ROT90 => FRAC_PI_2,
            Rotation::ROT180 => PI,
            Rotation::ROT270 => PI + FRAC_PI_2,
        }
    }

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

impl Add for Rotation {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        fn to_u8(a: Rotation) -> u8 {
            match a {
                Rotation::ROT0 => 0,
                Rotation::ROT90 => 1,
                Rotation::ROT180 => 2,
                Rotation::ROT270 => 3,
            }
        }

        fn from_u8(a: u8) -> Rotation {
            match a {
                0 => Rotation::ROT0,
                1 => Rotation::ROT90,
                2 => Rotation::ROT180,
                _ => Rotation::ROT270,
            }
        }

        return from_u8((to_u8(self) + to_u8(rhs)) % 4);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PrimitiveComponent {
    pub typ: PrimitiveType,
    pub pos: GridPos,
    pub rotation: Rotation,
}

impl PrimitiveComponent {
    pub fn get_actions(&self) -> &'static [ComponentAction] {
        if self.typ.is_customizable() {
            &[
                ComponentAction::RotateDown,
                ComponentAction::RotateUp,
                ComponentAction::Customize,
                ComponentAction::Remove,
            ]
        } else {
            &[
                ComponentAction::RotateDown,
                ComponentAction::RotateUp,
                ComponentAction::Remove,
            ]
        }
    }

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
        Some(self.apply_rotation_grid_pos(self.typ.get_dock_cell_raw(connection_id) + self.pos))
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
            let radius = match self.typ {
                PrimitiveType::Point => state.grid_size * 0.2,
                _ => state.grid_size * Self::CONNECTION_SCALE,
            };
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

        // Draw text labels:
        if state.lod_level() == LodLevel::Max {
            for (pos, text, rotation, anchor) in self.typ.get_text_labels() {
                show_text_with_debounce(
                    self.apply_rotation(pos * state.grid_size + screen_pos, state),
                    text,
                    state,
                    painter,
                    None,
                    rotation + self.rotation,
                    anchor,
                );
            }
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
            apply_rotation_for_raw_points(
                &mut raw_line,
                self.rotation,
                self.typ.get_dimension_raw(),
            );
            for p in &mut *raw_line {
                *p = (*p + raw_offset) * scale;
            }
            result.push_str(&(svg_line(&raw_line, stroke_color, stroke_w) + &"\n"));
        }

        // Ports:
        let radius = match self.typ {
            PrimitiveType::Point => scale * 0.2,
            _ => scale * Self::CONNECTION_SCALE,
        };
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

        // Text labels:
        let font_size = 0.5 * scale;
        for (pos, text, rotation, anchor) in self.typ.get_text_labels() {
            result.push_str(
                &(svg_single_line_text(
                    text,
                    (self.apply_rotation(pos + pos_vec2, &SVG_DUMMY_STATE) + offset_vec2) * scale,
                    font_size,
                    rotation + self.rotation,
                    theme,
                    anchor,
                ) + &"\n"),
            );
        }

        result
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct DFFParams {
    pub has_enable: bool,
    pub has_async_reset: bool,
    pub has_sync_reset: bool,

    pub async_reset_inverted: bool,
    pub sync_reset_inverted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum PPort {
    // Common ports:
    Input(usize),
    Output(usize),

    // Mux:
    Select,

    // Adder ports:
    Cin,
    Cout,

    // DFF ports:
    D,
    Q,
    AsyncReset,
    SyncReset,
    Enable,
    Clk,
}

impl PPort {
    /// Returns the additional ports of dff (beyond D, CLK, Q) based on parameters
    fn dff_additional_ports(params: &DFFParams) -> &'static [Option<Self>; 3] {
        // Pre-define all possible combinations for quick lookup
        static PORT_COMBINATIONS: [[Option<PPort>; 3]; 8] = [
            // 0: no additional ports
            [None, None, None], // None placeholder
            // 1: sync reset only
            [Some(PPort::SyncReset), None, None],
            // 2: async reset only
            [Some(PPort::AsyncReset), None, None],
            // 3: sync + async reset
            [Some(PPort::SyncReset), Some(PPort::AsyncReset), None],
            // 4: enable only
            [Some(PPort::Enable), None, None],
            // 5: sync reset + enable
            [Some(PPort::SyncReset), Some(PPort::Enable), None],
            // 6: async reset + enable
            [Some(PPort::AsyncReset), Some(PPort::Enable), None],
            // 7: all additional ports
            [
                Some(PPort::SyncReset),
                Some(PPort::AsyncReset),
                Some(PPort::Enable),
            ],
        ];

        let index = (params.has_enable as usize) << 2
            | (params.has_async_reset as usize) << 1
            | params.has_sync_reset as usize;

        &PORT_COMBINATIONS[index]
    }

    /// Converts a connection ID to a port type
    fn from_id(typ: &PrimitiveType, id: usize) -> Option<Self> {
        match typ {
            PrimitiveType::And(n_inputs)
            | PrimitiveType::Or(n_inputs)
            | PrimitiveType::Xor(n_inputs)
            | PrimitiveType::Nand(n_inputs) => match id {
                0 => Some(Self::Output(0)),
                _ => {
                    if id <= *n_inputs {
                        Some(Self::Input(id - 1))
                    } else {
                        None
                    }
                }
            },

            PrimitiveType::Mux(n_inputs) => match id {
                0 => Some(Self::Output(0)),
                1 => Some(Self::Select),
                _ => {
                    if id <= *n_inputs + 1 {
                        Some(Self::Input(id - 2))
                    } else {
                        None
                    }
                }
            },
            PrimitiveType::DFF(params) => match id {
                0 => Some(Self::Clk),
                1 => Some(Self::D),
                2 => Some(Self::Q),
                3..=5 => *Self::dff_additional_ports(params).get(id - 3)?,
                _ => None,
            },
            PrimitiveType::Not => match id {
                0 => Some(Self::Input(0)),
                1 => Some(Self::Output(0)),
                _ => None,
            },
            PrimitiveType::Point => match id {
                0 => Some(Self::Output(0)),
                _ => None,
            },
            PrimitiveType::Input => match id {
                0 => Some(Self::Output(0)),
                _ => None,
            },
            PrimitiveType::Output => match id {
                0 => Some(Self::Input(0)),
                _ => None,
            },
            PrimitiveType::Comparator(_) => match id {
                0 => Some(Self::Input(0)),
                1 => Some(Self::Input(1)),
                2 => Some(Self::Output(0)),
                _ => None,
            },
            PrimitiveType::Adder { cin, cout } => match id {
                0 => Some(Self::Input(0)),
                1 => Some(Self::Input(1)),
                2 => Some(Self::Output(0)),
                3 => {
                    if *cin {
                        Some(Self::Cin)
                    } else if *cout {
                        Some(Self::Cout)
                    } else {
                        None
                    }
                }
                4 => {
                    if *cin && *cout {
                        Some(Self::Cout)
                    } else {
                        None
                    }
                }
                _ => None,
            },
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum ComparisonType {
    /// Equal (==)
    EQ,
    /// Less than (<)
    LT,
    /// Less than or equal (<)
    LTE,
    /// Greater than (>)
    GT,
    /// Greater than or equal (>=)
    GTE,
}

impl ComparisonType {
    const TYPES: &[ComparisonType] = &[Self::EQ, Self::LT, Self::LTE, Self::GT, Self::GTE];

    fn to_str(self) -> &'static str {
        match self {
            ComparisonType::EQ => "==",
            ComparisonType::LT => "<",
            ComparisonType::LTE => "<=",
            ComparisonType::GT => ">",
            ComparisonType::GTE => ">=",
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Hash, PartialEq, Eq)]
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

    // I/O:
    Input,
    Output,

    // Arithmetic:
    Comparator(ComparisonType),
    Adder { cin: bool, cout: bool },

    // D-type flip-flop:
    DFF(DFFParams),
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

    fn get_and_gate_dock_cell_raw(port: PPort, n_inputs: usize) -> GridPos {
        match port {
            PPort::Output(0) => {
                let raw_dim = Self::get_and_gate_dimension_raw(n_inputs);
                grid_pos(raw_dim.0, raw_dim.1 / 2)
            }
            PPort::Input(inp_id) => {
                if n_inputs % 2 == 0 {
                    grid_pos(-1, 2 * inp_id as i32)
                } else {
                    grid_pos(-1, inp_id as i32)
                }
            }
            _ => panic!("Unexpected port"),
        }
    }

    fn get_and_gate_connection_position_raw(port: PPort, n_inputs: usize) -> Pos2 {
        match port {
            PPort::Output(0) => {
                let (w, h) = Self::get_and_gate_dimension_raw(n_inputs);
                pos2(w as f32, h as f32 / 2.0)
            }
            PPort::Input(inp_id) => {
                if n_inputs % 2 == 0 {
                    pos2(0.0, (2 * inp_id) as f32 + 0.5)
                } else {
                    pos2(0.0, inp_id as f32 + 0.5)
                }
            }
            _ => panic!("Unexpected port"),
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

    fn get_or_gate_dock_cell_raw(port: PPort, n_inputs: usize) -> GridPos {
        Self::get_and_gate_dock_cell_raw(port, n_inputs)
    }

    fn get_or_gate_connection_position_raw(port: PPort, n_inputs: usize) -> Pos2 {
        Self::get_and_gate_connection_position_raw(port, n_inputs)
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
            let p0 = Self::get_or_gate_connection_position_raw(PPort::Input(i), n_inputs);
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

    fn get_xor_gate_dock_cell_raw(port: PPort, n_inputs: usize) -> GridPos {
        Self::get_and_gate_dock_cell_raw(port, n_inputs)
    }

    fn get_xor_gate_connection_position_raw(port: PPort, n_inputs: usize) -> Pos2 {
        Self::get_and_gate_connection_position_raw(port, n_inputs)
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
        let height: f32 = height_factor * grid_size;

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

    fn get_nand_gate_dock_cell_raw(port: PPort, n_inputs: usize) -> GridPos {
        Self::get_and_gate_dock_cell_raw(port, n_inputs)
    }

    fn get_nand_gate_connection_position_raw(port: PPort, n_inputs: usize) -> Pos2 {
        Self::get_and_gate_connection_position_raw(port, n_inputs)
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
        let w = if n_inputs > 3 { 2 } else { 1 };
        let h = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as i32
        } else {
            n_inputs as i32
        };
        return (w, h);
    }

    fn get_mux_dock_cell_raw(port: PPort, n_inputs: usize) -> GridPos {
        match port {
            PPort::Output(0) => {
                let (w, h) = Self::get_mux_dimension_raw(n_inputs);
                grid_pos(w, h / 2)
            }
            PPort::Select => {
                let (w, h) = Self::get_mux_dimension_raw(n_inputs);
                if w == 1 {
                    grid_pos(0, h)
                } else {
                    grid_pos(1, h)
                }
            }
            PPort::Input(inp_id) => {
                if n_inputs % 2 == 0 {
                    grid_pos(-1, 2 * inp_id as i32)
                } else {
                    grid_pos(-1, inp_id as i32)
                }
            }
            _ => panic!("Unexpected port"),
        }
    }

    fn get_mux_connection_position_raw(port: PPort, n_inputs: usize) -> Pos2 {
        match port {
            PPort::Output(0) => {
                // Output:
                let (w, h) = Self::get_mux_dimension_raw(n_inputs);
                pos2(w as f32, h as f32 / 2.0)
            }
            PPort::Select => {
                let (w, h) = Self::get_mux_dimension_raw(n_inputs);
                if w == 1 {
                    pos2(0.5, h as f32 - 0.25)
                } else {
                    pos2(1.5, h as f32 - 0.75)
                }
            }
            PPort::Input(inp_id) => {
                // Inputs:
                if n_inputs % 2 == 0 {
                    pos2(0.0, (2 * inp_id) as f32 + 0.5)
                } else {
                    pos2(0.0, inp_id as f32 + 0.5)
                }
            }
            _ => panic!("Unexpected port"),
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
    fn get_input_dock_cell_raw() -> GridPos {
        grid_pos(2, 0)
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

    fn get_input_connection_position_raw(_port: PPort) -> Pos2 {
        pos2(2.0, 0.5)
    }

    //
    // *** Output ***
    //
    fn get_output_dock_cell_raw() -> GridPos {
        grid_pos(-1, 0)
    }

    fn get_output_polygon_points_raw() -> Vec<Pos2> {
        Self::get_input_polygon_points_raw()
    }

    fn get_output_connection_position_raw(_port: PPort) -> Pos2 {
        pos2(0.0, 0.5)
    }

    fn get_output_lines_raw() -> Vec<Vec<Pos2>> {
        vec![vec![pos2(0.0, 0.5), pos2(0.5, 0.5)]]
    }

    //
    // *** Not ***
    //
    fn get_not_dock_cell_raw(port: PPort) -> GridPos {
        match port {
            PPort::Input(0) => grid_pos(-1, 1),
            PPort::Output(0) => grid_pos(3, 1),
            _ => panic!("Unexpected port"),
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

    fn get_not_connection_position_raw(port: PPort) -> Pos2 {
        match port {
            PPort::Input(0) => pos2(0.0, 1.5),
            PPort::Output(0) => pos2(3.0, 1.5),
            _ => panic!("Unexpected port"),
        }
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
    // *** Comparator ***
    //
    const CMP_DIMENSION: (i32, i32) = (3, 3);
    const CMP_N_CONNECTIONS: usize = 3;

    fn get_cmp_dock_cell_raw(port: PPort) -> GridPos {
        match port {
            PPort::Input(0) => grid_pos(-1, 0),
            PPort::Input(1) => grid_pos(-1, 2),
            PPort::Output(0) => grid_pos(3, 1),
            _ => panic!("Unexpected port"),
        }
    }

    fn get_cmp_connection_position_raw(port: PPort) -> Pos2 {
        match port {
            PPort::Input(0) => pos2(0.0, 0.5),
            PPort::Input(1) => pos2(0.0, 2.5),
            PPort::Output(0) => pos2(3.0, 1.5),
            _ => panic!("Unexpected port"),
        }
    }

    fn get_cmp_polygons_points_raw(lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        vec![Self::get_circle_points(pos2(1.5, 1.5), 1.2, lod_level)]
    }

    fn get_cmp_lines_raw() -> Vec<Vec<Pos2>> {
        vec![
            vec![pos2(0.0, 0.5), pos2(0.85, 0.5)],
            vec![pos2(0.0, 2.5), pos2(0.85, 2.5)],
            vec![pos2(3.0, 1.5), pos2(2.7, 1.5)],
        ]
    }

    fn get_cmp_text_labels(
        comparison_type: &ComparisonType,
    ) -> Vec<(Pos2, String, Rotation, Align2)> {
        vec![(
            pos2(1.5, 1.5),
            comparison_type.to_str().to_owned(),
            Rotation::ROT0,
            Align2::CENTER_CENTER,
        )]
    }

    //
    // *** Adder ***
    //
    fn get_adder_dimension_raw(cin: bool, _cout: bool) -> (i32, i32) {
        if cin { (3, 4) } else { (3, 3) }
    }

    fn get_adder_connections_number(cin: bool, cout: bool) -> usize {
        return 3 + if cin { 1 } else { 0 } + if cout { 1 } else { 0 };
    }

    fn get_adder_dock_cell_raw(port: PPort, cin: bool) -> GridPos {
        let y_offs = if cin { 1 } else { 0 };

        match port {
            PPort::Input(0) => grid_pos(-1, 0 + y_offs),
            PPort::Input(1) => grid_pos(-1, 2 + y_offs),
            PPort::Output(0) => grid_pos(3, 1 + y_offs),
            PPort::Cin => grid_pos(-1, 0),
            PPort::Cout => grid_pos(3, 2 + y_offs),
            _ => panic!("Unexpected port"),
        }
    }

    fn get_adder_connection_position_raw(port: PPort, cin: bool) -> Pos2 {
        let y_offs = if cin { 1.0 } else { 0.0 };

        match port {
            PPort::Input(0) => pos2(0.0, 0.5 + y_offs),
            PPort::Input(1) => pos2(0.0, 2.5 + y_offs),
            PPort::Output(0) => pos2(3.0, 1.5 + y_offs),
            PPort::Cin => pos2(0.0, 0.5),
            PPort::Cout => pos2(3.0, 2.5 + y_offs),
            _ => panic!("Unexpected port"),
        }
    }

    fn get_adder_polygons_points_raw(lod_level: LodLevel, cin: bool) -> Vec<Vec<Pos2>> {
        let y_offs = if cin { 1.0 } else { 0.0 };

        vec![Self::get_circle_points(
            pos2(1.5, 1.5 + y_offs),
            1.2,
            lod_level,
        )]
    }

    fn get_adder_lines_raw(cin: bool, cout: bool) -> Vec<Vec<Pos2>> {
        let y_offs = if cin { 1.0 } else { 0.0 };
        let mut result = vec![
            vec![pos2(0.0, 0.5 + y_offs), pos2(0.85, 0.5 + y_offs)],
            vec![pos2(0.0, 2.5 + y_offs), pos2(0.85, 2.5 + y_offs)],
            vec![pos2(3.0, 1.5 + y_offs), pos2(2.7, 1.5 + y_offs)],
        ];
        if cin {
            result.push(vec![pos2(0.0, 0.5), pos2(0.5, 0.5), pos2(1.3, 1.3)]);
        }
        if cout {
            result.push(vec![
                pos2(2.5, 2.0 + y_offs),
                pos2(2.75, 2.5 + y_offs),
                pos2(3.0, 2.5 + y_offs),
            ]);
        }
        result
    }

    fn get_adder_text_labels(cin: bool, cout: bool) -> Vec<(Pos2, String, Rotation, Align2)> {
        let y_offs = if cin { 1.0 } else { 0.0 };

        let mut result = vec![(
            pos2(1.5, 1.5 + y_offs),
            "+".to_owned(),
            Rotation::ROT0,
            Align2::CENTER_CENTER,
        )];
        if cin {
            result.push((
                pos2(0.0, -0.15),
                "cin".to_owned(),
                Rotation::ROT0,
                Align2::LEFT_TOP,
            ));
        }
        if cout {
            result.push((
                pos2(3.0, 2.5 + y_offs),
                "cout".to_owned(),
                Rotation::ROT0,
                Align2::RIGHT_TOP,
            ));
        }
        result
    }

    //
    // *** DFF (D-type flip-flop) ***
    //
    const DFF_DIMENSION: (i32, i32) = (5, 5);

    fn get_dff_connections_number(params: &DFFParams) -> usize {
        3 + if params.has_sync_reset { 1 } else { 0 }
            + if params.has_async_reset { 1 } else { 0 }
            + if params.has_enable { 1 } else { 0 }
    }

    fn get_dff_dock_cell_raw(port: PPort) -> GridPos {
        match port {
            PPort::D => grid_pos(0, 1),
            PPort::Clk => grid_pos(0, 3),
            PPort::Q => grid_pos(4, 2),
            PPort::AsyncReset => grid_pos(2, 0),
            PPort::SyncReset => grid_pos(0, 2),
            PPort::Enable => grid_pos(0, 4),
            _ => panic!("Unexpected port"),
        }
    }

    fn get_dff_polygons_points_raw(params: &DFFParams, lod_level: LodLevel) -> Vec<Vec<Pos2>> {
        let (width, height) = Self::DFF_DIMENSION;
        let mut result = Vec::with_capacity(4);
        result.extend([
            vec![
                pos2(1.05, 1.05),
                pos2(width as f32 - 1.05, 1.05),
                pos2(width as f32 - 1.05, height as f32 - 0.05),
                pos2(1.05, height as f32 - 0.05),
            ],
            vec![pos2(1.05, 3.0), pos2(2.0, 3.5), pos2(1.05, 4.0)],
        ]);
        if params.has_sync_reset && params.sync_reset_inverted {
            result.push(Self::get_circle_points(pos2(1.0, 2.5), 0.17, lod_level));
        }
        if params.has_async_reset && params.async_reset_inverted {
            result.push(Self::get_circle_points(pos2(2.5, 1.0), 0.17, lod_level));
        }
        result
    }

    fn get_dff_connection_position_raw(port: PPort) -> Pos2 {
        match port {
            PPort::D => pos2(0.5, 1.5),
            PPort::Clk => pos2(0.5, 3.5),
            PPort::Q => pos2(4.5, 2.5),
            PPort::AsyncReset => pos2(2.5, 0.5),
            PPort::SyncReset => pos2(0.5, 2.5),
            PPort::Enable => pos2(0.5, 4.5),
            _ => panic!("Unexpected port"),
        }
    }

    fn get_dff_text_labels(params: &DFFParams) -> Vec<(Pos2, String, Rotation, Align2)> {
        let n_connections = Self::get_dff_connections_number(params);
        let mut result: Vec<(Pos2, String, Rotation, Align2)> =
            Vec::with_capacity(n_connections - 1);
        result.extend([
            (
                pos2(1.25, 1.25),
                "D".into(),
                Rotation::ROT0,
                Align2::LEFT_TOP,
            ),
            (
                pos2(3.45, 2.25),
                "Q".into(),
                Rotation::ROT0,
                Align2::LEFT_TOP,
            ),
        ]);
        if params.has_enable {
            result.push((
                pos2(1.25, 4.25),
                "EN".into(),
                Rotation::ROT0,
                Align2::LEFT_TOP,
            ));
        }
        if params.has_async_reset {
            result.push((
                pos2(1.9, 1.1),
                "ARST".to_string()
                    + if params.async_reset_inverted {
                        "_N"
                    } else {
                        ""
                    },
                Rotation::ROT0,
                Align2::LEFT_TOP,
            ));
        }
        if params.has_sync_reset {
            result.push((
                pos2(1.25, 2.25),
                "RST".to_string() + if params.sync_reset_inverted { "_N" } else { "" },
                Rotation::ROT0,
                Align2::LEFT_TOP,
            ));
        }
        result
    }

    fn get_dff_lines_raw(params: &DFFParams) -> Vec<Vec<Pos2>> {
        let n_connections = Self::get_dff_connections_number(params);
        let mut result = Vec::with_capacity(n_connections);

        result.extend([
            vec![pos2(0.5, 1.5), pos2(1.0, 1.5)], // D
            vec![pos2(0.5, 3.5), pos2(1.0, 3.5)], // Clk
            vec![pos2(4.5, 2.5), pos2(3.5, 2.5)], // Q
        ]);
        if params.has_enable {
            result.push(vec![pos2(0.5, 4.5), pos2(1.0, 4.5)]);
        }
        if params.has_sync_reset {
            result.push(vec![pos2(0.5, 2.5), pos2(1.0, 2.5)]);
        }
        if params.has_async_reset {
            result.push(vec![pos2(2.5, 0.5), pos2(2.5, 1.0)]);
        }
        result
    }

    //
    // *** Common ***
    //

    fn get_port_type(&self, id: Id) -> Option<PPort> {
        PPort::from_id(self, id)
    }

    pub fn get_connections_number(&self) -> usize {
        match self {
            Self::And(n_inputs) => *n_inputs + 1,
            Self::Or(n_inputs) => *n_inputs + 1,
            Self::Xor(n_inputs) => *n_inputs + 1,
            Self::Nand(n_inputs) => *n_inputs + 1,
            Self::Not => 2,
            Self::Mux(n_inputs) => *n_inputs + 2,
            Self::Comparator(_) => Self::CMP_N_CONNECTIONS,
            Self::Adder { cin, cout } => Self::get_adder_connections_number(*cin, *cout),
            Self::DFF(params) => Self::get_dff_connections_number(params),
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
            Self::Mux(n_inputs) => Self::get_mux_dimension_raw(*n_inputs),
            Self::Comparator(_) => Self::CMP_DIMENSION,
            Self::Adder { cin, cout } => Self::get_adder_dimension_raw(*cin, *cout),
            Self::DFF(_) => Self::DFF_DIMENSION,
            Self::Input => (2, 1),
            Self::Output => (2, 1),
            Self::Point => (1, 1),
        }
    }

    fn get_dock_cell_raw(&self, connection_id: Id) -> GridPos {
        let port = self.get_port_type(connection_id).unwrap(); // Check that port is exist
        match self {
            Self::And(n_inputs) => Self::get_and_gate_dock_cell_raw(port, *n_inputs),
            Self::Or(n_inputs) => Self::get_or_gate_dock_cell_raw(port, *n_inputs),
            Self::Xor(n_inputs) => Self::get_xor_gate_dock_cell_raw(port, *n_inputs),
            Self::Nand(n_inputs) => Self::get_nand_gate_dock_cell_raw(port, *n_inputs),
            Self::Not => Self::get_not_dock_cell_raw(port),
            Self::Mux(n_inputs) => Self::get_mux_dock_cell_raw(port, *n_inputs),
            Self::Comparator(_) => Self::get_cmp_dock_cell_raw(port),
            Self::Adder { cin, cout: _ } => Self::get_adder_dock_cell_raw(port, *cin),
            Self::DFF(_) => Self::get_dff_dock_cell_raw(port),
            Self::Input => Self::get_input_dock_cell_raw(),
            Self::Output => Self::get_output_dock_cell_raw(),
            Self::Point => grid_pos(0, 0),
        }
    }

    fn get_connection_position_raw(&self, connection_id: Id) -> Pos2 {
        let port = self.get_port_type(connection_id).unwrap(); // Check that port is exist
        match self {
            Self::And(n_inputs) => Self::get_and_gate_connection_position_raw(port, *n_inputs),
            Self::Or(n_inputs) => Self::get_or_gate_connection_position_raw(port, *n_inputs),
            Self::Xor(n_inputs) => Self::get_xor_gate_connection_position_raw(port, *n_inputs),
            Self::Nand(n_inputs) => Self::get_nand_gate_connection_position_raw(port, *n_inputs),
            Self::Not => Self::get_not_connection_position_raw(port),
            Self::Mux(n_inputs) => Self::get_mux_connection_position_raw(port, *n_inputs),
            Self::Comparator(_) => Self::get_cmp_connection_position_raw(port),
            Self::Adder { cin, cout: _ } => Self::get_adder_connection_position_raw(port, *cin),
            Self::DFF(_) => Self::get_dff_connection_position_raw(port),
            Self::Input => Self::get_input_connection_position_raw(port),
            Self::Output => Self::get_output_connection_position_raw(port),
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
            Self::Input => vec![Self::get_input_polygon_points_raw()],
            Self::Output => vec![Self::get_output_polygon_points_raw()],
            Self::Not => Self::get_not_polygons_points_raw(lod_level),
            Self::Comparator(_) => Self::get_cmp_polygons_points_raw(lod_level),
            Self::Adder { cin, cout: _ } => Self::get_adder_polygons_points_raw(lod_level, *cin),
            Self::Mux(n_inputs) => vec![Self::get_mux_polygon_points_raw(*n_inputs)],
            Self::DFF(params) => Self::get_dff_polygons_points_raw(params, lod_level),
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
            Self::DFF(params) => Self::get_dff_lines_raw(params),
            Self::Comparator(_) => Self::get_cmp_lines_raw(),
            Self::Adder { cin, cout } => Self::get_adder_lines_raw(*cin, *cout),
            _ => vec![],
        }
    }

    fn get_text_labels(&self) -> Vec<(Pos2, String, Rotation, Align2)> {
        match self {
            Self::DFF(params) => Self::get_dff_text_labels(params),
            Self::Comparator(typ) => Self::get_cmp_text_labels(typ),
            Self::Adder { cin, cout } => Self::get_adder_text_labels(*cin, *cout),
            _ => vec![],
        }
    }

    pub fn is_customizable(&self) -> bool {
        match self {
            Self::And(_)
            | Self::Or(_)
            | Self::Xor(_)
            | Self::Nand(_)
            | Self::Mux(_)
            | Self::DFF(_)
            | Self::Adder { cin: _, cout: _ }
            | Self::Comparator(_) => true,
            Self::Not | Self::Input | Self::Output | Self::Point => false,
        }
    }

    /// Returns a list of connection permutations.
    pub fn get_connections_diff(&self, other: &Self) -> HashMap<Id, Option<Id>> {
        let mut self_port_map = HashMap::new();
        let self_n_ports = self.get_connections_number();
        for id in 0..self_n_ports {
            self_port_map.insert(PPort::from_id(self, id).unwrap(), id);
        }

        let mut other_port_map = HashMap::new();
        let other_n_ports = other.get_connections_number();
        for id in 0..other_n_ports {
            other_port_map.insert(PPort::from_id(other, id).unwrap(), id);
        }

        let mut result = HashMap::new();
        for (port, id) in &self_port_map {
            let other_id = other_port_map.get(port).cloned();
            if other_id != Some(*id) {
                result.insert(*id, other_id);
            }
        }
        return result;
    }

    pub fn show_customization_panel(&mut self, ui: &mut egui::Ui, locale: &'static Locale) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        match self {
            Self::And(n_inputs)
            | Self::Or(n_inputs)
            | Self::Xor(n_inputs)
            | Self::Nand(n_inputs)
            | Self::Mux(n_inputs) => {
                let mut buffer = n_inputs.to_string();
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", locale.inputs_number));

                    if ui
                        .add(egui::TextEdit::singleline(&mut buffer).desired_width(50.0))
                        .changed()
                    {
                        match buffer.parse::<usize>() {
                            Ok(num) => {
                                if num < 100 && num >= 2 {
                                    *n_inputs = num
                                }
                            }
                            _ => {
                                if buffer.is_empty() {
                                    *n_inputs = 2
                                }
                            }
                        }
                    }
                    if ui.button(RichText::new("+").monospace()).clicked() && *n_inputs < 100 {
                        *n_inputs += 1;
                    }
                    if ui.button(RichText::new("-").monospace()).clicked() && *n_inputs > 2 {
                        *n_inputs -= 1;
                    }
                });
            }
            Self::DFF(params) => {
                ui.checkbox(&mut params.has_sync_reset, locale.sync_reset);
                if params.has_sync_reset {
                    ui.checkbox(&mut params.sync_reset_inverted, locale.sync_reset_inverted);
                }
                ui.checkbox(&mut params.has_async_reset, locale.async_reset);
                if params.has_async_reset {
                    ui.checkbox(
                        &mut params.async_reset_inverted,
                        locale.async_reset_inverted,
                    );
                }
                ui.checkbox(&mut params.has_enable, locale.enable_signal);
            }
            Self::Adder { cin, cout } => {
                ui.checkbox(cin, "cin");
                ui.checkbox(cout, "cout");
            }
            Self::Comparator(curr_typ) => {
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", locale.type_));
                    ui.menu_button(curr_typ.to_str(), |ui: &mut egui::Ui| {
                        for typ in ComparisonType::TYPES {
                            ui.selectable_value(curr_typ, *typ, typ.to_str());
                        }
                    });
                });
            }
            _ => {}
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
