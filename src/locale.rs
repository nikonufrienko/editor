#[cfg(not(target_arch = "wasm32"))]
use sys_locale::get_locale;

#[allow(unused)]
pub struct Locale {
    pub locale_name: &'static str,
    pub file: &'static str,
    pub save: &'static str,
    pub open: &'static str,
    pub file_save_error : &'static str,
    pub grid: &'static str,
    pub view: &'static str,
    pub cells: &'static str,
    pub dots: &'static str,
    pub language: &'static str,
    pub components: &'static str,
    pub saving_file: &'static str,
    pub opening_file: &'static str,
    pub file_load_error: &'static str,
    pub file_wrong_format: &'static str,
    pub file_hovered_messege: &'static str,
}

pub const RU_LOCALE : Locale = Locale {
    locale_name:        "RU",
    file:               "Файл",
    save:               "Сохранить",
    open:               "Открыть",
    file_save_error:    "Ошибка сохранения файла",
    grid:               "Сетка",
    cells:              "Клетки", // Сетка в клеточку
    dots:               "Точки",  // Сетка в виде точек
    view:               "Вид",
    language:           "Язык",
    components:         "Компоненты",
    saving_file:        "Сохранение файла...",
    opening_file:       "Открытие файла...",
    file_load_error:    "Ошибка при открытии файла",
    file_wrong_format:  "Неверный формат файла",
    file_hovered_messege: "А ну давай это сюда"
};


pub const EN_LOCALE : Locale = Locale {
    locale_name:        "EN",
    file:               "File",
    save:               "Save",
    open:               "Open",
    file_save_error:    "File save error",
    grid:               "Grid",
    view:               "View",
    cells:              "Cells",
    dots:               "Dots",
    language:           "Language",
    components:         "Components",
    saving_file:        "Saving file...",
    opening_file:       "Opening file...",
    file_load_error:    "File open error",
    file_wrong_format:  "File wrong format",
    file_hovered_messege: "Put it here"
};


pub fn get_system_default_locale() -> LocaleType {
    let locale;

    #[cfg(not(target_arch = "wasm32"))]
    {
        locale = get_locale().unwrap_or_else(|| "en-US".into());
    }

    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("No global `window` found");
        let navigator = window.navigator();
        locale = navigator.language().unwrap_or_else(|| "en-US".into());
    }

    match locale.to_lowercase().as_str() {
        s if s.starts_with("ru") => LocaleType::Ru,
        _ => LocaleType::En,
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum LocaleType {
    En,
    Ru
}

impl LocaleType {
    pub fn locale(&self) -> &'static Locale {
        match self {
            Self::En => &EN_LOCALE,
            Self::Ru => &RU_LOCALE,
        }
    }
}

pub const SUPPORTED_LOCALES: &'static [LocaleType] = &[
    LocaleType::Ru,
    LocaleType::En
];
