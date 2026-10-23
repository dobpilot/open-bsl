//! Интеграционные тесты крейта, собранные в одну цель.
//!
//! Каждый файл `tests/*.rs` cargo собирает в отдельный тестовый бинарник,
//! и каждый такой бинарник заново компонуется со всеми зависимостями, так
//! что однопробные файлы стоили гигабайтов в `target/`. Здесь бывшие файлы
//! стали модулями одного бинарника, и полное имя теста теперь
//! `модуль::имя` — это важно тестам, которые перезапускают себя через
//! `current_exe()` с фильтром `--exact`.

mod call_target;
mod configuration;
mod configuration_shared_tables;
mod constant_domain;
mod fused_operands;
mod image_finalize;
mod imports;
mod incremental_verification;
mod jump_target;
mod module_compilation_error;
mod object_call_arguments;
mod section_counts;
mod shared_import_storage;
mod support;
mod text_format;
