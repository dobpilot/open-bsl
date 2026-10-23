use super::*;

#[test]
fn an_existing_balance_export_keeps_its_own_result_and_arity() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .common_module(
            "МенеджерКонтроля",
            "Функция Остатки(Значение) Экспорт Возврат Значение; КонецФункции",
        )
        .build()
        .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![
                MetadataObject::new("register", "Контроль", MetadataKind::AccumulationRegister)
                    .with_manager_module("МенеджерКонтроля"),
            ],
        )
        .unwrap(),
    );
    let session = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    assert_eq!(
        state
            .exec("Возврат РегистрыНакопления.Контроль.Остатки(23);")
            .unwrap(),
        Value::number_from_i64(23)
    );
    assert!(
        state
            .exec("Возврат РегистрыНакопления.Контроль.Остатки();")
            .is_err()
    );
}

#[derive(Debug, Default)]
struct Access {
    calls: RefCell<Vec<QueryRequest>>,
}

impl DataAccess for Access {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(failure("константы не используются"))
    }
    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "user");
        self.calls.borrow_mut().push(request.clone());
        QueryResultData::new(
            vec![
                "ПолеОстатков0".into(),
                "ПолеОстатков1".into(),
                "ПолеОстатков2".into(),
            ],
            vec![vec![
                Value::Str("a".into()),
                Value::Str("a".into()),
                Value::number_from_i64(3),
            ]],
        )
        .map_err(|error| failure(&error.to_string()))
    }
}

fn catalog() -> Arc<MetadataCatalog> {
    let types = |id, quals: &[&str]| {
        vec![open_bsl::ColumnType {
            id: open_bsl::TypeRef::Native(id),
            quals: quals.iter().map(|value| (*value).into()).collect(),
        }]
    };
    Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "register",
                "Контроль",
                MetadataKind::AccumulationRegister,
            )],
        )
        .unwrap()
        .with_accumulation_register_metadata(
            "register",
            Some(AccumulationRegisterKind::Balance),
            Some(vec![DocumentAttribute::new(
                "dimension",
                "Категория",
                types(TypeId::String, &["12", "0"]),
            )]),
            Some(vec![DocumentAttribute::new(
                "resource",
                "Количество",
                types(TypeId::Number, &["15", "2", "0"]),
            )]),
        )
        .unwrap(),
    )
}

#[test]
fn balance_uses_principal_parameters_and_preserves_duplicate_result_columns() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
        .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    let table = state.exec("Возврат РегистрыНакопления.Контроль.Остатки(, Новый Структура(\"Категория\", \"a\"), \"категория, Категория\");").unwrap();
    let Value::Object(object) = table else {
        panic!()
    };
    let BslObject::ValueTable(data) = object.as_ref() else {
        panic!()
    };
    let data = data.borrow();
    assert_eq!(data.column_names, ["Категория", "Категория", "Количество"]);
    assert_eq!(data.row_count(), 1);
    assert_eq!(data.columns[2][0], Value::number_from_i64(3));
    assert_eq!(
        data.column_types[2].as_ref().unwrap()[0].quals,
        ["38", "2", "0"]
    );
    let calls = access.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0]
            .text()
            .contains("КоличествоОстаток КАК ПолеОстатков2")
    );
    assert!(calls[0].text().contains("Категория = &ОтборОстатков0"));
    assert_eq!(
        calls[0].parameters().collect::<Vec<_>>(),
        [("ОтборОстатков0", &Value::Str("a".into()))]
    );
}

#[test]
fn invalid_arguments_fail_before_data_access() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
        .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    for args in [
        "Null",
        "42",
        ", Новый Соответствие",
        ", Новый Структура(\"НетИзмерения\", 1)",
        ", , \"НетИзмерения\"",
        ", , , \"НетРесурса\"",
    ] {
        assert!(
            state
                .exec(&format!(
                    "Возврат РегистрыНакопления.Контроль.Остатки({args});"
                ))
                .is_err()
        );
    }
    assert!(access.calls.borrow().is_empty());
}

#[test]
fn unknown_register_metadata_does_not_produce_a_successful_empty_table() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let catalog = Arc::new(
        MetadataCatalog::new(
            "cfg",
            vec![MetadataObject::new(
                "register",
                "Контроль",
                MetadataKind::AccumulationRegister,
            )],
        )
        .unwrap(),
    );
    let session = SessionData::new(catalog, Principal::new("user", "Тест", vec![]))
        .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    let error = state
        .exec("Возврат РегистрыНакопления.Контроль.Остатки();")
        .unwrap_err();
    assert!(error.to_string().contains("вид регистра неизвестен"));
    assert!(access.calls.borrow().is_empty());
}

#[test]
fn a_date_is_a_query_parameter_and_keeps_its_time() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
        .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    let expected = state.exec("Возврат Дата(2026, 10, 6, 12, 0, 1);").unwrap();
    state.exec("Возврат AccumulationRegisters.Контроль.Balance(Дата(2026, 10, 6, 12, 0, 1), , \"Категория, Категория\");").unwrap();
    let calls = access.calls.borrow();
    assert!(calls[0].text().contains("Остатки(&МоментОстатков, )"));
    assert_eq!(
        calls[0].parameters().collect::<Vec<_>>(),
        [("МоментОстатков", &expected)]
    );
}

#[derive(Debug)]
struct EmptyBalanceAccess;

impl DataAccess for EmptyBalanceAccess {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(failure("константы не используются"))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "user");
        let count = request.text().matches(" КАК ПолеОстатков").count();
        QueryResultData::new(
            (0..count).map(|index| format!("ПолеОстатков{index}")),
            vec![],
        )
        .map_err(|error| failure(&error.to_string()))
    }
}

#[test]
fn all_native_argument_and_duplicate_column_receipts_match_with_empty_data() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    for (source, expected) in [
        (
            include_str!(
                "../../../docs/research/fixtures/accumulation-register-balance/empty-ru.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/accumulation-register-balance/empty-ru.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/accumulation-register-balance/arguments.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/accumulation-register-balance/arguments.platform.txt"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/accumulation-register-balance/duplicate-selection.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/accumulation-register-balance/duplicate-selection.platform.txt"
            ),
        ),
    ] {
        let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
            .with_data_access(Rc::new(EmptyBalanceAccess));
        let mut state = engine.state_builder().host_data(Rc::new(session)).build();
        // Меняются имя контрольного регистра и приёмник вывода, сами пробы сохраняются.
        let source = source
            .replace("ЗамерОстатков", "Контроль")
            .replace("Сообщить(", "РезультатыПробы.Добавить(");
        let result = state
            .exec(&format!(
                "РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы;"
            ))
            .unwrap();
        let Value::Object(object) = result else {
            panic!("массив результатов")
        };
        let BslObject::Array(lines) = object.as_ref() else {
            panic!("массив результатов")
        };
        let actual = lines
            .borrow()
            .iter()
            .map(|value| {
                let Value::Str(line) = value else {
                    panic!("строка результата")
                };
                line.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(actual, expected);
    }
}

#[test]
fn temporal_balance_arguments_reach_the_existing_principal_bound_query_adapter() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let access = Rc::new(Access::default());
    let session = SessionData::new(catalog(), Principal::new("user", "Тест", vec![]))
        .with_data_access(access.clone());
    let mut state = engine.state_builder().host_data(Rc::new(session)).build();
    for expression in [
        "Новый МоментВремени(Дата(2026, 10, 19, 12))",
        "Новый Граница(Дата(2026, 10, 19, 12), ВидГраницы.Исключая)",
        "Новый Граница(Новый МоментВремени(Дата(2026, 10, 19, 12)))",
    ] {
        let expected = state.exec(&format!("Возврат {expression};")).unwrap();
        state
            .exec(&format!(
                "Возврат РегистрыНакопления.Контроль.Остатки({expression}, , \"Категория, Категория\");"
            ))
            .unwrap();
        let calls = access.calls.borrow();
        let call = calls.last().unwrap();
        assert!(call.text().contains("Остатки(&МоментОстатков, )"));
        assert_eq!(
            call.parameters().collect::<Vec<_>>(),
            [("МоментОстатков", &expected)]
        );
    }
}
