use std::sync::{Arc, atomic::AtomicBool};

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

use egui::mutex::Mutex;

use crate::{grid_db::GridBD, locale::Locale};

#[derive(PartialEq, Debug)]
enum FileManagerState {
    OpenFile,
    SaveFile,
    ExportSVG,
    None,
    Error(&'static str),
}

pub struct FileManager {
    state: FileManagerState,
    done: Arc<AtomicBool>, // For async action status checking
    loaded_data: Arc<Mutex<Result<GridBD, &'static str>>>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            state: FileManagerState::None,
            done: Arc::new(AtomicBool::new(false)),
            loaded_data: Arc::new(Mutex::new(Err(&""))), // Dummy value
        }
    }

    fn check_dropping_files(&mut self, ctx: &egui::Context, locale: &Locale, bd: &mut GridBD) {
        if ctx.input(|input_state| !input_state.raw.hovered_files.is_empty()) {
            egui::modal::Modal::new("FileManager".into())
                .show(ctx, |ui| ui.label(locale.file_hovered_message));
        }

        let file_read_err = ctx.input(|input_state| {
            if !input_state.raw.dropped_files.is_empty() {
                if let Some(file) = input_state.raw.dropped_files.first() {
                    // bd.name = file.name // TODO: use name of file in bd
                    if let Some(bytes) = file.bytes.clone() {
                        if let Ok(json) = String::from_utf8(bytes.to_vec()) {
                            if let Ok(result) = GridBD::load_from_json(json) {
                                *bd = result;
                                return false;
                            }
                        }
                    } else {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            println!("{:?}", file.path);
                            if let Some(path) = &file.path {
                                if let Ok(mut file) = File::open(path) {
                                    let mut bytes = vec![];
                                    if let Ok(_size) = file.read_to_end(&mut bytes) {
                                        println!("2:{_size}");
                                        if let Ok(json) = String::from_utf8(bytes.to_vec()) {
                                            if let Ok(result) = GridBD::load_from_json(json) {
                                                *bd = result;
                                                return false;
                                            }
                                        }
                                    }
                                }
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

    pub fn update(&mut self, ctx: &egui::Context, locale: &Locale, bd: &mut GridBD) {
        if self.state != FileManagerState::None {
            // Display state modal
            egui::modal::Modal::new("FileManager".into()).show(ctx, |ui| match self.state {
                FileManagerState::SaveFile => {
                    ui.label(locale.saving_file);
                }
                FileManagerState::OpenFile => {
                    ui.label(locale.opening_file);
                }
                FileManagerState::Error(err) => {
                    ui.horizontal(|ui| {
                        ui.label(err);
                    });
                    if ui.button("OK").clicked() {
                        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                FileManagerState::ExportSVG => {
                    ui.label(locale.ongoing_export_to_svg);
                }
                _ => {}
            });
            match self.state {
                FileManagerState::OpenFile => {
                    if self.done.load(std::sync::atomic::Ordering::Relaxed) {
                        match &mut *self.loaded_data.lock() {
                            Ok(new_bd) => {
                                *bd = std::mem::take(new_bd);
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
            self.check_dropping_files(ctx, locale, bd);
        }
    }

    fn load_data(data: Vec<u8>, locale: &'static Locale) -> Result<GridBD, &'static str> {
        if let Ok(json) = String::from_utf8(data) {
            if let Ok(new_bd) = GridBD::load_from_json(json) {
                return Ok(new_bd);
            } else {
                Err(locale.file_wrong_format)
            }
        } else {
            Err(locale.file_wrong_format)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
        // this is stupid... use any executor of your choice instead
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
                    *receiver = Self::load_data(data, locale);
                } else {
                    let mut receiver = resp.lock();
                    *receiver = Err(locale.file_load_error);
                }
                status.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
    }

    pub fn save_file(&mut self, bd: &GridBD) {
        if let Some(data) = bd.dump_to_json() {
            self.state = FileManagerState::SaveFile;
            #[cfg(not(target_arch = "wasm32"))]
            {
                let arc = self.done.clone().clone();
                Self::execute(async move {
                    if let Some(file) = rfd::AsyncFileDialog::new()
                        .set_file_name("1.json")
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

                a.set_download("1.json");
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

    pub fn export_to_svg(&mut self, bd: &GridBD) {
        self.state = FileManagerState::ExportSVG;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bd_arc = Arc::new(bd);
            let data = bd_arc.dump_to_svg();
            let arc = self.done.clone().clone();
            Self::execute(async move {
                if let Some(file) = rfd::AsyncFileDialog::new()
                    .set_file_name("1.svg")
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
            let data = bd.dump_to_svg();
            use eframe::wasm_bindgen::JsCast;
            use eframe::wasm_bindgen::prelude::Closure;
            use web_sys::{Blob, BlobPropertyBag, Url};

            // Создаем Blob с правильным MIME-типом
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

            // Проверяем, что окно открылось (может быть заблокировано popup-блокером)
            if opened.is_some() {
                // Освобождаем ресурсы с задержкой
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
                // Fallback: если popup заблокирован, показываем alert с инструкцией
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
