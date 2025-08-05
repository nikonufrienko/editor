use std::sync::{Arc, atomic::AtomicBool};

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

use egui::{Theme, mutex::Mutex};

use crate::{grid_db::GridDB, locale::Locale};

#[derive(PartialEq, Debug)]
enum FileManagerState {
    OpenFile,
    SaveFile,
    ExportSVGDialog {
        export_theme: Theme,
        cell_size: String,
    },
    ExportSVG,
    None,
    Error(&'static str),
}

pub struct FileManager {
    state: FileManagerState,
    done: Arc<AtomicBool>, // For async action status checking
    loaded_data: Arc<Mutex<Result<(GridDB, String), &'static str>>>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            state: FileManagerState::None,
            done: Arc::new(AtomicBool::new(false)),
            loaded_data: Arc::new(Mutex::new(Err(&""))), // Dummy value
        }
    }

    fn check_dropping_files(&mut self, ctx: &egui::Context, locale: &'static Locale) {
        if ctx.input(|input_state| !input_state.raw.hovered_files.is_empty()) {
            egui::modal::Modal::new("FileManager".into())
                .show(ctx, |ui| ui.label(locale.file_hovered_message));
        }

        let file_read_err = ctx.input(|input_state| {
            if !input_state.raw.dropped_files.is_empty() {
                if let Some(file) = input_state.raw.dropped_files.first() {
                    let resp = self.loaded_data.clone();
                    if let Some(bytes) = file.bytes.clone() {
                        let file_name = file.name.clone();
                        self.state = FileManagerState::OpenFile;
                        let status = self.done.clone().clone();
                        Self::execute(async move {
                            let data = bytes.to_vec();
                            let mut receiver = resp.lock();
                            *receiver = Self::load_data(data, locale, file_name);
                            status.store(true, std::sync::atomic::Ordering::Relaxed);
                        });
                        return false;
                    } else {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if let Some(path) = file.path.clone() {
                                let file_name = file.name.clone();
                                self.state = FileManagerState::OpenFile;
                                let status = self.done.clone().clone();

                                Self::execute(async move {
                                    let mut receiver = resp.lock();
                                    if let Ok(mut file) = File::open(path) {
                                        let mut bytes = vec![];
                                        if let Ok(_size) = file.read_to_end(&mut bytes) {
                                            *receiver = Self::load_data(bytes, locale, file_name);
                                            status
                                                .store(true, std::sync::atomic::Ordering::Relaxed);
                                        } else {
                                            *receiver = Err(locale.file_load_error);
                                            status
                                                .store(true, std::sync::atomic::Ordering::Relaxed);
                                        }
                                    } else {
                                        *receiver = Err(locale.file_load_error);
                                        status.store(true, std::sync::atomic::Ordering::Relaxed);
                                    }
                                });

                                return true;
                            }
                        }
                    }
                }
                true
            } else {
                false
            }
        });
        if file_read_err {
            self.state = FileManagerState::Error(locale.file_load_error);
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        locale: &'static Locale,
        db: &mut GridDB,
        file_name: &mut String,
    ) {
        if self.state != FileManagerState::None {
            // Display state modal
            egui::modal::Modal::new("FileManager".into()).show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                match &mut self.state {
                    FileManagerState::SaveFile => {
                        ui.label(locale.saving_file);
                    }
                    FileManagerState::OpenFile => {
                        ui.label(locale.opening_file);
                    }
                    FileManagerState::Error(err) => {
                        ui.horizontal(|ui| {
                            ui.label(*err);
                        });
                        if ui.button("OK").clicked() {
                            self.done.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    FileManagerState::ExportSVG => {
                        ui.label(locale.ongoing_export_to_svg);
                    }
                    FileManagerState::ExportSVGDialog {
                        export_theme: _,
                        cell_size: _,
                    } => {
                        self.export_file_dialog(ui, locale, db, file_name);
                    }
                    _ => {}
                }
            });
            match self.state {
                FileManagerState::OpenFile => {
                    if self.done.load(std::sync::atomic::Ordering::Relaxed) {
                        match &mut *self.loaded_data.lock() {
                            Ok((new_db, new_file_name)) => {
                                *db = std::mem::take(new_db);
                                *file_name = new_file_name.clone();
                                self.state = FileManagerState::None;
                                self.done.store(false, std::sync::atomic::Ordering::Relaxed);
                            }
                            Err(err) => {
                                self.state = FileManagerState::Error(err);
                                self.done.store(false, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                }
                _ => {
                    if self.done.load(std::sync::atomic::Ordering::Relaxed) {
                        self.state = FileManagerState::None;
                        self.done.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            };
        } else {
            self.check_dropping_files(ctx, locale);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn show_preview_wasm(db: &mut GridDB, grid_size: f32, theme: Theme) {
        {
            let data = db.dump_to_svg(theme, grid_size);
            use eframe::wasm_bindgen::JsCast;
            use eframe::wasm_bindgen::prelude::Closure;
            use web_sys::{Blob, BlobPropertyBag, Url};

            let blob_properties = BlobPropertyBag::new();
            blob_properties.set_type("image/svg+xml");

            let blob = Blob::new_with_str_sequence_and_options(
                &js_sys::Array::of1(&js_sys::JsString::from(data)),
                &blob_properties,
            )
            .unwrap();

            let url = Url::create_object_url_with_blob(&blob).unwrap();

            let window = web_sys::window().unwrap();
            let opened = window.open_with_url_and_target(&url, "_blank").unwrap();

            if opened.is_some() {
                let closure = Closure::once(move || {
                    Url::revoke_object_url(&url).unwrap();
                });

                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        5000,
                    )
                    .unwrap();

                closure.forget();
            } else {
                window
                    .alert_with_message(
                        "Popup blocked! Please allow popups for this site and try again.",
                    )
                    .unwrap();
                Url::revoke_object_url(&url).unwrap();
            }
        }
    }

    fn export_file_dialog(
        &mut self,
        ui: &mut egui::Ui,
        locale: &'static Locale,
        db: &mut GridDB,
        file_name: &String,
    ) {
        let (export_theme, cell_size) = match &mut self.state {
            FileManagerState::ExportSVGDialog {
                export_theme,
                cell_size,
            } => (export_theme, cell_size),
            _ => panic!(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let egui::Vec2 { x, y } = ui.ctx().available_rect().size();
            ui.set_min_size(egui::vec2(x.min(y), x.min(y)) * 0.5);
            ui.set_max_size(egui::vec2(x.min(y), x.min(y)) * 0.5);
            let mut preview_valid = true;
            ui.horizontal(|ui| {
                ui.label(locale.theme);
                let change0 = ui
                    .radio_value(export_theme, Theme::Dark, locale.theme_dark)
                    .changed();
                let change1 = ui
                    .radio_value(export_theme, Theme::Light, locale.theme_light)
                    .changed();
                if change0 || change1 {
                    Self::reload_preview(ui.ctx(), db, *export_theme);
                    preview_valid = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label(locale.cell_size);
                ui.add(egui::TextEdit::singleline(cell_size).desired_width(30.0))
            });
            if preview_valid {
                ui.add(egui::Image::new(egui::ImageSource::Uri(
                    "bytes://preview.svg".into(),
                )));
            }
            ui.add_space((ui.available_height() - 20.0).max(0.0));
            let theme = export_theme.clone();
            if ui.button("OK").clicked() {
                match cell_size.parse::<f32>() {
                    Ok(cell_size) => self.export_to_svg(db, file_name, theme, cell_size),
                    Err(_) => self.state = FileManagerState::Error(locale.illegal_cell_size),
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            ui.horizontal(|ui| {
                ui.label(locale.theme);
                ui.radio_value(export_theme, Theme::Dark, locale.theme_dark)
                    .changed();
                ui.radio_value(export_theme, Theme::Light, locale.theme_light)
                    .changed();
            });
            let parse_result = cell_size.parse::<f32>();

            ui.horizontal(|ui| {
                ui.label(locale.cell_size);
                ui.add(egui::TextEdit::singleline(cell_size).desired_width(30.0));
                if parse_result.is_err() {
                    ui.label("⚠");
                }
            });
            let theme = export_theme.clone();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    match parse_result {
                        Ok(cell_size) => self.export_to_svg(db, file_name, theme, cell_size),
                        Err(_) => self.state = FileManagerState::Error(locale.illegal_cell_size),
                    }
                }
                if ui.button(locale.preview).clicked() {
                    Self::show_preview_wasm(db, 100.0, theme);
                }
            });
        }
    }

    fn load_data(
        data: Vec<u8>,
        locale: &'static Locale,
        file_name: String,
    ) -> Result<(GridDB, String), &'static str> {
        if let Ok(json) = String::from_utf8(data) {
            if let Ok(new_db) = GridDB::load_from_json(json) {
                let striped_name = file_name
                    .strip_suffix(".json")
                    .unwrap_or(&file_name)
                    .to_string();
                return Ok((new_db, striped_name));
            } else {
                Err(locale.file_wrong_format)
            }
        } else {
            Err(locale.file_wrong_format)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
        smol::spawn(f).detach();
    }

    #[cfg(target_arch = "wasm32")]
    fn execute<F: Future<Output = ()> + 'static>(f: F) {
        wasm_bindgen_futures::spawn_local(f);
    }

    pub fn open_file(&mut self, locale: &'static Locale) {
        self.state = FileManagerState::OpenFile;
        {
            let status = self.done.clone().clone();
            let resp = self.loaded_data.clone();

            Self::execute(async move {
                if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                    let data = file.read().await;
                    let mut receiver = resp.lock();
                    *receiver = Self::load_data(data, locale, file.file_name());
                } else {
                    let mut receiver = resp.lock();
                    *receiver = Err(locale.file_load_error);
                }
                status.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn save_file_wasm(default_file_name: String, content: String) {
        #[cfg(target_arch = "wasm32")]
        {
            use eframe::wasm_bindgen::JsCast;
            use web_sys::{Blob, Url};

            let blob =
                Blob::new_with_str_sequence(&js_sys::Array::of1(&js_sys::JsString::from(content)))
                    .unwrap();

            let url = Url::create_object_url_with_blob(&blob).unwrap();

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let a = document
                .create_element("a")
                .unwrap()
                .dyn_into::<web_sys::HtmlAnchorElement>()
                .unwrap();

            a.set_download(&default_file_name);
            a.set_href(&url);
            //a.set_style("display", "none");

            document.body().unwrap().append_child(&a).unwrap();
            a.click();
            document.body().unwrap().remove_child(&a).unwrap();

            Url::revoke_object_url(&url).unwrap();
        }
    }

    pub fn save_file(&mut self, db: &GridDB, file_name: &String) {
        if let Some(data) = db.dump_to_json() {
            self.state = FileManagerState::SaveFile;
            let default_file_name = format!("{file_name}.json");
            #[cfg(not(target_arch = "wasm32"))]
            {
                let arc = self.done.clone().clone();
                Self::execute(async move {
                    if let Some(file) = rfd::AsyncFileDialog::new()
                        .set_file_name(default_file_name)
                        .save_file()
                        .await
                    {
                        file.write(data.as_bytes()).await.ok();
                        //errors.lock().push(error_msg.into());
                    }
                    arc.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                Self::save_file_wasm(default_file_name, data);
                self.done.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        } else {
            // self.errors.lock().push(error_msg.into());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reload_preview(ctx: &egui::Context, db: &GridDB, theme: Theme) {
        ctx.loaders().bytes.lock().iter().for_each(|loader| {
            loader.forget("bytes://preview.svg");
        });
        let svg = db.dump_to_svg(theme, 100.0);
        let bytes = svg.as_bytes();
        _ = egui::ImageSource::Bytes {
            uri: format!("bytes://preview.svg").into(),
            bytes: egui::load::Bytes::Shared(Arc::from(bytes)),
        }
        .load(
            ctx,
            egui::TextureOptions::default(),
            egui::SizeHint::Scale(1.0.into()),
        );
    }

    #[allow(unused_variables)]
    pub fn start_export_svg(&mut self, ctx: &egui::Context, db: &GridDB, default_theme: Theme) {
        #[cfg(not(target_arch = "wasm32"))]
        Self::reload_preview(ctx, db, default_theme);

        self.state = FileManagerState::ExportSVGDialog {
            export_theme: default_theme,
            cell_size: "40".into(),
        };
    }

    fn export_to_svg(&mut self, db: &GridDB, file_name: &String, theme: Theme, grid_size: f32) {
        self.state = FileManagerState::ExportSVG;
        let default_file_name = format!("{file_name}.svg");
        let data = db.dump_to_svg(theme, grid_size);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let arc = self.done.clone().clone();
            Self::execute(async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .set_file_name(default_file_name)
                    .save_file()
                    .await
                {
                    file.write(data.as_bytes()).await.ok();
                }
                arc.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::save_file_wasm(default_file_name, data);
            self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}
