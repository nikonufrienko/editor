use std::sync::{Arc, atomic::AtomicBool};

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

use egui::{Theme, mutex::Mutex};

use crate::{grid_db::GridBD, locale::Locale};

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
    loaded_data: Arc<Mutex<Result<(GridBD, String), &'static str>>>,
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
        bd: &mut GridBD,
        file_name: &mut String,
    ) {
        if self.state != FileManagerState::None {
            // Display state modal
            egui::modal::Modal::new("FileManager".into()).show(ctx, |ui| match &mut self.state {
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
                    export_theme,
                    cell_size,
                } => {
                    ui.horizontal(|ui| {
                        ui.label(locale.theme);
                        ui.radio_value(export_theme, Theme::Dark, locale.theme_dark);
                        ui.radio_value(export_theme, Theme::Light, locale.theme_light);
                    });
                    ui.horizontal(|ui| {
                        ui.label(locale.cell_size);
                        ui.add(egui::TextEdit::singleline(cell_size).desired_width(30.0))
                    });
                    if ui.button("OK").clicked() {
                        let theme = *export_theme;
                        match cell_size.parse::<f32>() {
                            Ok(cell_size) => self.export_to_svg(bd, file_name, theme, cell_size),
                            Err(_) => {
                                self.state = FileManagerState::Error(locale.illegal_cell_size)
                            }
                        }
                    }
                }
                _ => {}
            });
            match self.state {
                FileManagerState::OpenFile => {
                    if self.done.load(std::sync::atomic::Ordering::Relaxed) {
                        match &mut *self.loaded_data.lock() {
                            Ok((new_bd, new_file_name)) => {
                                *bd = std::mem::take(new_bd);
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

    fn load_data(
        data: Vec<u8>,
        locale: &'static Locale,
        file_name: String,
    ) -> Result<(GridBD, String), &'static str> {
        if let Ok(json) = String::from_utf8(data) {
            if let Ok(new_bd) = GridBD::load_from_json(json) {
                let striped_name = file_name
                    .strip_suffix(".json")
                    .unwrap_or(&file_name)
                    .to_string();
                return Ok((new_bd, striped_name));
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

    pub fn save_file(&mut self, bd: &GridBD, file_name: &String) {
        if let Some(data) = bd.dump_to_json() {
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
                use eframe::wasm_bindgen::JsCast;
                use web_sys::{Blob, Url};

                let blob =
                    Blob::new_with_str_sequence(&js_sys::Array::of1(&js_sys::JsString::from(data)))
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
                self.done.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        } else {
            // self.errors.lock().push(error_msg.into());
        }
    }

    pub fn start_export_svg(&mut self, default_theme: Theme) {
        self.state = FileManagerState::ExportSVGDialog {
            export_theme: default_theme,
            cell_size: "40".into(),
        };
    }

    fn export_to_svg(&mut self, bd: &GridBD, file_name: &String, theme: Theme, grid_size: f32) {
        self.state = FileManagerState::ExportSVG;
        let default_file_name = format!("{file_name}.svg");
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bd_arc = Arc::new(bd);
            let data = bd_arc.dump_to_svg(theme, grid_size);
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
            let data = bd.dump_to_svg(theme, grid_size);
            use eframe::wasm_bindgen::JsCast;
            use eframe::wasm_bindgen::prelude::Closure;
            use web_sys::{Blob, BlobPropertyBag, Url};

            let mut blob_properties = BlobPropertyBag::new();
            blob_properties.type_("image/svg+xml");

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

            self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}
