use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use sys_locale::get_locale;

#[allow(unused)]
pub struct Locale {
    pub grid: &'static str,
    // Grid types:
    pub cells: &'static str,
    pub dots: &'static str,
    pub empty: &'static str,

    // Component types:
    pub common_components: &'static str,
    pub logic_gates: &'static str,
    pub muxes: &'static str,
    pub input_outputs: &'static str,
    pub custom_units: &'static str,
    pub flip_flops: &'static str,
    pub arithmetic_primitives: &'static str,

    // UI:
    pub file: &'static str,
    pub save: &'static str,
    pub open: &'static str,
    pub view: &'static str,
    pub language: &'static str,
    pub components: &'static str,
    pub filter: &'static str,
    pub export_to_svg: &'static str,
    pub help: &'static str,
    pub about: &'static str,
    pub project_name: &'static str,
    pub theme: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    pub text_labels: &'static str,
    pub cell_size: &'static str,
    pub preview: &'static str,
    pub type_: &'static str,

    // Modal dialogs:
    pub illegal_cell_size: &'static str,
    pub saving_file: &'static str,
    pub opening_file: &'static str,
    pub file_load_error: &'static str,
    pub file_wrong_format: &'static str,
    pub file_hovered_message: &'static str,
    pub ongoing_export_to_svg: &'static str,
    pub file_save_error: &'static str,

    // Components parameters:
    pub inputs_number: &'static str,
    pub sync_reset: &'static str,
    pub async_reset: &'static str,
    pub sync_reset_inverted: &'static str,
    pub async_reset_inverted: &'static str,
    pub enable_signal: &'static str,
}

pub const RU_LOCALE: Locale = Locale {
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
    arithmetic_primitives: "Арифметические примитивы",
    muxes: "Мультиплексоры",
    input_outputs: "Входы/выходы",
    custom_units: "Кастомизируемые блоки",
    flip_flops: "Триггеры",
    export_to_svg: "Экспорт в SVG",
    ongoing_export_to_svg: "Идет экспорт в SVG...",
    help: "Помощь",
    about: "О программе",
    project_name: "Имя проекта",
    theme: "Тема",
    theme_dark: "Темная",
    theme_light: "Светлая",
    text_labels: "Текстовые метки",
    cell_size: "Размер клетки:",
    illegal_cell_size: "ОШИБКА: Неправильно задан размер клетки",
    inputs_number: "Количество входов",
    sync_reset: "Синхронный сброс",
    async_reset: "Асинхронный сброс",
    sync_reset_inverted: "Синхронный сброс инвертирован",
    async_reset_inverted: "Асинхронный сброс инвертирован",
    enable_signal: "Имеет вход сигнала включения (enable)",
    preview: "Предпросмотр",
    type_: "Тип",
};

pub const EN_LOCALE: Locale = Locale {
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
    arithmetic_primitives: "Arithmetic",
    logic_gates: "Logic gates",
    muxes: "Muxes",
    input_outputs: "I/O",
    custom_units: "Custom units",
    flip_flops: "Flip-flops",
    export_to_svg: "Export to SVG",
    ongoing_export_to_svg: "Exporting to svg...",
    help: "Help",
    about: "About",
    project_name: "Project name",
    theme: "Theme",
    theme_dark: "Dark",
    theme_light: "Light",
    text_labels: "Text labels",
    cell_size: "Cell size:",
    illegal_cell_size: "ERROR: illegal cell size",
    inputs_number: "Number of inputs",
    sync_reset: "Synchronous reset",
    async_reset: "Asynchronous reset",
    sync_reset_inverted: "Synchronous reset inverted",
    async_reset_inverted: "Asynchronous reset inverted",
    enable_signal: "Enable signal",
    preview: "Preview",
    type_: "Type"
};

#[cfg(feature = "unifont")]
pub const ZH_LOCALE: Locale = Locale {
    file: "文件",
    save: "保存",
    open: "打开",
    file_save_error: "文件保存错误",
    grid: "网格",
    cells: "单元格",
    dots: "点阵",
    empty: "空白",
    view: "视图",
    language: "语言",
    components: "组件",
    saving_file: "正在保存文件...",
    opening_file: "正在打开文件...",
    file_load_error: "文件打开错误",
    file_wrong_format: "文件格式错误",
    file_hovered_message: "拖放到此处",
    filter: "筛选:",
    common_components: "常用",
    logic_gates: "逻辑门",
    muxes: "多路复用器",
    arithmetic_primitives: "算术原语",
    input_outputs: "输入/输出",
    custom_units: "自定义模块",
    flip_flops: "触发器",
    export_to_svg: "导出为SVG",
    ongoing_export_to_svg: "正在导出SVG...",
    help: "帮助",
    about: "关于",
    project_name: "项目名称",
    theme: "主题",
    theme_dark: "深色",
    theme_light: "浅色",
    text_labels: "文本标签",
    cell_size: "单元格大小:",
    illegal_cell_size: "错误: 非法的单元格大小",
    inputs_number: "输入数量",
    sync_reset: "同步复位",
    async_reset: "异步复位",
    sync_reset_inverted: "反向同步复位",
    async_reset_inverted: "反向异步复位",
    enable_signal: "使能信号",
    preview: "预览",
    type_: "类型"
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
        #[cfg(feature = "unifont")]
        s if s.starts_with("zh") => LocaleType::Zh,
        _ => LocaleType::En,
    }
}

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum LocaleType {
    En,
    Ru,
    Zh,
}

impl LocaleType {
    pub fn is_supported(&self) -> bool {
        match self {
            #[cfg(not(feature = "unifont"))]
            Self::Zh => false,
            _ => true,
        }
    }

    pub fn locale(&self) -> &'static Locale {
        match self {
            Self::En => &EN_LOCALE,
            Self::Ru => &RU_LOCALE,
            #[cfg(feature = "unifont")]
            Self::Zh => &ZH_LOCALE,
            #[cfg(not(feature = "unifont"))]
            Self::Zh => panic!("unifont function required"),
        }
    }

    pub fn get_readme(&self) -> &'static str {
        match self {
            Self::Ru => include_str!("../README_ru.md"),
            #[cfg(feature = "unifont")]
            Self::Zh => include_str!("../README_zh.md"),
            #[cfg(not(feature = "unifont"))]
            Self::Zh => panic!("unifont function required"),
            Self::En => include_str!("../README.md"),
        }
    }

    pub fn get_name(self) -> String {
        match self {
            LocaleType::En => "EN".into(),
            LocaleType::Ru => "RU".into(),
            LocaleType::Zh => "ZH".into(),
        }
    }
}

pub const SUPPORTED_LOCALES: &'static [LocaleType] =
    &[LocaleType::Ru, LocaleType::En, LocaleType::Zh];
