use std::{f32::consts::PI, sync::Arc};

use egui::{epaint::PathShape, pos2, vec2, Color32, Painter, Pos2, Shape, Stroke};
use lyon::{path::Path, tessellation::{FillOptions, FillTessellator, FillVertex, VertexBuffers}};
use serde::{Deserialize, Serialize};
use lyon::geom::{point, Point};

use crate::field::FieldState;

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
        
        // Получаем точки полигона и применяем вращение
        let raw_points = self.typ.get_polygon_points_raw(state, stroke_w, &self.pos);
        let points = self.apply_rotation(raw_points, state);
        
        // Создаем путь с помощью Lyon
        let mut builder = Path::builder();
        if let Some(first) = points.first() {
            builder.begin(point(first.x, first.y));
            for p in &points[1..] {
                builder.line_to(point(p.x, p.y));
            }
            builder.close();
        }
        let path = builder.build();
    
        // Триангулируем путь (используем правильные типы вершин)
        let mut geometry: VertexBuffers<egui::epaint::Vertex, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        
        tessellator.tessellate_path(
            &path,
            &FillOptions::default(),
            &mut lyon::tessellation::BuffersBuilder::new(
                &mut geometry,
                |vertex: FillVertex| {  // Используем FillVertex вместо Point
                    egui::epaint::Vertex {
                        pos: pos2(vertex.position().x, vertex.position().y),
                        uv: egui::epaint::WHITE_UV,
                        color: Color32::GRAY,
                    }
                },
            ),
        ).expect("Tessellation failed");
    
        // Создаем меш egui
        let mut mesh = egui::Mesh::default();
        mesh.vertices = geometry.vertices;
        mesh.indices = geometry.indices.iter().map(|i| *i as u32).collect();
        
        // Рисуем меш
        painter.add(egui::Shape::Mesh(Arc::new(mesh)));
        
        // Рисуем соединения (как раньше)
        (0..=self.typ.get_connections_number()).for_each(|i| {
            painter.circle_filled(
                self.apply_rotation(
                    vec![self.typ.get_connection_position_raw(i, state, &self.pos)],
                    state,
                )[0],
                state.grid_size * Self::CONNECTION_SCALE,
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
            (2, (2 * n_inputs - 1) as i32)
        } else {
            (2, n_inputs as i32)
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
        let center = pos + vec2(state.grid_size, height / 2.0);

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
        // Fixme
        if connection_id < n_inputs as Id {
            if n_inputs % 2 == 0 {
                *primitive_pos + grid_pos(-1, 2 * connection_id as i32)
            } else {
                *primitive_pos + grid_pos(-1, connection_id as i32)
            }
        } else {
            let raw_dim = Self::get_or_gate_dimension_raw(n_inputs);
            *primitive_pos + grid_pos(raw_dim.0, raw_dim.1 / 2)
        }
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
        let height = if n_inputs % 2 == 0 {
            (2 * n_inputs - 1) as f32
        } else {
            n_inputs as f32
        } * state.grid_size;
        let radius_x = state.grid_size - stroke_w / 2.0;
        let radius_y = height / 2.0 - stroke_w / 2.0;
        let pos = state.grid_to_screen(primitive_pos);
        let left_center = pos2(
            pos.x,
            pos.y + height / 2.0
        );
        let right_center = pos2(
            pos.x + state.grid_size,
            pos.y + height / 2.0
        );
        let n_points = 30;
        let mut points = Vec::new();
        // Левая дуга: выпуклая влево, снизу вверх
        for i in 0..=n_points {
            let angle = -PI/2.0 + PI * (i as f32 / n_points as f32);
            points.push(pos2(
                left_center.x + radius_x * angle.cos(), // минус для выпуклости влево
                left_center.y + radius_y * angle.sin()
            ));
        }
        // Правая дуга: выпуклая вправо, сверху вниз (в обратном порядке)
        for i in (0..=n_points).rev() {
            let angle = -PI/2.0 + PI * (i as f32 / n_points as f32);
            points.push(pos2(
                right_center.x + radius_x * angle.cos(),
                right_center.y + radius_y * angle.sin()
            ));
        }
        points
    }

    //
    // *** Common ***
    //
    
    fn get_dimension_raw(&self) -> (i32, i32) {
        match self {
            Self::And(n_inputs) => Self::get_and_gate_dimension_raw(*n_inputs),
            Self::Or(n_inputs) => Self::get_or_gate_dimension_raw(*n_inputs)
        }
    }
    
    fn get_dock_cell_raw(&self, connection_id: Id, primitive_pos: &GridPos) -> GridPos {
        // TODO: remove Option here
        match self {
            Self::And(n_inputs) =>  Self::get_and_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos),
            Self::Or(n_inputs) =>  Self::get_or_gate_dock_cell_raw(connection_id, *n_inputs, primitive_pos),
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
    ) -> Vec<Pos2> {
        match self {
            Self::And(n_inputs) => {
                Self::get_and_gate_polygon_points_raw(state, stroke_w, *n_inputs, primitive_pos)
            }
            Self::Or(n_inputs) => {
                Self::get_or_gate_polygon_points_raw(state, stroke_w, *n_inputs, primitive_pos)
            }
        }
    }
}
