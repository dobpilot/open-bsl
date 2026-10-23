//! Объектная модель данных и метаданных конфигурации 1С.
//!
//! Крейт является runtime-компонентом для существующего интерпретатора
//! `open-bsl`. Он не разбирает и не исполняет BSL самостоятельно. Физический
//! доступ к данным будет внедряться данными сеанса; каталог ниже хранит только
//! неизменяемое описание конфигурации и связи с её BSL-модулями.

#![warn(missing_docs)]

mod exchange_plan;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use open_bsl::{
    Arity, BslNumber, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, EnumKind,
    EnumValue, FunctionCode, FunctionDescriptor, FunctionKind, GlobalDescriptor, GraphLimits,
    LibraryDescriptor, MethodDescriptor, ObjectMembersDescriptor, ObjectProtocol, PICTURE_TYPE,
    PropertyDescriptor, RtError, RtResult, SerializedValueGraph, TypeDescriptor, TypeRef, Value,
    ValueTypeDescriptor, call_method_from_table, folded_eq, get_property_from_table, lookup_enum,
    lookup_member, set_property_from_table,
};

mod common_attribute_separation;
pub use common_attribute_separation::CommonAttributeSeparationParameters;
mod common_attribute_content;
pub use common_attribute_content::{CommonAttributeCondition, CommonAttributeContentEntry};

mod accumulation_balance;
mod query_boundary;
pub use query_boundary::query_boundary_parts;
mod accumulation_metadata;
mod attribute_filling;
mod business_process_metadata;
mod business_process_object;
mod catalog_object;
mod catalog_selection;
mod catalog_standard_search;
mod characteristic_object;
mod data_processor_object;
mod document_movements;
mod document_object;
mod document_tabular_section;
mod functional_options;
mod information_register_get;
mod metadata_attributes;
mod metadata_find_by_type;
mod reference_uuid;
mod route_points;
mod second_factor_setting;

mod access_rights;
mod catalog_error;
mod configuration_extension;
mod configuration_properties;
mod data_access;
mod data_composition;
mod globals;
mod infobase;
mod infobase_users;
mod library_tables;
mod managers;
mod metadata_catalog;
mod metadata_object;
mod metadata_values;
mod picture_library;
mod query;
mod references;
mod unavailable_constructors;
mod value_storage;

use access_rights::*;
use configuration_extension::*;
use data_access::*;
use data_composition::*;
use globals::*;
use infobase::*;
use infobase_users::*;
use library_tables::*;
use managers::*;
use metadata_catalog::*;
use metadata_values::*;
use picture_library::*;
use query::*;
use references::*;
use unavailable_constructors::*;
use value_storage::*;

pub use accumulation_metadata::AccumulationRegisterKind;
pub use business_process_metadata::BusinessProcessNumberSettings;
pub use catalog_error::CatalogError;
pub use configuration_extension::{
    ConfigurationExtension, ConfigurationExtensionPurpose, ConfigurationExtensionScope,
    ConfigurationExtensionsSnapshot, ExtensionApplicationIssue, ExtensionApplicationIssueSeverity,
    ExtensionSafeMode,
};
pub use configuration_properties::{CompatibilityMode, PredefinedDataUpdateMode, ScriptVariant};
pub use data_access::{
    AccessRightRequest, AccessRightSubject, AuthenticationLockSettingsSnapshot, DataAccess,
    DataLockRequest, FindByAttributeRequest, FindByRefRequest, FunctionalOptionRequest, Principal,
    QueryRequest, QueryResultData, QueryResultDataError, SessionData, TempTablesManagerHandle,
    UserPasswordNumericSetting, UserPasswordPolicySnapshot, UserPasswordSetting,
};
pub use functional_options::{
    FunctionalOptionLocation, FunctionalOptionParameter, FunctionalOptionSettings,
};
pub use information_register_get::InformationRegisterPeriodicity;
pub use metadata_catalog::MetadataCatalog;
pub use metadata_object::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogHierarchyType,
    CatalogStandardAttributeFillChecking, CatalogStandardAttributes, CharacteristicSettings,
    ConfigurationEnumValue, DocumentAttribute, DocumentNumberPeriodicity, DocumentNumberSettings,
    DocumentNumberType, DocumentNumberingSettings, DocumentTabularSection, FillChecking,
    MetadataKind, MetadataObject, MetadataObjectSource,
};
pub use references::{
    catalog_reference_parts, catalog_reference_type_id, catalog_reference_value,
    configuration_reference_parts, configuration_reference_value,
};
pub use route_points::BusinessProcessRoutePoint;
pub use value_storage::{value_storage_from_internal, value_storage_from_internal_with_resolver};

/// Платформенные методы собственного модуля объекта бизнес-процесса.
pub fn business_process_object_receiver_methods() -> &'static [MethodDescriptor] {
    business_process_object::METHODS
}

/// Стандартные свойства собственного модуля объекта бизнес-процесса.
pub fn business_process_object_receiver_properties() -> &'static [PropertyDescriptor] {
    business_process_object::PROPERTIES
}

/// Платформенные методы собственного получателя модуля объекта справочника.
pub fn catalog_object_receiver_methods() -> &'static [MethodDescriptor] {
    catalog_object::METHODS
}

/// Платформенные свойства собственного получателя модуля объекта справочника.
pub fn catalog_object_receiver_properties() -> &'static [PropertyDescriptor] {
    catalog_object::PROPERTIES
}

/// Платформенные методы получателя модуля объекта плана видов характеристик.
pub fn characteristic_object_receiver_methods() -> &'static [MethodDescriptor] {
    characteristic_object::METHODS
}

/// Платформенные свойства получателя модуля объекта плана видов характеристик.
pub fn characteristic_object_receiver_properties() -> &'static [PropertyDescriptor] {
    characteristic_object::PROPERTIES
}

/// Платформенные методы получателя модуля объекта обработки.
pub fn data_processor_object_receiver_methods() -> &'static [MethodDescriptor] {
    data_processor_object::METHODS
}

/// Платформенные свойства получателя модуля объекта отчёта.
pub fn report_object_receiver_properties() -> &'static [PropertyDescriptor] {
    data_processor_object::REPORT_PROPERTIES
}

/// Платформенные методы получателя модуля объекта документа.
pub fn document_object_receiver_methods() -> &'static [MethodDescriptor] {
    document_object::METHODS
}

/// Платформенные свойства получателя модуля объекта документа.
pub fn document_object_receiver_properties() -> &'static [PropertyDescriptor] {
    document_object::PROPERTIES
}

#[cfg(test)]
#[path = "lazy_metadata_tests.rs"]
mod lazy_metadata_tests;

/// Возвращает runtime-библиотеку ORM для регистрации в `EngineBuilder`.
#[must_use]
pub const fn library() -> LibraryDescriptor {
    LIBRARY
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "exchange_plan_tests.rs"]
mod exchange_plan_tests;

#[cfg(test)]
#[path = "catalog_object_metadata_tests.rs"]
mod catalog_object_metadata_tests;

#[cfg(test)]
#[path = "document_object_metadata_tests.rs"]
mod document_object_metadata_tests;

#[cfg(test)]
#[path = "catalog_check_filling_tests.rs"]
mod catalog_check_filling_tests;

#[cfg(test)]
#[path = "characteristic_settings_tests.rs"]
mod characteristic_settings_tests;

#[cfg(test)]
#[path = "common_attribute_separation_tests.rs"]
mod common_attribute_separation_tests;

#[cfg(test)]
#[path = "common_attribute_content_tests.rs"]
mod common_attribute_content_tests;

#[cfg(test)]
#[path = "characteristic_reference_tests.rs"]
mod characteristic_reference_tests;

#[cfg(test)]
#[path = "configuration_extension_purpose_tests.rs"]
mod configuration_extension_purpose_tests;

#[cfg(test)]
#[path = "document_check_filling_message_tests.rs"]
mod document_check_filling_message_tests;

#[cfg(test)]
#[path = "business_process_check_message_tests.rs"]
mod business_process_check_message_tests;

#[cfg(test)]
#[path = "configuration_extension_hash_tests.rs"]
mod configuration_extension_hash_tests;

#[cfg(test)]
#[path = "metadata_type_names_tests.rs"]
mod metadata_type_names_tests;

#[cfg(test)]
#[path = "user_data_separation_tests.rs"]
mod user_data_separation_tests;

#[cfg(test)]
#[path = "user_uuid_tests.rs"]
mod user_uuid_tests;

#[cfg(test)]
#[path = "query_parameters_property_tests.rs"]
mod query_parameters_property_tests;

#[cfg(test)]
#[path = "enumeration_constant_tests.rs"]
mod enumeration_constant_tests;

#[cfg(test)]
#[path = "query_selection_index_tests.rs"]
mod query_selection_index_tests;

#[cfg(test)]
#[path = "metadata_find_by_type_tests.rs"]
mod metadata_find_by_type_tests;

#[cfg(test)]
#[path = "configuration_access_right_tests.rs"]
mod configuration_access_right_tests;

#[cfg(test)]
#[path = "filter_criteria_global_tests.rs"]
mod filter_criteria_global_tests;

#[cfg(test)]
#[path = "default_interface_tests.rs"]
mod default_interface_tests;

#[cfg(test)]
#[path = "query_boundary_tests.rs"]
mod query_boundary_tests;

#[cfg(test)]
#[path = "metadata_collection_index_tests.rs"]
mod metadata_collection_index_tests;

mod file_source;

#[cfg(test)]
#[path = "file_source_tests.rs"]
mod file_source_tests;
