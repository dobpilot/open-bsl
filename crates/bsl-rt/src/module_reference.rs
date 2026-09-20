//! Дескриптор собственного типа ссылки на исполняемый модуль open-bsl.

/// Тип значения ЭтотОбъект. Экземпляр и экспортные методы принадлежат VM;
/// это не объект управляемой формы и не имя платформенного типа 1С.
pub static BSL_MODULE_TYPE: crate::TypeDescriptor = crate::TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "МодульBSL",
    type_display: "Модуль BSL",
    type_names: &["BSLModule"],
};
