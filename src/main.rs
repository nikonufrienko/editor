
use eframe::egui;
use egui::{vec2, Id, LayerId, Rect, Sense, Stroke};

use crate::{
    field::{Field, GridType}, file_managment::FileManager, locale::{get_system_default_locale, SUPPORTED_LOCALES}, preview::PreviewPanel
};

mod component_lib;
mod field;
mod grid_db;
mod preview;
mod locale;
mod file_managment;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
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
    preview_window: PreviewPanel,
    //errors: Arc<Mutex<Vec<String>>>, // Errors
    locale: locale::LocaleType,
    file_manager: FileManager,
}

impl EditorApp {
    fn new() -> Self {
        EditorApp {
            field: Field::new(),
            preview_window: PreviewPanel::new(),
            //errors: Arc::new(Mutex::new(vec![])),
            locale: get_system_default_locale(),
            file_manager: FileManager::new(),
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let locale: &'static locale::Locale = self.locale.locale();
        let foreground: LayerId = LayerId::new(egui::Order::Foreground, Id::new("foreground"));
        self.file_manager.update(ctx, locale, &mut self.field.grid_db);
        egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(locale.file, |ui| {
                        if ui.button(locale.open).clicked() {
                            self.file_manager.open_file(locale);
                            ui.close_menu();
                        }
                        if ui.button(locale.save).clicked() {
                            self.file_manager.save_file(&self.field.grid_db);
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(locale.view, |ui| {
                        ui.menu_button(locale.grid, |ui| {
                            ui.radio_value(&mut self.field.grid_type, GridType::Cells, locale.cells);
                            ui.radio_value(&mut self.field.grid_type, GridType::Dots, locale.dots);
                        });
                        ui.menu_button(locale.language, |ui| {
                            for other_local in SUPPORTED_LOCALES {
                                ui.radio_value(&mut self.locale, *other_local, other_local.locale().locale_name);
                            }
                        });
                    });
                    panel_left_switch(ui, &mut self.preview_window.is_expanded);
                });
            });
        });


        self.field.set_external_drag_resp(self.preview_window.show(
            ctx,
            foreground,
            self.field.state.scale,
        ));


        egui::CentralPanel::default().show(ctx, |ui| {
            self.field.show(ui);
        });
    }
}


fn panel_left_switch(ui:&mut egui::Ui, is_expanded: &mut bool) {
    let h = ui.available_height();
    ui.add_space((ui.available_width()- h * 2.0).max(0.0));
    let (rect, resp) = ui.allocate_exact_size(vec2(1.5*h, h), Sense::click());
    let visuals = ui.visuals();
    let color = if *is_expanded {visuals.text_color()} else {visuals.weak_text_color()};
    let stroke = Stroke::new(h * 0.075, color);
    let paint_rect = rect.scale_from_center(0.8);
    let r = paint_rect.height() * 0.06;
    ui.painter().rect_stroke(paint_rect, r, stroke, egui::StrokeKind::Inside);
    ui.painter().rect_filled(Rect::from_min_size(paint_rect.min, vec2(paint_rect.height() *0.5, paint_rect.height())), r, color);

    //ui.painter().rect_filled(rect, visuals.menu_corner_radius / 2.0, stroke, egui::StrokeKind::Middle);

    if resp.clicked() {
        *is_expanded = !*is_expanded;
    }
}
