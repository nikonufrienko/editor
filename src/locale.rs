#[cfg(not(target_arch = "wasm32"))]
use sys_locale::get_locale;

#[allow(unused)]
pub struct Locale {
    pub locale_name: &'static str,
    pub file: &'static str,
    pub save: &'static str,
    pub open: &'static str,
    pub file_save_error: &'static str,
    pub grid: &'static str,
    pub view: &'static str,
    pub cells: &'static str,
    pub dots: &'static str,
    pub empty: &'static str,
    pub language: &'static str,
    pub components: &'static str,
    pub saving_file: &'static str,
    pub opening_file: &'static str,
    pub file_load_error: &'static str,
    pub file_wrong_format: &'static str,
    pub file_hovered_message: &'static str,
    pub common_components: &'static str,
    pub filter: &'static str,
    pub logic_gates: &'static str,
    pub muxes: &'static str,
    pub input_outputs: &'static str,
    pub custom_units: &'static str,
    pub export_to_svg: &'static str,
    pub ongoing_export_to_svg: &'static str,
    pub help: &'static str,
    pub about: &'static str,
}

pub const RU_LOCALE: Locale = Locale {
    locale_name: "RU",
    file: "Файл",
    save: "Сохранить",
    open: "Открыть",
    file_save_error: "Ошибка сохранения файла",
    grid: "Сетка",
    cells: "Клетки",
    dots: "Точки",
    empty: "Пустая",
    view: "Вид",
    language: "Язык",
    components: "Компоненты",
    saving_file: "Сохранение файла...",
    opening_file: "Открытие файла...",
    file_load_error: "Ошибка при открытии файла",
    file_wrong_format: "Неверный формат файла",
    file_hovered_message: "А ну давай это сюда",
    filter: "Фильтр:",
    common_components: "Общие",
    logic_gates: "Логические гейты",
    muxes: "Мультиплексоры",
    input_outputs: "Входы/выходы",
    custom_units: "Кастомизируемые блоки",
    export_to_svg: "Экспорт в SVG",
    ongoing_export_to_svg: "Идет экспорт в SVG...",
    help: "Помощь",
    about: "О программе"
};

pub const EN_LOCALE: Locale = Locale {
    locale_name: "EN",
    file: "File",
    save: "Save",
    open: "Open",
    file_save_error: "File save error",
    grid: "Grid",
    view: "View",
    cells: "Cells",
    dots: "Dots",
    empty: "Empty",
    language: "Language",
    components: "Components",
    saving_file: "Saving file...",
    opening_file: "Opening file...",
    file_load_error: "File open error",
    file_wrong_format: "File wrong format",
    file_hovered_message: "Put it here",
    filter: "filter:",
    common_components: "Common",
    logic_gates: "Logic gates",
    muxes: "Muxes",
    input_outputs: "I/O",
    custom_units: "Custom units",
    export_to_svg: "Export to SVG",
    ongoing_export_to_svg: "Exporting to svg...",
    help: "Help",
    about: "About"
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
    Ru,
}

impl LocaleType {
    pub fn locale(&self) -> &'static Locale {
        match self {
            Self::En => &EN_LOCALE,
            Self::Ru => &RU_LOCALE,
        }
    }
}

pub const SUPPORTED_LOCALES: &'static [LocaleType] = &[LocaleType::Ru, LocaleType::En];
