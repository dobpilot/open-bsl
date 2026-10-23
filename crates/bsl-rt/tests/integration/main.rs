//! Интеграционные тесты крейта, собранные в одну цель.
//!
//! Каждый файл `tests/*.rs` cargo собирает в отдельный тестовый бинарник,
//! и каждый такой бинарник заново компонуется со всеми зависимостями, так
//! что однопробные файлы стоили гигабайтов в `target/`. Здесь бывшие файлы
//! стали модулями одного бинарника, и полное имя теста теперь
//! `модуль::имя` — это важно тестам, которые перезапускают себя через
//! `current_exe()` с фильтром `--exact`.

mod application_command_line;
mod application_host;
mod arity;
mod array_access_methods;
mod base64_string_argument;
mod chart_label_location;
mod chart_plot_area;
mod chart_scale_label_location;
mod delete_races;
mod file_async;
mod file_begin_oracle;
mod file_handle_resize;
mod file_masks;
mod file_metadata;
mod find_files;
mod map_key_index;
mod method_lookup;
mod notification_method_matrix;
mod notification_subscriber_type;
mod open_questions_registry;
mod point_reference_order;
mod public_paths;
mod query_schema_order_direction;
mod spreadsheet_detail_use;
mod spreadsheet_pattern;
mod system_application;
mod system_file_removal;
mod table_access_methods;
mod temporary_directory;
mod temporary_file_ownership;
mod temporary_file_transfer;
