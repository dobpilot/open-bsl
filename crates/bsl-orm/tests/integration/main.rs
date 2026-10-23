//! Интеграционные тесты крейта, собранные в одну цель.
//!
//! Каждый файл `tests/*.rs` cargo собирает в отдельный тестовый бинарник,
//! и каждый такой бинарник заново компонуется со всеми зависимостями, так
//! что однопробные файлы стоили гигабайтов в `target/`. Здесь бывшие файлы
//! стали модулями одного бинарника, и полное имя теста теперь
//! `модуль::имя` — это важно тестам, которые перезапускают себя через
//! `current_exe()` с фильтром `--exact`.

mod access_right_metadata;
mod adjusted_point_constructor;
mod chart_predefined_references;
mod compatibility_property_dot;
mod constant_set;
mod data_processor_objects;
mod document_attribute_right_targets;
mod information_register_get;
mod metadata_common_attributes;
mod point_in_time;
mod predefined_full_name_variant;
mod reference_type_presentation;
mod storage_internal_contents;
mod storage_reference_contents;
