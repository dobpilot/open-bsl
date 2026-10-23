use super::*;
use open_bsl::{Engine, Error, State};

#[derive(Debug, Default)]
struct Capture {
    calls: RefCell<Vec<Vec<(String, Value)>>>,
}

impl DataAccess for Capture {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константа не запрашивалась".into()))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.calls.borrow_mut().push(
            request
                .parameters()
                .map(|(name, value)| (name.to_owned(), value.clone()))
                .collect(),
        );
        QueryResultData::new(vec!["Проба".into()], vec![])
            .map_err(|failure| RtError::DynamicError(failure.to_string()))
    }
}

fn state(access: Rc<Capture>) -> State {
    Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(
                Arc::new(MetadataCatalog::new("query-parameters", vec![]).unwrap()),
                Principal::new("user", "Проба", vec![]),
            )
            .with_data_access(access),
        ))
        .build()
}

#[test]
fn query_parameters_share_a_live_core_structure() {
    let mut state = state(Rc::new(Capture::default()));
    assert_eq!(state.exec(r#"
        ЗапросПробы = Новый Запрос("ВЫБРАТЬ &П КАК Значение");
        ДоУстановки = ЗапросПробы.Параметры;
        Если ДоУстановки.Количество() <> 0 Тогда Возврат Ложь; КонецЕсли;
        ЗапросПробы.УстановитьПараметр("П", 41);
        Если ТипЗнч(ДоУстановки) <> Тип("Структура") Или ДоУстановки.П <> 41 Тогда Возврат Ложь; КонецЕсли;
        ДоУстановки.Вставить("П", 42);
        Если ЗапросПробы.Parameters.П <> 42 Тогда Возврат Ложь; КонецЕсли;
        ЗапросПробы.SetParameter("П", 43);
        Если ДоУстановки.П <> 43 Тогда Возврат Ложь; КонецЕсли;
        ДоУстановки.Удалить("П");
        ЗапросПробы.Текст = "ВЫБРАТЬ 42 КАК Значение";
        Возврат ЗапросПробы.Параметры = ДоУстановки И ДоУстановки.Количество() = 0;
    "#).unwrap(), Value::Boolean(true));
}

#[test]
fn execution_captures_structure_mutations_and_the_latest_key_spelling() {
    let access = Rc::new(Capture::default());
    let mut state = state(access.clone());
    state
        .exec(
            r#"
        ЗапросПробы = Новый Запрос("ВЫБРАТЬ &П КАК Значение");
        ПараметрыПробы = ЗапросПробы.Параметры;
        ЗапросПробы.УстановитьПараметр("П", 41);
        ЗапросПробы.УстановитьПараметр("п", 42);
        ПараметрыПробы.Вставить("Второй", 7);
        ЗапросПробы.Выполнить();
        ПараметрыПробы.Удалить("п");
        ЗапросПробы.Выполнить();
    "#,
        )
        .unwrap();
    assert_eq!(
        access.calls.borrow().as_slice(),
        [
            vec![
                ("п".into(), Value::number_from_i64(42)),
                ("Второй".into(), Value::number_from_i64(7))
            ],
            vec![("Второй".into(), Value::number_from_i64(7))],
        ]
    );
}

#[test]
fn query_parameter_structures_belong_to_their_own_query() {
    let mut state = state(Rc::new(Capture::default()));
    assert_eq!(
        state
            .exec(
                r#"
        Первый = Новый Запрос; Второй = Новый Запрос;
        Первый.Параметры.Вставить("П", 41);
        Возврат Второй.Параметры.Количество() = 0 И Первый.Параметры <> Второй.Параметры;
    "#
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn the_query_parameters_property_itself_is_read_only() {
    for name in ["Параметры", "Parameters"] {
        let error = state(Rc::new(Capture::default()))
            .exec(&format!(
                "ЗапросПробы = Новый Запрос; ЗапросПробы.{name} = Новый Структура;"
            ))
            .unwrap_err();
        assert!(
            matches!(error, Error::Runtime(RtError::PropertyReadOnly { .. })),
            "{error}"
        );
    }
}
