//! Интеграционные тесты крейта, собранные в одну цель.
//!
//! Каждый файл `tests/*.rs` cargo собирает в отдельный тестовый бинарник,
//! и каждый такой бинарник заново компонуется со всеми зависимостями, так
//! что однопробные файлы стоили гигабайтов в `target/`. Здесь бывшие файлы
//! стали модулями одного бинарника, и полное имя теста теперь
//! `модуль::имя` — это важно тестам, которые перезапускают себя через
//! `current_exe()` с фильтром `--exact`.

mod background_service_facade;
mod base64;
mod file_configuration;
mod fixed_map_iteration;
mod handler_entry_mode;
mod internal_value_decoder;
mod internal_value_resolver;
mod local_date;
mod value_list_find;
