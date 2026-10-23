//! Интеграционные тесты крейта, собранные в одну цель.
//!
//! Каждый файл `tests/*.rs` cargo собирает в отдельный тестовый бинарник,
//! и каждый такой бинарник заново компонуется со всеми зависимостями, так
//! что однопробные файлы стоили гигабайтов в `target/`. Здесь бывшие файлы
//! стали модулями одного бинарника, и полное имя теста теперь
//! `модуль::имя` — это важно тестам, которые перезапускают себя через
//! `current_exe()` с фильтром `--exact`.

mod cfg;
mod configuration_error_context;
mod configuration_method_name_capacity;
mod configuration_narrow_name_links;
mod const_fold;
mod copy_elim;
mod debug_info;
mod duplicate_structure;
mod dynamic_await;
mod goto;
mod jump_target;
mod object_call_arguments;
mod open_method_omissions;
mod operands;
mod pressured_configuration_names;
mod shared_configuration_names;
mod ssa_live_out;
mod text_round_trip;
