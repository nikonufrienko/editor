use egui_commonmark::{commonmark_str, CommonMarkCache};

use crate::locale::LocaleType;

pub struct Helpers {
    cache: CommonMarkCache,
    pub about_showed: bool
}

impl Helpers {
    pub fn new() -> Self {
        Self {
            cache: CommonMarkCache::default(), about_showed: false }
    }

    pub fn show(&mut self, ctx: &egui::Context, locale_type: LocaleType) {
        self.show_about_window(ctx, locale_type);
    }

    fn show_about_window(&mut self, ctx: &egui::Context, locale_type: LocaleType) {
        egui::Window::new(locale_type.locale().about).id("about".into()).collapsible(false).open(&mut self.about_showed).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match locale_type {
                    LocaleType::Ru => commonmark_str!(ui, &mut self.cache, "docs/about_ru.md"),
                    LocaleType::En => commonmark_str!(ui, &mut self.cache, "docs/about_en.md"),
                }
            });
        });
    }
}

