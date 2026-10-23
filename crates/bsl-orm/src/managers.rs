//! Глобальные коллекции менеджеров (`Справочники`, `Документы`, `Регистры…`)
//! и методы самих менеджеров объектов.

use super::*;

#[derive(Debug)]
struct ManagersRoot {
    catalog: Arc<MetadataCatalog>,
    kind: MetadataKind,
}

#[derive(Debug)]
pub(crate) struct ManagerValue {
    pub(crate) catalog: Arc<MetadataCatalog>,
    pub(crate) object_id: Arc<str>,
}

pub(crate) static MANAGERS_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "МенеджерыОбъектов");

pub(crate) static MANAGER_TYPE: TypeDescriptor = TypeDescriptor::new("bsl-orm", "МенеджерОбъекта");

impl ObjectProtocol for ManagersRoot {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MANAGERS_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self
            .catalog
            .object(self.kind, name)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        Ok(Value::new_object(ManagerValue {
            catalog: Arc::clone(&self.catalog),
            object_id: Arc::clone(&object.id),
        }))
    }
}

impl ObjectProtocol for ManagerValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MANAGER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        if self
            .catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| object.kind() == MetadataKind::BusinessProcess)
        {
            BUSINESS_PROCESS_MANAGER_PROPERTIES
        } else {
            MANAGER_PROPERTIES
        }
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("объект менеджера отсутствует в каталоге".into())
            })?;
        if object.kind() == MetadataKind::Enumeration
            && let Some(value) = self.catalog.enumeration_value(&self.object_id, name)
        {
            return enumeration_reference_value(
                Arc::clone(&self.catalog),
                &self.object_id,
                &value.value_id,
            );
        }
        if matches!(
            object.kind(),
            MetadataKind::Catalog
                | MetadataKind::ChartOfCharacteristicTypes
                | MetadataKind::ChartOfAccounts
        ) && let Some(name) = object
            .catalog_predefined_names()
            .iter()
            .find(|known| folded_eq(known, name))
        {
            // GUID предопределённых данных не является UUID записи. Обычный
            // адаптер запроса разрешает ссылку в базе текущего сеанса.
            let prefix = if object.kind() == MetadataKind::ChartOfCharacteristicTypes {
                "ПланВидовХарактеристик"
            } else if object.kind() == MetadataKind::ChartOfAccounts {
                "ПланСчетов"
            } else {
                "Справочник"
            };
            let request = QueryRequest {
                text: format!(
                    "ВЫБРАТЬ ЗНАЧЕНИЕ({prefix}.{}.{name}) КАК Ссылка",
                    object.name()
                )
                .into(),
                parameters: Rc::from([]),
                temp_tables_manager: None,
            };
            let (principal, data_access) = current_data_access(context)?;
            let result = data_access.execute_query(&request, &principal, context)?;
            let value = result
                .rows()
                .first()
                .and_then(|row| row.first())
                .ok_or_else(|| {
                    RtError::DynamicError("предопределённый элемент не вернул ссылку".into())
                })?;
            if matches!(value, Value::Null) {
                return Err(RtError::DynamicError(format!(
                    "{prefix}.{}.{name}. Предопределенный элемент не найден в данных информационной базы",
                    object.name()
                )));
            }
            if reference_matches_owner(value, &self.catalog, object.id(), object.kind()) {
                return Ok(value.clone());
            }
            return Err(RtError::DynamicError(
                "адаптер вернул несовместимый тип предопределённого элемента".into(),
            ));
        }
        get_property_from_table(
            self.property_table(),
            MANAGER_TYPE.name,
            self,
            name,
            context,
        )
    }

    fn set_property(
        &self,
        name: &str,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        if self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .is_some_and(|object| {
                matches!(
                    object.kind(),
                    MetadataKind::Catalog
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfAccounts
                ) && object
                    .catalog_predefined_names()
                    .iter()
                    .any(|known| folded_eq(known, name))
            })
        {
            return Err(RtError::PropertyReadOnly {
                property: name.into(),
                receiver: MANAGER_TYPE.name,
            });
        }
        set_property_from_table(
            self.property_table(),
            MANAGER_TYPE.name,
            self,
            name,
            value,
            context,
        )
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let object = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("объект менеджера отсутствует в каталоге".into())
            })?;
        let methods = manager_methods(object.kind());
        let existing_lookup_export = if (folded_eq(name, "НайтиПоРеквизиту")
            || folded_eq(name, "FindByAttribute")
            || (object.kind() == MetadataKind::Catalog
                && [
                    "НайтиПоКоду",
                    "FindByCode",
                    "НайтиПоНаименованию",
                    "FindByDescription",
                ]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (matches!(
                object.kind(),
                MetadataKind::Catalog
                    | MetadataKind::InformationRegister
                    | MetadataKind::AccumulationRegister
                    | MetadataKind::Report
                    | MetadataKind::ChartOfAccounts
            ) && (folded_eq(name, "ПолучитьМакет") || folded_eq(name, "GetTemplate")))
            || (object.kind() == MetadataKind::Catalog
                && (folded_eq(name, "СоздатьГруппу") || folded_eq(name, "CreateFolder")))
            || (object.kind() == MetadataKind::Document
                && (folded_eq(name, "СоздатьДокумент") || folded_eq(name, "CreateDocument")))
            || (matches!(
                object.kind(),
                MetadataKind::DataProcessor | MetadataKind::Report
            ) && (folded_eq(name, "Создать") || folded_eq(name, "Create")))
            || (object.kind() == MetadataKind::SettingsStorage
                && (folded_eq(name, "Загрузить") || folded_eq(name, "Load")))
            || (matches!(
                object.kind(),
                MetadataKind::Task
                    | MetadataKind::BusinessProcess
                    | MetadataKind::ChartOfCharacteristicTypes
            ) && ["ПустаяСсылка", "EmptyRef", "ПолучитьСсылку", "GetRef"]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (object.kind() == MetadataKind::BusinessProcess
                && (folded_eq(name, "СоздатьБизнесПроцесс")
                    || folded_eq(name, "CreateBusinessProcess")))
            || (object.kind() == MetadataKind::InformationRegister
                && [
                    "Получить",
                    "Get",
                    "СрезПервых",
                    "SliceFirst",
                    "СрезПоследних",
                    "SliceLast",
                    "ПолучитьПоследнее",
                    "GetLast",
                    "Выбрать",
                    "Select",
                ]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (object.kind() == MetadataKind::Constant
                && [
                    "Установить",
                    "Set",
                    "СоздатьМенеджерЗначения",
                    "CreateValueManager",
                ]
                .iter()
                .any(|known| folded_eq(name, known)))
            || (object.kind() == MetadataKind::AccumulationRegister
                && [
                    "СоздатьНаборЗаписей",
                    "CreateRecordSet",
                    "Остатки",
                    "Balance",
                ]
                .iter()
                .any(|known| folded_eq(name, known))))
            && let Some(module) = object.manager_module()
        {
            context.configuration_has_export(module, name)?
        } else {
            false
        };
        if method_exists(methods, name) && !existing_lookup_export {
            return call_method_from_table(
                methods,
                MANAGER_TYPE.name,
                self,
                name,
                arguments,
                context,
            );
        }
        let module = object
            .manager_module()
            .ok_or_else(|| RtError::UnknownMethod {
                method: name.into(),
                receiver: MANAGER_TYPE.name,
            })?;
        context
            .call_configuration_module(
                module,
                name,
                arguments.to_vec(),
                Some(Value::new_object(ManagerValue {
                    catalog: Arc::clone(&self.catalog),
                    object_id: Arc::clone(&self.object_id),
                })),
            )
            .map(|(value, _)| value)
    }

    fn has_method(&self, name: &str) -> bool {
        self.catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| {
                method_exists(manager_methods(object.kind()), name)
                    || object.manager_module().is_some()
            })
    }
}

static CATALOG_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        catalog_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
    MethodDescriptor::new(
        &["СоздатьЭлемент", "CreateItem"],
        Arity::exact(0),
        catalog_object::create,
    ),
    MethodDescriptor::new(
        &["СоздатьГруппу", "CreateFolder"],
        Arity::exact(0),
        catalog_object::create_folder,
    ),
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 4),
        catalog_selection::select,
    ),
    MethodDescriptor::new(
        &["НайтиПоРеквизиту", "FindByAttribute"],
        Arity::range(2, 4),
        manager_find_by_attribute,
    ),
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        catalog_get_template,
    ),
    MethodDescriptor::new(
        &["НайтиПоКоду", "FindByCode"],
        Arity::range(1, 4),
        catalog_standard_search::code,
    ),
    MethodDescriptor::new(
        &["НайтиПоНаименованию", "FindByDescription"],
        Arity::range(1, 4),
        catalog_standard_search::description,
    ),
];

// Получение настоящего макета требует приобретённого ресурса.
// Конечная сигнатура связывает исходный модуль, но не подменяет результат.
fn catalog_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Справочник.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

fn enumeration_empty_ref(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    enumeration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, "")
}

fn enumeration_count(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let count = manager
        .catalog
        .enumeration_values
        .iter()
        .filter(|value| value.owner_id == manager.object_id)
        .count();
    Ok(Value::number_from_i64(count as i64))
}

pub(crate) fn collection_index_number(value: &Value) -> RtResult<BslNumber> {
    Ok(match value {
        Value::Number(number) => number.clone(),
        Value::Str(text) => {
            open_bsl::parse_number(&text.to_string(), &open_bsl::NumberFormat::default())?
        }
        Value::Boolean(value) => BslNumber::from_i64(i64::from(*value)),
        _ => return Err(RtError::BadIndex),
    })
}

fn enumeration_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let number = collection_index_number(&arguments[0])?;
    let index = number
        .trunc_to_scale(0)
        .to_i64_exact()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(RtError::BadIndex)?;
    let value = manager
        .catalog
        .enumeration_values
        .iter()
        .filter(|value| value.owner_id == manager.object_id)
        .nth(index)
        .ok_or(RtError::BadIndex)?;
    enumeration_reference_value(
        Arc::clone(&manager.catalog),
        &manager.object_id,
        &value.value_id,
    )
}

fn enumeration_index(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let value = arguments[0]
        .object_ref()
        .and_then(|object| object.downcast_ref::<EnumerationReferenceValue>())
        .ok_or(RtError::BadIndex)?;
    let ty = manager
        .catalog
        .reference_types
        .iter()
        .find(|(id, _)| id == &manager.object_id)
        .map(|(_, ty)| ty)
        .ok_or(RtError::BadIndex)?;
    if TypeRef::Value(Arc::clone(ty)) != value.value_type() {
        return Err(RtError::BadIndex);
    }
    let index = manager
        .catalog
        .enumeration_values
        .iter()
        .filter(|member| member.owner_id == manager.object_id)
        .position(|member| member.value_id == value.value_id)
        .ok_or(RtError::BadIndex)?;
    Ok(Value::number_from_i64(index as i64))
}

static ENUMERATION_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        enumeration_empty_ref,
    ),
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), enumeration_count),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), enumeration_get),
    MethodDescriptor::new(&["Индекс", "IndexOf"], Arity::exact(1), enumeration_index),
];

pub(crate) static MANAGER_PUBLIC_METHODS: &[MethodDescriptor] = &[
    exchange_plan::METHODS[0],
    exchange_plan::METHODS[2],
    CHART_OF_ACCOUNTS_MANAGER_METHODS[0],
    SETTINGS_STORAGE_MANAGER_METHODS[0],
    business_process_object::MANAGER_METHODS[0],
    accumulation_balance::METHOD,
    CONSTANT_MANAGER_METHODS[0],
    CONSTANT_MANAGER_METHODS[1],
    CONSTANT_MANAGER_METHODS[2],
    ENUMERATION_MANAGER_METHODS[1],
    ENUMERATION_MANAGER_METHODS[2],
    ENUMERATION_MANAGER_METHODS[3],
    CATALOG_MANAGER_METHODS[0],
    CATALOG_MANAGER_METHODS[1],
    CATALOG_MANAGER_METHODS[2],
    CATALOG_MANAGER_METHODS[3],
    CATALOG_MANAGER_METHODS[4],
    CATALOG_MANAGER_METHODS[7],
    CATALOG_MANAGER_METHODS[8],
    INFORMATION_REGISTER_MANAGER_METHODS[0],
    INFORMATION_REGISTER_MANAGER_METHODS[1],
    INFORMATION_REGISTER_MANAGER_METHODS[2],
    INFORMATION_REGISTER_MANAGER_METHODS[3],
    INFORMATION_REGISTER_MANAGER_METHODS[4],
    INFORMATION_REGISTER_MANAGER_METHODS[5],
    INFORMATION_REGISTER_MANAGER_METHODS[6],
    INFORMATION_REGISTER_MANAGER_METHODS[7],
    INFORMATION_REGISTER_MANAGER_METHODS[8],
    DATA_PROCESSOR_MANAGER_METHODS[0],
    DATA_PROCESSOR_MANAGER_METHODS[1],
    DOCUMENT_MANAGER_METHODS[0],
    DOCUMENT_MANAGER_METHODS[1],
    REPORT_MANAGER_METHODS[0],
    REPORT_MANAGER_METHODS[1],
];

// Сигнатура нужна при связывании исходного модуля менеджера обработки.
// Без приобретённых ресурсов выдавать пустой макет было бы неверно.
pub(crate) static DATA_PROCESSOR_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        data_processor_get_template,
    ),
    MethodDescriptor::new(
        &["Создать", "Create"],
        Arity::exact(0),
        data_processor_object::create,
    ),
];

fn data_processor_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Обработка.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

// Сигнатура отчёта проверена отдельно по синтакс-помощнику.
// Декларация не подменяет отсутствующие ресурсы успешным пустым макетом.
static REPORT_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        report_get_template,
    ),
    MethodDescriptor::new(
        &["Создать", "Create"],
        Arity::exact(0),
        data_processor_object::create,
    ),
];

fn report_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Отчет.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

// Задачи и бизнес-процессы используют общие фабрики конфигурационных ссылок.
// Сигнатуры проверены по синтакс-помощнику и нативным контролям.
// Нативный прямой вызов без аргументов возвращает `Неопределено`.
// Именованный макет требует настоящего приобретённого ресурса.
// Замер: `docs/research/fixtures/chart-manager-template/runtime.oracle`.
fn chart_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    if arguments.is_empty() {
        return Ok(Value::Undefined);
    }
    Err(RtError::DynamicError(
        "ПланСчетов.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

static CHART_OF_ACCOUNTS_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::range(0, 1),
        chart_get_template,
    ),
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
];

pub(crate) static CONFIGURATION_REFERENCE_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
];

// Документ имеет собственный контекст менеджера; сигнатура проверена
// отдельно по синтакс-помощнику, а не перенесена по аналогии с обработкой.
static DOCUMENT_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["НайтиПоРеквизиту", "FindByAttribute"],
        Arity::exact(2),
        manager_find_by_attribute,
    ),
    MethodDescriptor::new(
        &["СоздатьДокумент", "CreateDocument"],
        Arity::exact(0),
        document_object::create,
    ),
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        document_get_template,
    ),
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        document_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        configuration_get_ref,
    ),
];

pub(crate) fn document_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "Документ.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

// Сигнатуры платформы нужны и в неисполняемых процедурах исходного модуля.
// До реализации записей нельзя подменять их создание пустым объектом.
static INFORMATION_REGISTER_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["СоздатьМенеджерЗаписи", "CreateRecordManager"],
        Arity::exact(0),
        information_register_create_record_manager,
    ),
    MethodDescriptor::new(
        &["СоздатьНаборЗаписей", "CreateRecordSet"],
        Arity::exact(0),
        information_register_create_record_set,
    ),
    // Метод подтверждён синтакс-помощником именно для менеджера регистра.
    // Сигнатура связывает исходник, но не подменяет отсутствующий ресурс.
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        information_register_get_template,
    ),
    MethodDescriptor::new(
        &["СоздатьКлючЗаписи", "CreateRecordKey"],
        Arity::exact(1),
        information_register_create_record_key,
    ),
    information_register_get::METHOD,
    // Два необязательных аргумента подтверждены синтакс-помощником.
    // Декларация не подменяет неизвестную периодичность и данные регистра.
    MethodDescriptor::new(
        &["СрезПервых", "SliceFirst"],
        Arity::range(0, 2),
        information_register_slice_first,
    ),
    MethodDescriptor::new(
        &["СрезПоследних", "SliceLast"],
        Arity::range(0, 2),
        information_register_slice_last,
    ),
    // Объединение двух сигнатур служит только разрешению исходного модуля.
    // Выбор формы требует приобретённой периодичности и реализации выборки.
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 4),
        information_register_select,
    ),
    // Сигнатура периодического регистра подтверждена нативным контролем.
    // Она разрешает исходник, но не подменяет отсутствующее чтение записи.
    // Замер: `docs/research/fixtures/information-register-last-native-1015/platform.oracle`.
    MethodDescriptor::new(
        &["ПолучитьПоследнее", "GetLast"],
        Arity::range(0, 2),
        information_register_get_last,
    ),
];

fn information_register_get_last(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.ПолучитьПоследнее: чтение последней записи пока не поддерживается".into(),
    ))
}

fn information_register_select(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "выборка регистра сведений пока не поддерживается".into(),
    ))
}

fn information_register_slice_first(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СрезПервых: получение среза пока не поддерживается".into(),
    ))
}

fn information_register_slice_last(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СрезПоследних: получение среза пока не поддерживается".into(),
    ))
}

fn information_register_create_record_key(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // Сигнатура связывает исходный менеджер; без измеренных ключевых полей
    // создавать успешное значение ключа было бы подменой реализации.
    Err(RtError::DynamicError(
        "РегистрСведений.СоздатьКлючЗаписи: создание ключа записи пока не поддерживается".into(),
    ))
}

fn information_register_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "РегистрСведений.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

fn information_register_create_record_manager(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СоздатьМенеджерЗаписи: менеджер записи пока не реализован".into(),
    ))
}

fn information_register_create_record_set(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "РегистрСведений.СоздатьНаборЗаписей: набор записей пока не реализован".into(),
    ))
}

static ACCUMULATION_REGISTER_MANAGER_METHODS: &[MethodDescriptor] = &[
    accumulation_balance::METHOD,
    // Сигнатура платформы связывает и неисполняемые методы менеджера.
    // Без настоящего ресурса получение макета завершается явным отказом.
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        accumulation_register_get_template,
    ),
    MethodDescriptor::new(
        &["СоздатьНаборЗаписей", "CreateRecordSet"],
        Arity::exact(0),
        accumulation_register_create_record_set,
    ),
];

fn accumulation_register_get_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(value) = file_source::receiver_template(receiver, arguments, context)? {
        return Ok(value);
    }
    Err(RtError::DynamicError(
        "РегистрНакопления.ПолучитьМакет: получение макета пока не поддерживается".into(),
    ))
}

fn accumulation_register_create_record_set(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // Сигнатура связывает исходный менеджер. Набор и его операции нельзя
    // подменять успешным пустым объектом до реализации поведения записей.
    Err(RtError::DynamicError(
        "РегистрНакопления.СоздатьНаборЗаписей: набор записей пока не реализован".into(),
    ))
}

fn manager_find_by_attribute(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or(RtError::NotAnObject)?;
    let Value::Str(name) = &args[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "НайтиПоРеквизиту",
        });
    };
    let name = name.to_string();
    let name = if object.kind() == MetadataKind::Document {
        object
            .document_attributes()
            .ok_or_else(|| {
                RtError::DynamicError("собственные реквизиты документа не приобретены".into())
            })?
            .iter()
            .map(DocumentAttribute::name)
            .find(|known| folded_eq(known, &name))
    } else {
        object
            .catalog_attribute_names()
            .ok_or_else(|| {
                RtError::DynamicError("собственные реквизиты справочника не приобретены".into())
            })?
            .iter()
            .map(String::as_str)
            .find(|known| folded_eq(known, &name))
    }
    .ok_or_else(|| RtError::UnknownProperty(name.clone()))?;
    if matches!(&args[1], Value::Object(value) if matches!(value.as_ref(), BslObject::Array(_))) {
        return Err(RtError::TypeError {
            expected: "Значение реквизита",
            op: "НайтиПоРеквизиту",
        });
    }
    let request = FindByAttributeRequest {
        attribute: Rc::from(name),
        value: args[1].clone(),
        parent: args
            .get(2)
            .filter(|value| !matches!(value, Value::Undefined))
            .cloned(),
        owner: args
            .get(3)
            .filter(|value| !matches!(value, Value::Undefined))
            .cloned(),
    };
    let (principal, adapter) = current_data_access(context)?;
    match adapter.find_by_attribute(object, &request, &principal, context)? {
        None => {
            configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, [0; 16])
        }
        Some(value)
            if reference_matches_owner(
                &value,
                &manager.catalog,
                &manager.object_id,
                object.kind(),
            ) =>
        {
            Ok(value)
        }
        Some(_) => Err(RtError::DynamicError(
            "адаптер поиска вернул несовместимую ссылку".into(),
        )),
    }
}

fn manager_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    Ok(Value::Str(object.name().into()))
}

fn constant_value(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let metadata = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .cloned()
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    let (principal, data_access) = {
        let session = session(context)?;
        let data_access = session.data_access.as_ref().ok_or_else(|| {
            RtError::DynamicError("доступ к данным конфигурации не подключён".into())
        })?;
        (session.principal.clone(), Rc::clone(data_access))
    };
    data_access.constant_value(&metadata, &principal, context)
}

fn set_constant_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let metadata = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    let (principal, data_access) = current_data_access(context)?;
    // Успех означает завершённую операцию адаптера, а не замену чтения
    // локальным значением. Адаптер SDK для чтения запись явно отвергает.
    data_access.set_constant_value(metadata, &arguments[0], &principal, context)?;
    Ok(Value::Undefined)
}

static MANAGER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Имя", "Name"],
    get: manager_name,
    set: None,
}];

pub(crate) static BUSINESS_PROCESS_MANAGER_PROPERTIES: &[PropertyDescriptor] = &[
    MANAGER_PROPERTIES[0],
    PropertyDescriptor {
        names: &["ТочкиМаршрута", "RoutePoints"],
        get: route_points::manager_property,
        set: None,
    },
];

fn create_constant_value_manager(
    _: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // Декларация позволяет разрешить исходный модуль, но отдельный объект
    // менеджера значения требует реализации и измерений его операций.
    Err(RtError::DynamicError(
        "менеджер значения константы не поддерживается".into(),
    ))
}

static CONSTANT_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(0), constant_value),
    MethodDescriptor::new(&["Установить", "Set"], Arity::exact(1), set_constant_value),
    MethodDescriptor::new(
        &["СоздатьМенеджерЗначения", "CreateValueManager"],
        Arity::exact(0),
        create_constant_value_manager,
    ),
];

static SETTINGS_STORAGE_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Загрузить", "Load"],
    Arity::range(1, 4),
    settings_storage_load,
)];

fn settings_storage_load(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект менеджера отсутствует в каталоге".into()))?;
    let module = object
        .manager_module()
        .ok_or_else(|| RtError::DynamicError("модуль хранилища настроек не подключён".into()))?;
    let user = match arguments.get(3) {
        None | Some(Value::Undefined) => Value::Str(session(context)?.principal().name().into()),
        Some(Value::Str(name)) if name.to_string().is_empty() => {
            Value::Str(session(context)?.principal().name().into())
        }
        Some(value) => value.clone(),
    };
    // Нативный обработчик выбирается по русскому имени, затем по английскому,
    // независимо от варианта языка и порядка объявлений. Проверка метаданных
    // не запускает модуль; при отсутствии обоих имён сохраняется старая диагностика.
    let handler = if context
        .configuration_function_parameter_count(module, "ОбработкаЗагрузки")?
        .is_none()
        && context
            .configuration_function_parameter_count(module, "LoadProcessing")?
            .is_some()
    {
        "LoadProcessing"
    } else {
        "ОбработкаЗагрузки"
    };
    // Пользовательское хранилище выполняет собственный обработчик; оно не
    // подменяется стандартным хранилищем. Значение возвращается из параметра
    // `Настройки`, как в нативном замере вызовов менеджера и его модуля
    // (`docs/research/fixtures/settings-storage-load/measure.platform.txt`).
    // СОЗНАТЕЛЬНОЕ ОТКЛОНЕНИЕ (решение проекта 2026-10-23): по справке
    // `Загрузить` вызывает `ОбработкаЗагрузки`, и ошибка обработчика, как
    // любого вызываемого кода, выходит из `Загрузить` ловимым исключением.
    // На 8.3.27.2342 платформа вместо этого требует перезапуска (явное
    // исключение и деление на ноль, по снимкам окна подробного отчёта:
    // `docs/research/fixtures/approved-sdk-production-integration-1016/
    // settings-storage-runtime-error-classes-1017/`); это поведение не
    // воспроизводится как возможная ошибка платформы.
    let (_, parameters) = context.call_configuration_handler(
        module,
        handler,
        vec![
            arguments[0].clone(),
            arguments.get(1).cloned().unwrap_or(Value::Undefined),
            Value::Undefined,
            arguments.get(2).cloned().unwrap_or(Value::Undefined),
            user,
        ],
        open_bsl::ConfigurationHandlerEntryMode::Inherit,
    )?;
    Ok(parameters[2].clone())
}

pub(crate) fn manager_methods(kind: MetadataKind) -> &'static [MethodDescriptor] {
    match kind {
        MetadataKind::Enumeration => ENUMERATION_MANAGER_METHODS,
        MetadataKind::Constant => CONSTANT_MANAGER_METHODS,
        MetadataKind::Catalog => CATALOG_MANAGER_METHODS,
        MetadataKind::InformationRegister => INFORMATION_REGISTER_MANAGER_METHODS,
        MetadataKind::AccumulationRegister => ACCUMULATION_REGISTER_MANAGER_METHODS,
        MetadataKind::DataProcessor => DATA_PROCESSOR_MANAGER_METHODS,
        MetadataKind::Report => REPORT_MANAGER_METHODS,
        MetadataKind::Document => DOCUMENT_MANAGER_METHODS,
        MetadataKind::BusinessProcess => business_process_object::MANAGER_METHODS,
        MetadataKind::Task => CONFIGURATION_REFERENCE_MANAGER_METHODS,
        MetadataKind::ExchangePlan => exchange_plan::METHODS,
        MetadataKind::ChartOfAccounts => CHART_OF_ACCOUNTS_MANAGER_METHODS,
        MetadataKind::ChartOfCharacteristicTypes => characteristic_object::MANAGER_METHODS,
        MetadataKind::SettingsStorage => SETTINGS_STORAGE_MANAGER_METHODS,
        _ => &[],
    }
}

pub(crate) fn method_exists(methods: &[MethodDescriptor], name: &str) -> bool {
    methods.iter().any(|method| {
        method
            .names()
            .iter()
            .any(|candidate| folded_eq(candidate, name))
    })
}

fn managers(ctx: &mut CallContext<'_>, kind: MetadataKind) -> RtResult<Value> {
    Ok(Value::new_object(ManagersRoot {
        catalog: Arc::clone(session(ctx)?.catalog()),
        kind,
    }))
}

pub(crate) fn catalogs(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Catalog)
}

pub(crate) fn documents(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Document)
}

pub(crate) fn document_journals(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::DocumentJournal)
}

pub(crate) fn reports(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Report)
}

pub(crate) fn data_processors(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::DataProcessor)
}

pub(crate) fn information_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::InformationRegister)
}

pub(crate) fn accumulation_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::AccumulationRegister)
}

pub(crate) fn accounting_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::AccountingRegister)
}

pub(crate) fn calculation_registers(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::CalculationRegister)
}

pub(crate) fn charts_of_accounts(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::ChartOfAccounts)
}

pub(crate) fn charts_of_characteristic_types(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    managers(ctx, MetadataKind::ChartOfCharacteristicTypes)
}

pub(crate) fn charts_of_calculation_types(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    managers(ctx, MetadataKind::ChartOfCalculationTypes)
}

pub(crate) fn business_processes(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::BusinessProcess)
}

pub(crate) fn tasks(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Task)
}

pub(crate) fn settings_storages(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::SettingsStorage)
}

pub(crate) fn sequences(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Sequence)
}

pub(crate) fn constants(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Constant)
}

pub(crate) fn enumerations(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::Enumeration)
}

pub(crate) fn exchange_plans(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    managers(ctx, MetadataKind::ExchangePlan)
}
