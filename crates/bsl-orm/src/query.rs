//! `Запрос`, `МенеджерВременныхТаблиц`, результат и выборка запроса.

use super::*;

#[derive(Debug)]
struct QueryValue {
    state: RefCell<QueryState>,
}

#[derive(Debug)]
struct QueryState {
    text: Rc<str>,
    // Та же живая структура доступна через свойство и метод установки параметра.
    parameters: Value,
    temp_tables_manager: Option<Value>,
}

struct TempTablesManagerValue {
    handle: TempTablesManagerHandle,
    data_access: Rc<dyn DataAccess>,
}

impl fmt::Debug for TempTablesManagerValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TempTablesManagerValue")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct QueryResultValue {
    data: Rc<QueryResultData>,
}

#[derive(Debug)]
pub(crate) struct QuerySelectionValue {
    pub(crate) data: Rc<QueryResultData>,
    /// Ноль означает позицию до первой строки; после конца остаётся номер
    /// последней выбранной строки, как в измерении платформы.
    pub(crate) cursor: Cell<usize>,
}

pub(crate) static QUERY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "Запрос",
    type_display: "Query",
    type_names: &["Query"],
};

pub(crate) static TEMP_TABLES_MANAGER_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "МенеджерВременныхТаблиц",
    type_display: "Temporary Table Manager",
    type_names: &["TempTablesManager"],
};

pub(crate) static QUERY_RESULT_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "РезультатЗапроса",
    type_display: "Query result",
    type_names: &["QueryResult"],
};

pub(crate) static QUERY_SELECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ВыборкаИзРезультатаЗапроса",
    type_display: "Selected data from query result",
    type_names: &["QueryResultSelection"],
};

impl ObjectProtocol for QueryValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &QUERY_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        QUERY_METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        QUERY_PROPERTIES
    }
}

impl ObjectProtocol for TempTablesManagerValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TEMP_TABLES_MANAGER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        TEMP_TABLES_MANAGER_METHODS
    }
}

impl ObjectProtocol for QueryResultValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &QUERY_RESULT_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        QUERY_RESULT_METHODS
    }
}

impl ObjectProtocol for QuerySelectionValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &QUERY_SELECTION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        QUERY_SELECTION_METHODS
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        query_selection_field(self, name)
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        if let Value::Str(name) = index {
            return query_selection_field(self, &name.shared_utf8());
        }
        let Value::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        query_selection_value(self, index)
    }
}

pub(crate) fn new_query(_: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let text = match arguments {
        [] => Rc::from(""),
        [Value::Str(text)] => Rc::from(text.to_string()),
        [_] => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый Запрос",
            });
        }
        _ => {
            return Err(RtError::DynamicError(
                "Новый Запрос ожидает не больше одного аргумента".into(),
            ));
        }
    };
    Ok(Value::new_object(QueryValue {
        state: RefCell::new(QueryState {
            text,
            parameters: Value::new_dictionary_structure(Vec::new()),
            temp_tables_manager: None,
        }),
    }))
}

pub(crate) fn new_temp_tables_manager(
    context: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let (principal, data_access) = current_data_access(context)?;
    let handle = data_access.create_temp_tables_manager(&principal, context)?;
    Ok(Value::new_object(TempTablesManagerValue {
        handle,
        data_access,
    }))
}

fn query_text(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(query.state.borrow().text.as_ref().into()))
}

fn query_parameters(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(query.state.borrow().parameters.clone())
}

fn set_query_text(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Str(value) = value else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Запрос.Текст",
        });
    };
    query.state.borrow_mut().text = Rc::from(value.to_string());
    Ok(())
}

fn temp_tables_manager(value: &Value) -> RtResult<&TempTablesManagerValue> {
    value
        .object_ref()
        .and_then(|object| object.downcast_ref::<TempTablesManagerValue>())
        .ok_or_else(|| RtError::TypeError {
            expected: "МенеджерВременныхТаблиц",
            op: "Запрос.МенеджерВременныхТаблиц",
        })
}

fn query_temp_tables_manager(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(query
        .state
        .borrow()
        .temp_tables_manager
        .clone()
        .unwrap_or(Value::Undefined))
}

fn set_query_temp_tables_manager(
    receiver: &dyn ObjectProtocol,
    value: Value,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let manager = temp_tables_manager(&value)?;
    let (_, data_access) = current_data_access(context)?;
    if !Rc::ptr_eq(&manager.data_access, &data_access) {
        return Err(RtError::DynamicError(
            "менеджер временных таблиц принадлежит другому сеансу".into(),
        ));
    }
    query.state.borrow_mut().temp_tables_manager = Some(value);
    Ok(())
}

fn close_temp_tables_manager(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<TempTablesManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let (principal, data_access) = current_data_access(context)?;
    if !Rc::ptr_eq(&manager.data_access, &data_access) {
        return Err(RtError::DynamicError(
            "менеджер временных таблиц принадлежит другому сеансу".into(),
        ));
    }
    data_access.close_temp_tables_manager(manager.handle, &principal, context)?;
    Ok(Value::Undefined)
}

fn query_set_parameter(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let [Value::Str(name), value] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка, Произвольный",
            op: "Запрос.УстановитьПараметр",
        });
    };
    let name = name.to_string();
    let parameters = query.state.borrow().parameters.clone();
    let shapes = context.runtime_shapes();
    let field = shapes.names.intern(&name);
    parameters.structure_insert_spelled(
        field,
        &name,
        value.clone(),
        &mut shapes.shapes,
        &shapes.names,
    )?;
    Ok(Value::Undefined)
}

fn query_execute(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let query = receiver
        .downcast_ref::<QueryValue>()
        .ok_or(RtError::NotAnObject)?;
    let request = {
        let state = query.state.borrow();
        let temp_tables_manager = match &state.temp_tables_manager {
            None => None,
            Some(value) => {
                let manager = temp_tables_manager(value)?;
                Some(manager.handle)
            }
        };
        // Конструктор создаёт структуру, а свойство не позволяет заменить её.
        // Снимок запроса собирается до вызова хоста, не удерживая заимствование.
        let Value::Object(parameters) = &state.parameters else {
            unreachable!("параметры запроса всегда являются структурой");
        };
        let BslObject::Structure(parameters) = parameters.as_ref() else {
            unreachable!("параметры запроса всегда являются структурой");
        };
        let parameters = parameters.borrow();
        let names = &context.runtime_shapes().names;
        let parameters = (0..parameters.len())
            .map(|index| {
                let (field, value) = parameters
                    .entry_at(index)
                    .ok_or(RtError::InvalidBytecode("индекс параметра запроса неверен"))?;
                let name = parameters
                    .spelling_at(index, names)
                    .ok_or(RtError::UnknownField(field))?;
                Ok((Rc::from(name), value))
            })
            .collect::<RtResult<Vec<_>>>()?;
        QueryRequest {
            text: Rc::clone(&state.text),
            parameters: parameters.into(),
            temp_tables_manager,
        }
    };
    let (principal, data_access) = {
        let session = session(context)?;
        let data_access = session.data_access.as_ref().ok_or_else(|| {
            RtError::DynamicError("доступ к данным конфигурации не подключён".into())
        })?;
        (session.principal.clone(), Rc::clone(data_access))
    };
    let Some(data) = data_access.execute_query_result(&request, &principal, context)? else {
        return Ok(Value::Undefined);
    };
    Ok(Value::new_object(QueryResultValue {
        data: Rc::new(data),
    }))
}

fn query_result(receiver: &dyn ObjectProtocol) -> RtResult<&QueryResultValue> {
    receiver
        .downcast_ref::<QueryResultValue>()
        .ok_or(RtError::NotAnObject)
}

fn query_result_is_empty(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(Value::Boolean(query_result(receiver)?.data.rows.is_empty()))
}

fn require_default_traversal(arguments: &[Value], operation: &str) -> RtResult<()> {
    if arguments.is_empty() {
        return Ok(());
    }
    Err(RtError::DynamicError(format!(
        "{operation}: поддерживается только прямой обход по умолчанию"
    )))
}

fn query_result_select(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    require_default_traversal(arguments, "РезультатЗапроса.Выбрать")?;
    Ok(Value::new_object(QuerySelectionValue {
        data: Rc::clone(&query_result(receiver)?.data),
        cursor: Cell::new(0),
    }))
}

fn query_result_unload(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    require_default_traversal(arguments, "РезультатЗапроса.Выгрузить")?;
    query_result_data_to_table(&query_result(receiver)?.data)
}

fn query_result_data_to_table(result: &QueryResultData) -> RtResult<Value> {
    let table = Value::new_table();
    let columns = table.get_field_by_name("Колонки")?;
    for name in result.columns.iter() {
        columns.table_add_column(&Value::Str(name.as_ref().into()), &Value::Undefined)?;
    }
    for values in result.rows.iter() {
        let row = table.table_add_row()?;
        for (name, value) in result.columns.iter().zip(values.iter()) {
            row.set_field_by_name(name, value.clone())?;
        }
    }
    Ok(table)
}

fn optional_array(value: &Value, operation: &'static str) -> RtResult<Option<Value>> {
    match value {
        Value::Undefined => Ok(None),
        Value::Object(object) if matches!(&**object, BslObject::Array(_)) => {
            Ok(Some(value.clone()))
        }
        _ => Err(RtError::TypeError {
            expected: "Массив или Неопределено",
            op: operation,
        }),
    }
}

pub(crate) fn find_by_ref(context: &mut CallContext<'_>, arguments: &[Value]) -> RtResult<Value> {
    let [references, search_area, include_objects, exclude_objects] = arguments else {
        return Err(RtError::InvalidBytecode(
            "НайтиПоСсылкам получила неверное число аргументов",
        ));
    };
    if !matches!(references, Value::Object(object) if matches!(&**object, BslObject::Array(_))) {
        return Err(RtError::TypeError {
            expected: "Массив",
            op: "НайтиПоСсылкам",
        });
    }
    let request = FindByRefRequest {
        references: references.clone(),
        search_area: optional_array(search_area, "НайтиПоСсылкам")?,
        include_objects: optional_array(include_objects, "НайтиПоСсылкам")?,
        exclude_objects: optional_array(exclude_objects, "НайтиПоСсылкам")?,
    };
    let (principal, data_access) = current_data_access(context)?;
    let result = data_access.find_by_ref(&request, &principal, context)?;
    query_result_data_to_table(&result)
}

fn query_selection(receiver: &dyn ObjectProtocol) -> RtResult<&QuerySelectionValue> {
    receiver
        .downcast_ref::<QuerySelectionValue>()
        .ok_or(RtError::NotAnObject)
}

fn query_selection_next(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let selection = query_selection(receiver)?;
    let cursor = selection.cursor.get();
    if cursor < selection.data.rows.len() {
        selection.cursor.set(cursor + 1);
        Ok(Value::Boolean(true))
    } else {
        Ok(Value::Boolean(false))
    }
}

fn query_selection_current_row(selection: &QuerySelectionValue) -> Option<&[Value]> {
    selection
        .cursor
        .get()
        .checked_sub(1)
        .and_then(|index| selection.data.rows.get(index))
        .map(AsRef::as_ref)
}

fn query_selection_value(selection: &QuerySelectionValue, index: usize) -> RtResult<Value> {
    let Some(row) = query_selection_current_row(selection) else {
        return Ok(Value::Undefined);
    };
    row.get(index).cloned().ok_or(RtError::IndexOutOfBounds {
        index: index as i64,
        len: row.len(),
    })
}

fn query_selection_field(selection: &QuerySelectionValue, name: &str) -> RtResult<Value> {
    let index = selection
        .data
        .columns
        .iter()
        .position(|candidate| folded_eq(candidate, name))
        .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
    query_selection_value(selection, index)
}

fn query_selection_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let [Value::Number(index)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Число",
            op: "ВыборкаИзРезультатаЗапроса.Получить",
        });
    };
    let index = index
        .to_i64_exact()
        .and_then(|index| usize::try_from(index).ok())
        .ok_or(RtError::BadIndex)?;
    query_selection_value(query_selection(receiver)?, index)
}

pub(crate) static QUERY_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Текст", "Text"],
        get: query_text,
        set: Some(set_query_text),
    },
    PropertyDescriptor {
        names: &["Параметры", "Parameters"],
        get: query_parameters,
        set: None,
    },
    PropertyDescriptor {
        names: &["МенеджерВременныхТаблиц", "TempTablesManager"],
        get: query_temp_tables_manager,
        set: Some(set_query_temp_tables_manager),
    },
];

pub(crate) static QUERY_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьПараметр", "SetParameter"],
        Arity::exact(2),
        query_set_parameter,
    ),
    MethodDescriptor::new(&["Выполнить", "Execute"], Arity::exact(0), query_execute),
];

pub(crate) static TEMP_TABLES_MANAGER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Закрыть", "Close"],
    Arity::exact(0),
    close_temp_tables_manager,
)
.as_procedure()];

pub(crate) static QUERY_RESULT_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Пустой", "IsEmpty"],
        Arity::exact(0),
        query_result_is_empty,
    ),
    MethodDescriptor::new(
        &["Выбрать", "Select"],
        Arity::range(0, 3),
        query_result_select,
    ),
    MethodDescriptor::new(
        &["Выгрузить", "Unload"],
        Arity::range(0, 1),
        query_result_unload,
    ),
];

pub(crate) static QUERY_SELECTION_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Следующий", "Next"],
        Arity::exact(0),
        query_selection_next,
    ),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), query_selection_get),
];
