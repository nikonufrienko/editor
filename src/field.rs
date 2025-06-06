

use egui::{epaint::TextShape, pos2, vec2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Vec2};

use crate::primitives::{grid_pos, grid_rect, Component, GridDB, GridPos, Net, Port, PortAlign, Unit};

pub enum GridType {
    Points,
    Cells
}

pub struct Field {
    pub scale: f32,
    pub offset: Vec2,
    pub grid_type: GridType,
    pub rect: Rect,
    pub label_font: FontId,
    pub grid_size: f32,
    pub label_visible: bool,
    pub grid_db: GridDB
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
        let mut db = GridDB::new();
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
                            inner_cell:grid_pos(0, 1),
                            align:PortAlign::LEFT,
                            name: "clk".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 2),
                            align:PortAlign::LEFT,
                            name: "reset".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 3),
                            align:PortAlign::LEFT,
                            name: "vld".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 4),
                            align:PortAlign::LEFT,
                            name: "data1".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(0, 5),
                            align:PortAlign::LEFT,
                            name: "data2".to_owned(),
                        },

                        Port{
                            inner_cell:grid_pos(4, 1),
                            align:PortAlign::RIGHT,
                            name: "vld".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(4, 2),
                            align:PortAlign::RIGHT,
                            name: "data1".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(4, 3),
                            align:PortAlign::RIGHT,
                            name: "data2".to_owned(),
                        },
                        Port{
                            inner_cell:grid_pos(2, 0),
                            align:PortAlign::TOP,
                            name: "info".to_owned(),
                        }
                ]};
                db.add_component(Component::Unit(unit));
            }
        }
        Self {
            scale: scale,
            grid_size: Self::BASE_GRID_SIZE * scale,
            offset: Vec2::default(),
            grid_type : GridType::Cells,
            rect : Rect { min: Pos2::default(), max: Pos2::default() },
            label_font: FontId::monospace((Self::BASE_GRID_SIZE * scale * 0.5).min(Self::MAX_FONT_SIZE)),
            label_visible: Self::BASE_GRID_SIZE * scale * 0.5 >= Self::MIN_DISPLAY_TEXT_SIZE,
            grid_db: db
        }

    }

    pub fn grid_to_screen(&self, grid_pos: GridPos) -> Pos2 {
        return pos2(self.rect.left() + self.offset.x + grid_pos.x as f32 * self.grid_size, self.rect.top()  + self.offset.y + grid_pos.y as f32 * self.grid_size);
    }

    fn generate_cells(&self, grid_pos: GridPos, width: i32, height: i32, color: Color32) -> Shape {
        let rect = Rect::from_min_size(self.grid_to_screen(grid_pos), vec2(self.grid_size * width as f32, self.grid_size * height as f32));
        Shape::rect_filled(rect.intersect(self.rect), 0.0, color)
    }

    fn screen_to_grid(&self, screen_pos: Pos2) -> GridPos {
        let grid_x = (screen_pos.x - self.rect.left() - self.offset.x) / self.grid_size;
        let grid_y = (screen_pos.y - self.rect.top() - self.offset.y) / self.grid_size;

        GridPos {x: grid_x.floor() as i32, y: grid_y.floor() as i32}
    }

    fn display_grid(&self, ui: &mut egui::Ui, response: &Response) {
        let delta_x = if self.offset.x >= 0.0 {self.offset.x % self.grid_size} else {self.grid_size - (self.offset.x.abs()% self.grid_size)} ;
        let delta_y = if self.offset.y >= 0.0 {self.offset.y % self.grid_size} else {self.grid_size - (self.offset.y.abs()% self.grid_size)} ;

        let stroke = Stroke::new(0.1, Color32::WHITE);
        let mut shapes = vec![];

        match self.grid_type {
            GridType::Cells => {
                if Self::GRID_MIN_SCALE < self.scale {
                    for i in 0..((self.rect.width() - delta_x) / self.grid_size) as i32 + 1 {
                        let x = self.rect.left() + delta_x + i as f32 * self.grid_size;
                        shapes.push(Shape::line_segment([pos2(x, self.rect.top()), pos2(x, self.rect.bottom())], stroke));
                    }

                    for j in 0..((self.rect.height() - delta_y) / self.grid_size) as i32 + 1 {
                        let y = self.rect.top() + delta_y + j as f32 * self.grid_size;
                        shapes.push(Shape::line_segment([pos2(self.rect.left(), y), pos2(self.rect.right(), y)], stroke));
                    }
                }
            }
            GridType::Points => {
                if Self::POINT_MIN_SCALE < self.scale {
                    let vertical_lines = ((self.rect.width() - delta_x) / self.grid_size) as i32 + 1;
                    let horizontal_lines = ((self.rect.height() - delta_y) / self.grid_size) as i32 + 1;

                    for i in 0..vertical_lines {
                        for j in 0..horizontal_lines {
                            let x = self.rect.left() + delta_x + i as f32 * self.grid_size;
                            let y = self.rect.top() + delta_y + j as f32 * self.grid_size;
                            shapes.push(Shape::circle_filled(pos2(x, y), 0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
                        }
                    }
                }
            }
        }

        if response.hovered() {
            if let Some(pos) = response.hover_pos() {
                let grid_cell_pos = self.screen_to_grid(pos);
                shapes.push(self.generate_cells(grid_cell_pos, 1, 1, Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
            }
        }

        shapes.push(Shape::rect_stroke(self.rect, 0.0, Stroke::new(0.1, Color32::WHITE), StrokeKind::Outside));

        ui.painter().extend(shapes);
    }

    fn port_align_to_vec2(&self, align:&PortAlign) -> Vec2 {
        match align {
            PortAlign::LEFT => {vec2(0.5 * self.grid_size, 0.0)},
            PortAlign::RIGHT => {vec2(-0.5 * self.grid_size, 0.0)},
            PortAlign::TOP => {vec2(0.0, -0.5 * self.grid_size)},
            PortAlign::BOTTOM => {vec2(0.0, 0.5 * self.grid_size)},
        }
    }

    fn draw_net(&self, net: &Net, ui:&mut egui::Ui) {
        let grid_center_vec = vec2(0.5 * self.grid_size, 0.5 * self.grid_size);
        let points: Vec<Pos2> = net.points.iter().map(|p| {self.grid_to_screen(p.clone()) + grid_center_vec}).collect();
        let len = points.len();
        if len > 1 {
            let start_point = points[0] + self.port_align_to_vec2(&net.aligns.0);
            let end_point = points[len-1] + self.port_align_to_vec2(&net.aligns.1);
            let mut extended = vec![start_point];
            extended.extend(points);
            extended.push(end_point);
            ui.painter().with_clip_rect(self.rect).line(extended, Stroke::new(self.grid_size * 0.1, Color32::DARK_GRAY));
        }
    }

    fn refresh(&mut self, ui: &mut egui::Ui, response: &Response) {
        if response.dragged() {
            self.offset += response.drag_delta();
        }

        if response.hovered() {
            let zoom_delta = ui.input(|i| i.zoom_delta());
            let new_scale = (self.scale * zoom_delta).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let zoom_factor = new_scale / self.scale;

            if let Some(hover_pos) = response.hover_pos() {
                self.offset = (self.offset - hover_pos.to_vec2()) * zoom_factor + hover_pos.to_vec2();
            }
            self.scale = new_scale;
            if zoom_delta != 1.0 {
                self.grid_size = Self::BASE_GRID_SIZE * self.scale;
                let label_text_size = (self.grid_size * 0.5).min(Self::MAX_FONT_SIZE);
                self.label_visible = label_text_size > Self::MIN_DISPLAY_TEXT_SIZE;
                self.label_font = FontId::monospace(label_text_size);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(self.rect, Sense::drag().union(Sense::hover()));
        self.refresh(ui, &response);
        self.display_grid(ui, &response);

        let grid_rect = grid_rect(0, self.screen_to_grid(self.rect.min),  self.screen_to_grid(self.rect.max));
        let units = self.grid_db.get_visible_components(grid_rect);
        units.iter().for_each(|u| {
            u.display(self, ui);
        });
    }
}
