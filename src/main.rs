use eframe::egui;
use egui::{Id, LayerId};

use crate::{
    field::{Field, GridType},
    preview_window::PreviewWindow,
};

mod component_lib;
mod field;
mod grid_db;
mod preview_window;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    _ = eframe::run_native(
        "Editor",
        options,
        Box::new(|_| Ok(Box::new(EditorApp::new()))),
    );
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    //eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(EditorApp::new()))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

struct EditorApp {
    field: Field,
    grid_sel: bool,
    preview_window: PreviewWindow,
}

impl EditorApp {
    fn new() -> Self {
        EditorApp {
            field: Field::new(),
            grid_sel: true,
            preview_window: PreviewWindow::new(),
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let foreground: LayerId = LayerId::new(egui::Order::Foreground, Id::new("foreground"));
        egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Файл", |ui| {
                    if ui.button("Сохранить").clicked() {
                        // TODO:
                        ui.close_menu();
                    }
                });
            });
        });
        self.field.set_external_drag_resp(self.preview_window.show(
            ctx,
            foreground,
            self.field.state.scale,
        ));
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
