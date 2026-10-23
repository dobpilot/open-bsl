//! Конструкторы, объявленные для компиляции модулей конфигурации, у которых
//! ещё нет объектной модели: создание значения явно отказывает.

use super::*;

pub(crate) static QUERY_SCHEMA_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "СхемаЗапроса",
    type_display: "Query schema",
    type_names: &["QuerySchema"],
};

pub(crate) static CONFIGURATION_UPDATE_DESCRIPTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОписаниеОбновленияКонфигурации",
    type_display: "Configuration update description",
    type_names: &["ConfigurationUpdateDescription"],
};

pub(crate) fn new_query_schema(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель схемы запроса не реализована".into(),
    ))
}

pub(crate) fn new_query_schema_expression(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Выражение участвует в построении схемы запроса; пустой объект скрыл бы
    // отсутствие её объектной модели и изменил результат работы конфигурации.
    Err(RtError::DynamicError(
        "объектная модель выражения схемы запроса не реализована".into(),
    ))
}

pub(crate) fn new_object_deletion(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Объект создаётся для последующего `Записать`, который удаляет запись
    // базы. Заглушка с успешным вызовом скрыла бы несделанное удаление.
    Err(RtError::DynamicError(
        "удаление объекта через текущий сеанс пока не поддерживается".into(),
    ))
}

pub(crate) fn new_configuration_metadata_object(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    // Двоичные данные `.cf` или `.cfe` описывают самостоятельную конфигурацию;
    // метаданные текущей базы не могут заменять результат их разбора.
    Err(RtError::DynamicError(
        "разбор файла конфигурации из двоичных данных пока не поддерживается".into(),
    ))
}

pub(crate) fn new_configuration_update_description(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "разбор описания обновления конфигурации из CFU не реализован".into(),
    ))
}

pub(crate) fn new_report_builder(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель построителя отчёта не реализована".into(),
    ))
}

pub(crate) fn new_query_builder(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Обычный `Запрос` не хранит отбор, порядок и выбранные поля построителя.
    Err(RtError::DynamicError(
        "объектная модель построителя запроса не реализована".into(),
    ))
}

pub(crate) fn new_data_source_description(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель описания источника данных не реализована".into(),
    ))
}

pub(crate) fn new_configuration_description(
    _: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "разбор описания конфигурации из файла cf/cfe не реализован".into(),
    ))
}

pub(crate) fn new_standard_period(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель стандартного периода не реализована".into(),
    ))
}

pub(crate) static ADJUSTED_POINT_IN_TIME_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МоментВремениСУточнениемПериода",
    type_display: "Point in time with period adjustment",
    type_names: &["PointInTimeWithPeriodAdjustment"],
};

// Объявление позволяет компилировать исходный менеджер обработки, как и
// соседняя граница запроса. Фактическое создание требует объектной модели.
pub(crate) fn new_adjusted_point_in_time(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "объектная модель момента времени с уточнением периода не реализована".into(),
    ))
}
