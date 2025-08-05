#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::{Storage, egui};

use egui::{
    CursorIcon, Id, KeyboardShortcut, LayerId, Modifiers, Rect, Sense, Stroke, Theme, vec2,
};

use crate::{
    components_panel::ComponentsPanel,
    field::{Field, SUPPORTED_GRID_TYPES},
    file_managment::FileManager,
    helpers::Helpers,
    locale::{LocaleType, SUPPORTED_LOCALES},
    settings::{AppSettings, GetName, SUPPORTED_THEMES},
};

mod component_lib;
mod components_panel;
mod field;
mod file_managment;
mod grid_db;
mod helpers;
mod interaction_manager;
mod locale;
mod settings;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::sync::Arc;

    let icon_data = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png"))
        .expect("The icon data must be valid");

    let options = eframe::NativeOptions {
        multisampling: 8,
        dithering: false,

        viewport: egui::ViewportBuilder::default()
            .with_drag_and_drop(true)
            .with_icon(Arc::new(icon_data)),
        ..Default::default()
    };
    _ = eframe::run_native(
        "Editor",
        options,
        Box::new(|cc| {
            #[cfg(feature = "unifont")]
            load_unifont(cc);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(EditorApp::new(cc)))
        }),
    );
}

#[cfg(feature = "unifont")]
fn load_unifont(cc: &eframe::CreationContext) {
    use egui::{
        FontData,
        epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    };
    cc.egui_ctx.add_font(FontInsert::new(
        "unifont",
        FontData::from_static(include_bytes!("../assets/fonts/unifont-16.0.04.otf")),
        vec![
            InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: FontPriority::Lowest,
            },
            InsertFontFamily {
                family: egui::FontFamily::Monospace,
                priority: FontPriority::Lowest,
            },
        ],
    ));
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
                Box::new(|cc| {
                    #[cfg(feature = "unifont")]
                    load_unifont(cc);
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    Ok(Box::new(EditorApp::new(cc)))
                }),
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
    preview_window: ComponentsPanel,
    locale: locale::LocaleType,
    file_manager: FileManager,
    helpers: Helpers,
    file_name: String,
    theme: Theme,
}

impl EditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings: AppSettings = cc
            .storage
            .and_then(|s| s.get_string("settings"))
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let mut field = Field::new();
        field.grid_type = settings.grid_type;

        EditorApp {
            field: field,
            preview_window: ComponentsPanel::new(),
            locale: if settings.locale.is_supported() {
                settings.locale
            } else {
                LocaleType::En
            },
            file_manager: FileManager::new(),
            helpers: Helpers::new(cc),
            file_name: "Untitled".into(),
            theme: settings.theme.into(),
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_theme(self.theme);
        let locale: &'static locale::Locale = self.locale.locale();
        let foreground: LayerId = LayerId::new(egui::Order::Foreground, Id::new("foreground"));
        self.file_manager
            .update(ctx, locale, &mut self.field.grid_db, &mut self.file_name);
        ctx.tessellation_options_mut(|options| options.feathering = false);
        egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button(locale.file, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        if ui.button(locale.open).clicked() {
                            self.file_manager.open_file(locale);
                            ui.close();
                        }
                        if ui.button(locale.save).clicked() {
                            self.file_manager
                                .save_file(&self.field.grid_db, &self.file_name);
                            ui.close();
                        }
                        if ui.button(locale.export_to_svg).clicked() {
                            self.file_manager.start_export_svg(
                                ctx,
                                &self.field.grid_db,
                                self.theme,
                            );
                            ui.close();
                        }
                    });
                    ui.menu_button(locale.view, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.menu_button(locale.grid, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            SUPPORTED_GRID_TYPES.iter().for_each(|grid_type| {
                                ui.radio_value(
                                    &mut self.field.grid_type,
                                    *grid_type,
                                    grid_type.get_name(locale),
                                );
                            });
                        });
                        ui.menu_button(locale.language, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            SUPPORTED_LOCALES.iter().for_each(|locale| {
                                ui.add_enabled_ui(locale.is_supported(), |ui| {
                                    ui.radio_value(&mut self.locale, *locale, locale.get_name());
                                });
                            });
                        });
                        ui.menu_button(locale.theme, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            SUPPORTED_THEMES.iter().for_each(|theme| {
                                ui.radio_value(&mut self.theme, *theme, theme.get_name(locale));
                            });
                        });
                    });
                    ui.menu_button(locale.help, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        if ui.button(locale.about).clicked() {
                            self.helpers.about_showed = true;
                            ui.close();
                        }
                    });
                    if ui.available_width() >= ui.available_height() * 2.5 + 40.0 {
                        ui.add_space(10.0);
                        ui.add(
                            egui::Label::new(locale.project_name.to_string() + &":")
                                .selectable(false),
                        );
                        let w = ui.available_width();
                        let h = ui.available_height();
                        ui.add(
                            egui::TextEdit::singleline(&mut self.file_name)
                                .hint_text(locale.project_name)
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
            self.field.show(ui, locale);
        });
        self.helpers.show(ctx, self.locale);

        // Check Ctrl+S:
        if ctx.input_mut(|state| {
            state.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, egui::Key::S))
        }) {
            self.file_manager
                .save_file(&self.field.grid_db, &self.file_name);
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let Ok(value) = serde_json::to_string(&AppSettings {
            grid_type: self.field.grid_type,
            locale: self.locale,
            theme: self.theme.into(),
        }) {
            storage.set_string("settings", value);
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
