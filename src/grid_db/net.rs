use egui::{pos2, vec2, Color32, Mesh, Painter, Pos2, Rect, Stroke, Theme, Vec2};
use serde::{Deserialize, Serialize};

use crate::{field::{FieldState, SVG_DUMMY_STATE}, grid_db::{mesh_line, svg_line, ComponentColor, GridDB, GridDBConnectionPoint, GridPos, Id}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Net {
    pub start_point: GridDBConnectionPoint,
    pub end_point: GridDBConnectionPoint,
    pub points: Vec<GridPos>,
}

impl Net {
    pub fn get_segments(&self, net_id: Id) -> Vec<NetSegment> {
        let mut result = Vec::with_capacity(self.points.len() - 1);
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
        db: &GridDB,
    ) -> Option<String> {
        if self.points.is_empty() {
            return Some(String::new());
        }
        let offset_vec2 = vec2(offset.x as f32, offset.y as f32);
        let first_point = db
            .get_component(&self.start_point.component_id)?
            .get_connection_position(self.start_point.connection_id, &SVG_DUMMY_STATE)?
            + offset_vec2;
        let last_point = db
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


pub struct NetSegment {
    pub inner_id: Id, // ID of segment in net
    pub net_id: Id,   // ID of net
    pub pos1: GridPos,
    pub pos2: GridPos,
    con1: Option<GridDBConnectionPoint>, // if segment
    con2: Option<GridDBConnectionPoint>, // Second position
}

impl NetSegment {
    pub fn new(
        inner_id: Id,
        net_id: Id,
        pos1: GridPos,
        pos2: GridPos,
        con1: Option<GridDBConnectionPoint>,
        con2: Option<GridDBConnectionPoint>,
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

    pub fn get_mesh(&self, db: &GridDB, state: &FieldState, theme: Theme) -> Mesh {
        let w = (state.grid_size * 0.1).max(1.0);
        let ofs = Vec2::new(0.5 * state.grid_size, 0.5 * state.grid_size);
        let color = theme.get_stroke_color();

        let p1 = state.grid_to_screen(&self.pos1) + ofs;
        let p2 = state.grid_to_screen(&self.pos2) + ofs;

        let mut pts = vec![p1, p2];

        if let Some(cp) = &self.con1 {
            if let Some(comp) = db.get_component(&cp.component_id) {
                pts.insert(
                    0,
                    comp.get_connection_position(cp.connection_id, state)
                        .unwrap(),
                );
            }
        }

        if let Some(cp) = &self.con2 {
            if let Some(comp) = db.get_component(&cp.component_id) {
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

#[derive(Clone, Copy)]
pub enum NetAction {
    RemoveNet,
    InsertPoint,
}

impl NetAction {
    pub const ACTIONS: &[Self] = &[Self::InsertPoint, Self::RemoveNet];

    pub fn draw(&self, painter: &Painter, rect: Rect, selected: bool) {
        let visuals = &painter.ctx().style().visuals;
        let stroke = if selected {
            Stroke::new(rect.height() / 8.0, visuals.strong_text_color())
        } else {
            Stroke::new(rect.height() / 8.0, visuals.text_color())
        };
        let scaled = rect.scale_from_center(0.6);
        match self {
            Self::RemoveNet => {
                painter.line_segment([scaled.left_top(), scaled.right_bottom()], stroke);
                painter.line_segment([scaled.left_bottom(), scaled.right_top()], stroke);
            },
            Self::InsertPoint => {
                painter.circle_filled(scaled.center(), stroke.width * 1.3, stroke.color);
                painter.line_segment([scaled.left_center(), scaled.right_center()], stroke);            }
        }
    }
}
