//! Глобальные операции над информационной базой: блокировки данных,
//! транзакции, монопольный режим, сервисы интеграции и предопределённые данные.

use super::*;

#[derive(Debug)]
struct IntegrationServicesManager;

#[derive(Debug, Default)]
struct DataLockValue {
    requests: RefCell<Vec<DataLockRequest>>,
}

#[derive(Debug)]
struct DataLockItemValue;

pub(crate) static INTEGRATION_SERVICES_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерСервисовИнтеграции",
    type_display: "Integration services manager",
    type_names: &["IntegrationServicesManager"],
};

pub(crate) static DATA_LOCK_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "БлокировкаДанных",
    type_display: "Блокировка данных",
    type_names: &["DataLock"],
};

pub(crate) static DATA_LOCK_ITEM_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ЭлементБлокировкиДанных",
    type_display: "Элемент блокировки данных",
    type_names: &["DataLockItem"],
};

impl ObjectProtocol for IntegrationServicesManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &INTEGRATION_SERVICES_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        INTEGRATION_SERVICES_MANAGER_METHODS
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        let (principal, data_access) = current_data_access(context)?;
        data_access.integration_service(name, &principal, context)
    }
}

impl ObjectProtocol for DataLockValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_LOCK_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_LOCK_METHODS
    }
}

impl ObjectProtocol for DataLockItemValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_LOCK_ITEM_TYPE
    }
}

pub(crate) fn new_data_lock(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(DataLockValue::default()))
}

fn data_lock_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let lock = receiver
        .downcast_ref::<DataLockValue>()
        .ok_or(RtError::NotAnObject)?;
    match arguments {
        [] => {}
        [Value::Str(lock_space)] => lock.requests.borrow_mut().push(DataLockRequest {
            lock_space: Rc::from(lock_space.to_string()),
        }),
        [_] => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "БлокировкаДанных.Добавить",
            });
        }
        _ => {
            return Err(RtError::DynamicError(
                "БлокировкаДанных.Добавить ожидает не больше одного аргумента".into(),
            ));
        }
    }
    Ok(Value::new_object(DataLockItemValue))
}

fn data_lock_lock(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let lock = receiver
        .downcast_ref::<DataLockValue>()
        .ok_or(RtError::NotAnObject)?;
    let requests = lock.requests.borrow().clone();
    let (principal, data_access) = current_data_access(context)?;
    data_access.lock_data(&requests, &principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn lock_data_for_edit(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "блокировка данных для редактирования не поддерживается адаптером сеанса".into(),
    ))
}

pub(crate) fn unlock_data_for_edit(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Err(RtError::DynamicError(
        "снятие блокировки данных для редактирования не поддерживается адаптером сеанса".into(),
    ))
}

pub(crate) fn erase_infobase_data(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Адаптер информационной базы здесь только читает данные. Удаление
    // всей базы или области не может стать пустой успешной операцией.
    Err(RtError::DynamicError(
        "УдалитьДанныеИнформационнойБазы недоступно в хосте только для чтения".into(),
    ))
}

pub(crate) fn filter_criteria(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Глобальное имя связывает исходный неисполняемый метод конфигурации.
    // Без приобретённого состава нельзя выдавать пустой менеджер критериев
    // или подменять выполнение отбора успешным результатом.
    Err(RtError::DynamicError(
        "критерии отбора конфигурации пока не поддерживаются этим хостом".into(),
    ))
}

pub(crate) fn ws_references(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Имена позволяют связать исходный модуль без переписывания текста.
    // Без приобретённого описания нельзя подменять состав WS-ссылок пустым.
    Err(RtError::DynamicError(
        "WS-ссылки конфигурации пока не поддерживаются этим хостом".into(),
    ))
}

pub(crate) fn get_database_data_size(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Сигнатура связывает неисполняемый обработчик исходной конфигурации.
    // Объём доступных пользователю данных зависит от прав и разделителей;
    // физический размер таблицы не может подменить этот подсчёт.
    Err(RtError::DynamicError(
        "ПолучитьРазмерДанныхБазыДанных: получение размера данных пока не поддерживается".into(),
    ))
}

pub(crate) fn delete_objects(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    // Объявление позволяет компилировать исходный модуль, но не предоставляет
    // хосту только для чтения право удалять данные или менять выходную таблицу.
    Err(RtError::DynamicError(
        "УдалитьОбъекты недоступно в хосте только для чтения".into(),
    ))
}

pub(crate) fn begin_transaction(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.begin_transaction(&principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn commit_transaction(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.commit_transaction(&principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn rollback_transaction(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.rollback_transaction(&principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn transaction_active(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.transaction_active(&principal, context)?,
    ))
}

pub(crate) fn exclusive_mode(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(Value::Boolean(
        data_access.exclusive_mode(&principal, context)?,
    ))
}

pub(crate) fn set_exclusive_mode(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let [Value::Boolean(enabled)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "УстановитьМонопольныйРежим",
        });
    };
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_exclusive_mode(*enabled, &principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn integration_services(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(IntegrationServicesManager))
}

fn set_integration_services_processing(
    receiver: &dyn ObjectProtocol,
    enabled: bool,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<IntegrationServicesManager>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, data_access) = current_data_access(context)?;
    // НЕ ИЗМЕРЕНО(INTEGRATION_SERVICES.PROCESSING): синтакс-помощник задаёт
    // назначение операций, но их побочные эффекты и ошибки ещё не сняты на 1С.
    data_access.set_integration_services_processing(enabled, &principal, context)?;
    Ok(Value::Undefined)
}

fn execute_integration_services_processing(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    set_integration_services_processing(receiver, true, context)
}

fn stop_integration_services_processing(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    set_integration_services_processing(receiver, false, context)
}

pub(crate) fn info_base_predefined_data_update(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    Ok(data_access
        .info_base_predefined_data_update(&principal, context)?
        .value())
}

pub(crate) fn set_info_base_predefined_data_update(
    context: &mut CallContext<'_>,
    arguments: &[Value],
) -> RtResult<Value> {
    let mode = PredefinedDataUpdateMode::from_value(&arguments[0])?;
    let (principal, data_access) = current_data_access(context)?;
    data_access.set_info_base_predefined_data_update(mode, &principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) fn initialize_predefined_data(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    data_access.initialize_predefined_data(&principal, context)?;
    Ok(Value::Undefined)
}

pub(crate) static DATA_LOCK_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(0, 1), data_lock_add),
    MethodDescriptor::new(&["Заблокировать", "Lock"], Arity::exact(0), data_lock_lock),
];

pub(crate) static INTEGRATION_SERVICES_MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ВыполнитьОбработку", "ExecuteProcessing"],
        Arity::exact(0),
        execute_integration_services_processing,
    ),
    MethodDescriptor::new(
        &["ОстановитьОбработку", "StopProcessing"],
        Arity::exact(0),
        stop_integration_services_processing,
    ),
];
