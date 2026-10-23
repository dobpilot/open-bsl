//! Интеграционные тесты крейта, собранные в одну цель.
//!
//! Каждый файл `tests/*.rs` cargo собирает в отдельный тестовый бинарник,
//! и каждый такой бинарник заново компонуется со всеми зависимостями, так
//! что однопробные файлы стоили гигабайтов в `target/`. Здесь бывшие файлы
//! стали модулями одного бинарника, и полное имя теста теперь
//! `модуль::имя` — это важно тестам, которые перезапускают себя через
//! `current_exe()` с фильтром `--exact`.

mod accumulation_register_balances;
mod accumulation_register_record_sets;
mod accumulation_register_templates;
mod business_process_filling;
mod business_process_object_creation;
mod business_process_object_types;
mod business_process_route_points;
mod business_process_write;
mod catalog_attribute_context;
mod catalog_check_filling;
mod catalog_common_attributes;
mod catalog_create_folder;
mod catalog_deletion_mark;
mod catalog_find_attribute;
mod catalog_folder_flag;
mod catalog_level;
mod catalog_lock;
mod catalog_new_codes;
mod catalog_new_reference;
mod catalog_objects;
mod catalog_own_extension_modes;
mod catalog_owner;
mod catalog_parent;
mod catalog_predefined_items;
mod catalog_predefined_name;
mod catalog_references;
mod catalog_selection;
mod catalog_standard_search;
mod catalog_tabular_section_context;
mod catalog_templates;
mod catalog_write_boundary;
mod characteristic_deletion_mark;
mod characteristic_manager_reference;
mod characteristic_objects;
mod characteristic_predefined;
mod characteristic_predefined_manager;
mod common_module_name_errors;
mod constant_set;
mod constant_value_manager;
mod conversation_type;
mod data_processor_object_templates;
mod data_processor_objects;
mod data_processor_templates;
mod data_separation_frames;
mod database_data_size;
mod document_attribute_metadata;
mod document_check_filling;
mod document_find_attribute;
mod document_lock_state;
mod document_modified;
mod document_movements;
mod document_new_numbers;
mod document_new_references;
mod document_objects;
mod document_point_in_time;
mod document_references;
mod document_tabular_sections;
mod document_templates;
mod document_write_boundary;
mod entry_source_roots;
mod enumeration_transfer;
mod error_description_instance;
mod event_log_entry;
mod information_register_factories;
mod information_register_get;
mod information_register_last;
mod information_register_record_keys;
mod information_register_selection;
mod information_register_templates;
mod on_demand_factory;
mod pending_user_messages;
mod planner_items;
mod query_effective_privilege;
mod receiver_alias_collision;
mod reference_storage;
mod report_object;
mod report_templates;
mod separation_initial_values;
mod server_administration;
mod session_dynamic_configuration_status;
mod session_started;
mod settings_storage_error_phases;
mod settings_storage_load;
mod settings_storage_load_errors;
mod subsystem_scope;
mod task_reference_managers;
mod ws_references;
