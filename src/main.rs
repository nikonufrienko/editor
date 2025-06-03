use std::{default, result};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{accesskit::TextAlign, epaint::TextShape, pos2, response, text::Fonts, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Vec2};

fn main()  {
    let options = eframe::NativeOptions::default();
    _ = eframe::run_native(
        "My App",
        options,
        Box::new(|_| Ok(Box::new(EditorApp::new()))),
    );
}


#[derive(Clone, Copy)]
struct GridPos {
    x : i32,
    y : i32
}

fn grid_pos(x:i32, y:i32) -> GridPos {
    GridPos{x,y}
}

enum PortAlign {LEFT, RIGHT, TOP, BOTTOM}

struct Port {
    inner_cell: GridPos,
    align: PortAlign,
    name: String
}

struct Unit {
    name: String,
    pos: GridPos,
    width: i32,
    height: i32,
    ports: Vec<Port>
}

enum GridType {
    Points,
    Cells
}


struct Field {
    scale: f32,
    offset: Vec2,
    grid_type: GridType,
    rect: Rect,
    label_font: FontId,
    grid_size: f32
}


impl Field {
    const BASE_GRID_SIZE: f32 = 10.0;
    const MIN_SCALE: f32 = 0.1;
    const MAX_SCALE: f32 = 100.0;
    const MAX_FONT_SIZE: f32 = 64.0;
    const POINT_MIN_SCALE: f32 = 2.0;

    fn new() -> Self {
        let scale = (Self::MAX_SCALE/10.0).max(Self::MIN_SCALE);
        Self {
            scale: scale,
            grid_size: Self::BASE_GRID_SIZE * scale,
            offset: Vec2::default(),
            grid_type : GridType::Cells,
            rect : Rect { min: Pos2::default(), max: Pos2::default() },
            label_font: FontId::monospace((Self::BASE_GRID_SIZE * scale * 0.5).min(Self::MAX_FONT_SIZE))
        }
    }

    fn grid_to_screen(&self, grid_pos: GridPos) -> Pos2 {
        return pos2(self.rect.left() + self.offset.x + grid_pos.x as f32 * self.grid_size, self.rect.top()  + self.offset.y + grid_pos.y as f32 * self.grid_size);
    }

    fn generate_cells(&self, grid_pos: GridPos, width: i32, height: i32, color: Color32) -> Shape {

        let x = self.rect.left() + self.offset.x + grid_pos.x as f32 * self.grid_size;
        let y = self.rect.top()  + self.offset.y + grid_pos.y as f32 * self.grid_size;

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
                for i in 0..((self.rect.width() - delta_x) / self.grid_size) as i32 + 1 {
                    let x = self.rect.left() + delta_x + i as f32 * self.grid_size;
                    shapes.push(Shape::line_segment([pos2(x, self.rect.top()), pos2(x, self.rect.bottom())], stroke));
                }

                for j in 0..((self.rect.height() - delta_y) / self.grid_size) as i32 + 1 {
                    let y = self.rect.top() + delta_y + j as f32 * self.grid_size;
                    shapes.push(Shape::line_segment([pos2(self.rect.left(), y), pos2(self.rect.right(), y)], stroke));
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

    fn display_unit(&self, unit: &Unit, ui:&mut egui::Ui) {
        let rect = Rect::from_min_size(self.grid_to_screen(unit.pos), vec2(self.grid_size * unit.width as f32, self.grid_size * unit.height as f32));
        let painter = ui.painter().with_clip_rect(self.rect);
        painter.rect_filled(rect.intersect(self.rect), 0.5 * self.scale, Color32::GRAY);
        painter.rect_stroke(rect.intersect(self.rect), 0.5 * self.scale, Stroke::new(1.0 * self.scale, Color32::DARK_GRAY),StrokeKind::Outside);
        for port in &unit.ports {
            let mut pos = self.grid_to_screen(GridPos { x: unit.pos.x + port.inner_cell.x, y: unit.pos.y + port.inner_cell.y });
            let text_pos;
            let anchor;
            match port.align {
                PortAlign::LEFT => {
                    pos.y += self.grid_size * 0.5;
                    text_pos = pos2(pos.x + self.grid_size * 0.5, pos.y);
                    anchor = Align2::LEFT_CENTER;
                }
                PortAlign::RIGHT => {
                    pos.y += self.grid_size * 0.5;
                    pos.x += self.grid_size;
                    text_pos = pos2(pos.x - self.grid_size * 0.5, pos.y);

                    anchor = Align2::RIGHT_CENTER;

                }
                _ => {anchor = Align2::LEFT_CENTER; text_pos = pos;}
            }
            painter.circle_filled(pos, self.grid_size/6.0, Color32::GRAY);
            painter.circle_stroke(pos, self.grid_size/6.0, Stroke::new(1.0 * self.scale, Color32::DARK_GRAY));
            painter.text(text_pos, anchor, port.name.clone(), self.label_font.clone(), Color32::WHITE);
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
                self.label_font = FontId::monospace((self.grid_size * 0.5).min(Self::MAX_FONT_SIZE));
            }
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        let start = Instant::now(); // Засекаем время
        self.rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(self.rect, Sense::drag().union(Sense::hover()));
        self.refresh(ui, &response);
        let duration: Duration = start.elapsed(); // Получаем прошедшее время
        if duration > Duration::from_millis(10) {
            println!("Функция выполнялась: {:?}", duration);
        }
        self.display_grid(ui, &response);

        self.display_unit(&Unit {
            name: "АМОГУС".to_owned(),
            pos: grid_pos(0,0),
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
                }
        ]}, ui);

    }
}


struct EditorApp {
    field: Field,
    grid_sel: bool ,
}

impl EditorApp {
    fn new() -> Self {
        EditorApp {field: Field::new(), grid_sel:true}
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.toggle_value(&mut self.grid_sel, "Сетка");
            if self.grid_sel {
                self.field.grid_type = GridType::Cells;
            } else {
                self.field.grid_type = GridType::Points;
            }
            self.field.show(ui);
        });
    }
}
