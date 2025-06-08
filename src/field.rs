

use eframe::egui_glow::painter;
use egui::{epaint::TextShape, pos2, vec2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Vec2};

use crate::primitives::{self, grid_pos, grid_rect, Component, Connection, ConnectionAlign, GridBD, GridBDConnectionPoint, GridPos, Net, Port, Unit};

pub enum GridType {
    Points,
    Cells
}


pub struct FieldState {
    pub scale: f32,
    pub offset: Vec2,
    pub grid_size: f32,
    pub rect: Rect,
    pub label_font: FontId,
    pub label_visible: bool,
    pub cursor_pos: Option<Pos2>,
}

impl FieldState {
    pub fn grid_to_screen(&self, grid_pos: &GridPos) -> Pos2 {
        return pos2(self.rect.left() + self.offset.x + grid_pos.x as f32 * self.grid_size, self.rect.top()  + self.offset.y + grid_pos.y as f32 * self.grid_size);
    }

    pub fn screen_to_grid(&self, screen_pos: Pos2) -> GridPos {
        let grid_x = (screen_pos.x - self.rect.left() - self.offset.x) / self.grid_size;
        let grid_y = (screen_pos.y - self.rect.top() - self.offset.y) / self.grid_size;

        GridPos {x: grid_x.floor() as i32, y: grid_y.floor() as i32}
    }
}

pub struct Field {
    pub state: FieldState,
    pub grid_type: GridType,
    pub grid_db: GridBD,
    pub connection_builder: ConnectionBuilder,
}

impl Field {
    // TODO: Move to settings
    pub const BASE_GRID_SIZE: f32 = 10.0;
    pub const MIN_SCALE: f32 = 0.1;
    pub const MAX_SCALE: f32 = 100.0;
    pub const MAX_FONT_SIZE: f32 = 32.0;
    pub const POINT_MIN_SCALE: f32 = 2.0;
    pub const GRID_MIN_SCALE: f32 = 0.5;
    pub const MIN_DISPLAY_TEXT_SIZE: f32 = 3.0;
    pub const LED_LEVEL0_SCALE: f32 = 0.5;

    pub fn new() -> Self {
        let scale = (Self::MAX_SCALE/10.0).max(Self::MIN_SCALE);
        let mut db = GridBD::new();
        let mut cnt = 0;
        for i in 0..100 {
            for j in 0..100 {
                cnt += 1;
                let unit = Unit {
                    id: cnt,
                    name: "АМОГУС".to_owned(),
                    pos: grid_pos(10 * i, 10 * j),
                    width: 5,
                    height: 6,
                    ports: vec![
                        Port{
                            inner_cell:grid_pos(0, 3),
                            align:ConnectionAlign::LEFT,
                            name: "vld".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 4),
                            align:ConnectionAlign::LEFT,
                            name: "data1".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 5),
                            align:ConnectionAlign::LEFT,
                            name: "data2".to_owned(),
                        },

                        Port{
                            inner_cell:grid_pos(4, 1),
                            align:ConnectionAlign::RIGHT,
                            name: "vld".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(4, 2),
                            align:ConnectionAlign::RIGHT,
                            name: "data1".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(4, 3),
                            align:ConnectionAlign::RIGHT,
                            name: "data2".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(2, 0),
                            align:ConnectionAlign::TOP,
                            name: "info".to_owned(),
                        }
                ]};
                db.add_component(Component::Unit(unit));
            }
        }
        Self {
            state : FieldState {
                scale: scale,
                grid_size: Self::BASE_GRID_SIZE * scale,
                offset: Vec2::default(),
                rect : Rect { min: Pos2::default(), max: Pos2::default() },
                label_font: FontId::monospace((Self::BASE_GRID_SIZE * scale * 0.5).min(Self::MAX_FONT_SIZE)),
                label_visible: Self::BASE_GRID_SIZE * scale * 0.5 >= Self::MIN_DISPLAY_TEXT_SIZE,
                cursor_pos: None
            },
            grid_type : GridType::Cells,
            grid_db: db,
            connection_builder: ConnectionBuilder::new()
        }

    }

    fn generate_cells(&self, grid_pos: GridPos, width: i32, height: i32, color: Color32) -> Shape {
        let rect = Rect::from_min_size(self.state.grid_to_screen(&grid_pos), vec2(self.state.grid_size * width as f32, self.state.grid_size * height as f32));
        Shape::rect_filled(rect.intersect(self.state.rect), 0.0, color)
    }


    fn display_grid(&self, ui: &mut egui::Ui, response: &Response) {
        let delta_x = if self.state.offset.x >= 0.0 {self.state.offset.x % self.state.grid_size} else {self.state.grid_size - (self.state.offset.x.abs()% self.state.grid_size)} ;
        let delta_y = if self.state.offset.y >= 0.0 {self.state.offset.y % self.state.grid_size} else {self.state.grid_size - (self.state.offset.y.abs()% self.state.grid_size)} ;

        let stroke = Stroke::new(0.1, Color32::WHITE);
        let mut shapes = vec![];

        match self.grid_type {
            GridType::Cells => {
                if Self::GRID_MIN_SCALE < self.state.scale {
                    for i in 0..((self.state.rect.width() - delta_x) / self.state.grid_size) as i32 + 1 {
                        let x = self.state.rect.left() + delta_x + i as f32 * self.state.grid_size;
                        shapes.push(Shape::line_segment([pos2(x, self.state.rect.top()), pos2(x, self.state.rect.bottom())], stroke));
                    }

                    for j in 0..((self.state.rect.height() - delta_y) / self.state.grid_size) as i32 + 1 {
                        let y = self.state.rect.top() + delta_y + j as f32 * self.state.grid_size;
                        shapes.push(Shape::line_segment([pos2(self.state.rect.left(), y), pos2(self.state.rect.right(), y)], stroke));
                    }
                }
            }
            GridType::Points => {
                if Self::POINT_MIN_SCALE < self.state.scale {
                    let vertical_lines = ((self.state.rect.width() - delta_x) / self.state.grid_size) as i32 + 1;
                    let horizontal_lines = ((self.state.rect.height() - delta_y) / self.state.grid_size) as i32 + 1;

                    for i in 0..vertical_lines {
                        for j in 0..horizontal_lines {
                            let x = self.state.rect.left() + delta_x + i as f32 * self.state.grid_size;
                            let y = self.state.rect.top() + delta_y + j as f32 * self.state.grid_size;
                            shapes.push(Shape::circle_filled(pos2(x, y), 0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
                        }
                    }
                }
            }
        }

        if response.hovered() {
            if let Some(pos) = response.hover_pos() {
                let grid_cell_pos = self.state.screen_to_grid(pos);
                shapes.push(self.generate_cells(grid_cell_pos, 1, 1, Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
            }
        }

        shapes.push(Shape::rect_stroke(self.state.rect, 0.0, Stroke::new(0.1, Color32::WHITE), StrokeKind::Outside));

        ui.painter().extend(shapes);
    }



    // Update state of field
    fn refresh(&mut self, ui: &mut egui::Ui, response: &Response) {
        if response.dragged() {
            self.state.offset += response.drag_delta();
        }

        if response.hovered() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let new_scale = (self.state.scale * zoom_delta).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let zoom_factor = new_scale / self.state.scale;

            if let Some(hover_pos) = response.hover_pos() {
                self.state.offset = (self.state.offset - hover_pos.to_vec2()) * zoom_factor + hover_pos.to_vec2();
            }

            self.state.scale = new_scale;
            if zoom_delta != 1.0 {
                self.state.grid_size = Self::BASE_GRID_SIZE * self.state.scale;
                let label_text_size = (self.state.grid_size * 0.5).min(Self::MAX_FONT_SIZE);
                self.state.label_visible = label_text_size > Self::MIN_DISPLAY_TEXT_SIZE;
                self.state.label_font = FontId::monospace(label_text_size);
            }
        }
        self.state.cursor_pos = response.hover_pos();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.state.rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(self.state.rect, Sense::drag().union(Sense::all()));
        self.refresh(ui, &response);
        self.display_grid(ui, &response);

        let grid_rect = grid_rect(0, self.state.screen_to_grid(self.state.rect.min),  self.state.screen_to_grid(self.state.rect.max));
        self.grid_db.get_visible_components(&grid_rect).iter().for_each(|u| { // Return iter
            u.display(&self.state, &ui.painter().with_clip_rect(self.state.rect));
        });

        let nets: Vec<&Net> = self.grid_db.get_visible_nets(&grid_rect);
        nets.iter().for_each(|n| {n.display(&self.grid_db, &self.state, &ui.painter().with_clip_rect(self.state.rect));});
        if let Some(con) = self.grid_db.get_hovered_connection(&self.state) {
            let comp =  self.grid_db.get_component(&con.component_id).unwrap();
            let connection = comp.get_connection(con.connection_id).unwrap();
            connection.highlight(&self.state, &comp.get_position(), ui.painter());
            if response.clicked() {
                self.connection_builder.toggle(&mut self.grid_db, &self.state, con);
            }
        }
        self.connection_builder.draw(&self.grid_db, &self.state, ui.painter());
    }
}


enum ConnectionBuilderState {
    IDLE,
    ACTIVE
}

struct ConnectionBuilder {
    state : ConnectionBuilderState,
    point : Option<GridBDConnectionPoint>
}

impl ConnectionBuilder {
    fn new() -> Self {
        Self {
            state: ConnectionBuilderState::IDLE,
            point: None
        }
    }

    fn toggle(&mut self, bd: &mut GridBD, state: &FieldState, point: GridBDConnectionPoint) {
        println!("Toggle");
        match self.state {
            ConnectionBuilderState::IDLE => {
                self.point = Some(point);
                self.state = ConnectionBuilderState::ACTIVE;
            }
            ConnectionBuilderState::ACTIVE => {
                let old_point = self.point.clone().unwrap();
                self.point = None;
                self.state = ConnectionBuilderState::IDLE;

                // TODO: refactor it
                if let Some(comp1) = bd.get_component(&old_point.component_id) {
                    if let Some(con1) = comp1.get_connection(old_point.connection_id) {
                        if let Some(comp2) = bd.get_component(&point.component_id) {
                            if let Some(con2) = comp2.get_connection(point.connection_id) {

                                let p1 = con1.get_pos(comp1) + con1.get_grid_connection_offset();
                                let p2: GridPos = con2.get_pos(comp2) + con2.get_grid_connection_offset();

                                let net = Net {
                                    id: bd.nets.len(), // fixme
                                    start_point: old_point,
                                    end_point: point,
                                    points: bd.find_net_path(p1, p2)
                                };
                                bd.add_net(net);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw(&mut self, bd: &GridBD, state: &FieldState, painter: &egui::Painter)  {
        if let Some(point) = &self.point {
            //painter.line_segment(points, stroke)
            if let Some(comp) = bd.get_component(&point.component_id) {
                if let Some(con) = comp.get_connection(point.connection_id) {
                    let p1 = con.center(&comp.get_position(), state);
                    if let Some(p2) = state.cursor_pos {
                        let mut points = vec![p1];
                        points.extend(bd.find_net_path(state.screen_to_grid(p1), state.screen_to_grid(p2)).iter().map(|g|{state.grid_to_screen(&g) + vec2(state.grid_size*0.5, state.grid_size*0.5)}));
                        points.push(p2);
                        painter.line(points, Stroke::new(state.grid_size * 0.3, Color32::DARK_GRAY));
                    }
                }
            }
        }
    }
}
