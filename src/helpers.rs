use std::sync::Arc;

use egui::{ImageSource, SizeHint, TextureOptions, load::Bytes};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use include_dir::{Dir, include_dir};

use crate::locale::LocaleType;

pub struct Helpers {
    cache: CommonMarkCache,
    pub about_showed: bool,
}

static ASSETS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/common");

impl Helpers {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        for file in ASSETS_DIR.files() {
            if let Some(file_name) = file.path().file_name() {
                let bytes = Bytes::from(Arc::from(file.contents()));
                _ = ImageSource::Bytes {
                    uri: format!("bytes://assets/common/{}", file_name.to_str().unwrap()).into(),
                    bytes,
                }
                .load(&cc.egui_ctx, TextureOptions::default(), SizeHint::default());
            }
        }
        Self {
            cache: CommonMarkCache::default(),
            about_showed: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, locale_type: LocaleType) {
        self.show_about_window(ctx, locale_type);
    }

    fn show_about_window(&mut self, ctx: &egui::Context, locale_type: LocaleType) {
        egui::Window::new(locale_type.locale().about)
            .id("about".into())
            .collapsible(false)
            .open(&mut self.about_showed)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CommonMarkViewer::new()
                        .default_implicit_uri_scheme("bytes://")
                        .show(
                            ui,
                            &mut self.cache,
                            match locale_type {
                                LocaleType::Ru => include_str!("../Readme_ru.md"),
                                LocaleType::En => include_str!("../Readme.md"),
                            },
                        );
                });
            });
    }
}
