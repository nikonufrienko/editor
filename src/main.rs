#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{fmt::format, time::Instant};

use eframe::egui;
use egui::{vec2, CursorIcon, Id, KeyboardShortcut, LayerId, Modifiers, Rect, Sense, Stroke, Theme, Visuals};

use crate::{
    field::{Field, GridType},
    file_managment::FileManager,
    helpers::Helpers,
    locale::{SUPPORTED_LOCALES, get_system_default_locale},
    preview::PreviewPanel,
};

mod component_lib;
mod field;
mod file_managment;
mod grid_db;
mod helpers;
mod locale;
mod preview;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let options = eframe::NativeOptions {
        multisampling: 8,
        dithering: false,

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

    let web_options = eframe::WebOptions {
        dithering: false,
        ..Default::default()
    };

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
                Box::new(|_cc| Ok(Box::new(EditorApp::new()))),
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
    locale: locale::LocaleType,
    file_manager: FileManager,
    helpers: Helpers,
    file_name: String,
    theme : Theme,
    last_frame_time: Option<Instant>,
}

impl EditorApp {
    fn new() -> Self {
        EditorApp {
            field: Field::new(),
            preview_window: PreviewPanel::new(),
            locale: get_system_default_locale(),
            file_manager: FileManager::new(),
            helpers: Helpers::new(),
            file_name: String::new(),
            theme: Theme::Dark,
            last_frame_time: None,
        }
    }
}

const DUMMY_NAME: &'static str = "Untitled";
impl EditorApp {
    fn save(&mut self) {
        let dummy = &DUMMY_NAME.into();
        self.file_manager.save_file(
            &self.field.grid_db,
            if self.file_name.is_empty() {
                dummy
            } else {
                &self.file_name
            },
        );
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.theme {
            Theme::Dark => ctx.set_visuals(Visuals::dark()),
            Theme::Light => ctx.set_visuals(Visuals::light()),
        }
        let locale: &'static locale::Locale = self.locale.locale();
        let foreground: LayerId = LayerId::new(egui::Order::Foreground, Id::new("foreground"));
        self.file_manager
            .update(ctx, locale, &mut self.field.grid_db, &mut self.file_name);
        ctx.tessellation_options_mut(|options| options.feathering = false);
        egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button(locale.file, |ui| {
                        if ui.button(locale.open).clicked() {
                            self.file_manager.open_file(locale);
                            ui.close();
                        }
                        if ui.button(locale.save).clicked() {
                            self.save();
                            ui.close();
                        }
                        if ui.button(locale.export_to_svg).clicked() {
                            let dummy = &DUMMY_NAME.into();
                            self.file_manager.export_to_svg(
                                &self.field.grid_db,
                                if self.file_name.is_empty() {
                                    dummy
                                } else {
                                    &self.file_name
                                },
                            );
                            ui.close();
                        }
                    });
                    ui.menu_button(locale.view, |ui| {
                        ui.menu_button(locale.grid, |ui| {
                            ui.radio_value(
                                &mut self.field.grid_type,
                                GridType::Cells,
                                locale.cells,
                            );
                            ui.radio_value(&mut self.field.grid_type, GridType::Dots, locale.dots);
                            ui.radio_value(&mut self.field.grid_type, GridType::None, locale.empty);
                        });
                        ui.menu_button(locale.language, |ui| {
                            for other_local in SUPPORTED_LOCALES {
                                ui.radio_value(
                                    &mut self.locale,
                                    *other_local,
                                    other_local.locale().locale_name,
                                );
                            }
                        });
                        ui.menu_button(locale.theme, |ui| {
                            ui.radio_value(&mut self.theme, Theme::Dark, locale.theme_dark);
                            ui.radio_value(&mut self.theme, Theme::Light, locale.theme_light);
                        });
                    });
                    ui.menu_button(locale.help, |ui| {
                        if ui.button(locale.about).clicked() {
                            self.helpers.about_showed = true;
                            ui.close();
                        }
                    });
                    if ui.available_width() >= ui.available_height() * 2.5 + 40.0 {
                        ui.add_space(10.0);
                        ui.add(egui::Label::new(locale.project_name).selectable(false));
                        let w = ui.available_width();
                        let h = ui.available_height();
                        ui.add(
                            egui::TextEdit::singleline(&mut self.file_name)
                                .hint_text(DUMMY_NAME)
                                .background_color(ui.visuals().faint_bg_color)
                                .desired_width(w - 10.0 - 2.0 * h)
                                .horizontal_align(egui::Align::Center),
                        );
                    }

                    panel_left_switch(ui, &mut self.preview_window.is_expanded);
                });
            });
        });

        self.field.set_external_drag_resp(self.preview_window.show(
            ctx,
            foreground,
            self.field.state.scale,
            locale,
        ));
        egui::CentralPanel::default().show(ctx, |ui| {
            self.field.show(ui);
        });
        self.helpers.show(ctx, self.locale);

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            let now = Instant::now();
            if let Some(last_time) = self.last_frame_time {
                let delta = now - last_time;
                let fps = 1.0 / delta.as_secs_f64();
                ui.label(format!("fps:{:}", fps.ceil() as i32));
            }
            self.last_frame_time = Some(now);
            ctx.request_repaint();
        });
        // Check Ctrl+S:
        if ctx.input_mut(|state| {
            state.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, egui::Key::S))
        }) {
            self.save();
        }
    }
}

fn panel_left_switch(ui: &mut egui::Ui, is_expanded: &mut bool) {
    let h = ui.available_height();
    ui.add_space((ui.available_width() - h * 2.0).max(0.0));
    let (rect, resp) = ui.allocate_exact_size(vec2(1.3 * h, h), Sense::click());
    let visuals = ui.visuals();
    let mut color = visuals.text_color();
    if resp.hovered() {
        ui.ctx()
            .output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
        color = visuals.strong_text_color()
    }
    let stroke = Stroke::new(h * 0.075, color);
    let paint_rect = rect.scale_from_center(0.8);
    let r = paint_rect.height() * 0.06;
    ui.painter()
        .rect_stroke(paint_rect, r, stroke, egui::StrokeKind::Inside);
    if *is_expanded {
        ui.painter().rect_filled(
            Rect::from_min_size(
                paint_rect.min,
                vec2(paint_rect.height() * 0.4, paint_rect.height()),
            ),
            r,
            color,
        );
    } else {
        ui.painter().rect_stroke(
            Rect::from_min_size(
                paint_rect.min,
                vec2(paint_rect.height() * 0.4, paint_rect.height()),
            ),
            r,
            stroke,
            egui::StrokeKind::Inside,
        );
    }

    if resp.clicked() {
        *is_expanded = !*is_expanded;
    }
}
