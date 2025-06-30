

pub struct Locale {
    pub file_save_error : &'static str,
    pub grid: &'static str
}

pub const RU_LOCALE : Locale = Locale {
    file_save_error: "Ошибка сохранения файла",
    grid:            "Сетка",
};


pub const EN_LOCALE : Locale = Locale {
    file_save_error: "File save error",
    grid:            "Grid",
};
