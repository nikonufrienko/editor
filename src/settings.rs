use egui::Theme;
use serde::{Deserialize, Serialize};

use crate::{
    field::GridType,
    locale::{Locale, LocaleType, get_system_default_locale},
};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ThemeWrapper {
    Dark,
    Light,
}

impl From<Theme> for ThemeWrapper {
    fn from(value: Theme) -> Self {
        match value {
            Theme::Dark => Self::Dark,
            Theme::Light => Self::Light,
        }
    }
}

impl Into<Theme> for ThemeWrapper {
    fn into(self) -> Theme {
        match self {
            Self::Dark => Theme::Dark,
            Self::Light => Theme::Light,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: ThemeWrapper,
    pub grid_type: GridType,
    pub locale: LocaleType,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: get_system_default_locale(),
            theme: ThemeWrapper::Dark,
            grid_type: GridType::Cells,
        }
    }
}

pub const SUPPORTED_THEMES: &[Theme] = &[Theme::Dark, Theme::Light];

pub trait GetName {
    fn get_name(&self, locale: &'static Locale) -> &'static str;
}

impl GetName for Theme {
    fn get_name(&self, locale: &'static Locale) -> &'static str {
        match self {
            Self::Dark => locale.theme_dark,
            Self::Light => locale.theme_light,
        }
    }
}
