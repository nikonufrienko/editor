use std::{ops::Deref, sync::Arc};

use eframe::egui;
use egui::{mutex::Mutex, Id, LayerId};
use serde_json::error;

use crate::{
    field::{Field, GridType},
    preview_window::PreviewWindow,
};

mod component_lib;
mod field;
mod grid_db;
mod preview_window;
mod locale;

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
    grid_sel: bool,
    preview_window: PreviewWindow,
    //errors: Arc<Mutex<Vec<String>>>, // Errors
    locale: locale::Locale
}

impl EditorApp {
    fn new() -> Self {
        EditorApp {
            field: Field::new(),
            grid_sel: true,
            preview_window: PreviewWindow::new(),
            //errors: Arc::new(Mutex::new(vec![])),
            locale: locale::RU_LOCALE
        }
    }

   /*
    fn show_errors(&mut self, ctx: &egui::Context) {
        //let mut errors = self.errors.lock();
        if !errors.is_empty() {
            let mut errors_to_remove = vec![];
            for (i, error) in errors.iter().enumerate() {
                egui::Window::new("Ошибка!")
                .id(egui::Id::new(format!("error_window_{}", i))) // FIXME
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(error);
                    ui.separator();

                    if ui.button("OK").clicked() {
                        errors_to_remove.push(i);
                    }
                });
            }
            errors_to_remove.iter().for_each(|i| {errors.remove(*i);});
        }
    } */

    fn save_file(&self) {
        //let error_msg = self.locale.file_save_error;
        if let Some(data) = self.field.grid_db.dump_to_json() {
            //let errors = self.errors.clone();
            // Для нативных платформ
            #[cfg(not(target_arch = "wasm32"))]
            {
                smol::spawn(async move {
                    if let Some(file) = rfd::AsyncFileDialog::new()
                        .set_file_name("1.json")
                        .save_file()
                        .await
                    {
                        file.write(data.as_bytes()).await.ok();
                        //errors.lock().push(error_msg.into());
                    }
                }).detach();
            }

            #[cfg(target_arch = "wasm32")]
            {
                use eframe::wasm_bindgen::JsCast;
                use web_sys::{Blob, Url};

                // Создаем Blob с содержимым
                let blob = Blob::new_with_str_sequence(
                    &js_sys::Array::of1(&js_sys::JsString::from(data))
                ).unwrap();

                // Создаем URL для Blob
                let url = Url::create_object_url_with_blob(&blob).unwrap();

                // Создаем временную ссылку для скачивания
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let a = document.create_element("a").unwrap()
                    .dyn_into::<web_sys::HtmlAnchorElement>().unwrap();

                a.set_download("1.json");
                a.set_href(&url);
                //a.set_style("display", "none");

                document.body().unwrap().append_child(&a).unwrap();
                a.click();
                document.body().unwrap().remove_child(&a).unwrap();

                // Освобождаем ресурсы
                Url::revoke_object_url(&url).unwrap();
            }
        } else {
            // self.errors.lock().push(error_msg.into());
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
                        self.save_file();
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

        // let dropped_files = ctx.input(|input_state| input_state.raw.dropped_files.clone());

        //self.show_errors(ctx);
    }
}
