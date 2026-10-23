use std::{rc::Rc, sync::Arc};

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtResult, Value, format_value};

struct NoReads;

impl DataAccess for NoReads {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("создание ссылки не читает базу");
    }
}

fn factory() -> SessionFactory {
    let manager = "Справочник.ЗамерВременныхТаблиц.МодульМенеджера";
    let code = ConfigurationCodeSnapshot::new("reference-test", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) Проба = Справочники.ЗамерВременныхТаблиц.ПроверитьСсылки(); КонецПроцедуры",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("manager", manager,
            include_str!("../../../../tests/conformance/measure/1c/manager-references-module.bsl"),
            ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("catalog"),
    ]).unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "reference-test",
            vec![
                MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
                    .with_manager_module(manager)
                    .with_reference_type_presentation("Замер временных таблиц"),
                MetadataObject::new("other", "Другой", MetadataKind::Catalog),
            ],
        )
        .unwrap(),
    );
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn session(factory: &SessionFactory) -> Session {
    factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(Rc::new(NoReads)),
        )
        .unwrap()
}

fn evaluate(session: &mut Session, expression: &str) -> Value {
    let module = session
        .instantiate(&format!(
            "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
        ))
        .unwrap();
    session.call(&module, "Проверить", vec![]).unwrap().0
}

#[test]
fn manager_reference_measurement_matches_native_1c() {
    let factory = factory();
    let mut session = session(&factory);
    let result = evaluate(
        &mut session,
        "Справочники.ЗамерВременныхТаблиц.ПроверитьСсылки()",
    );
    let Value::Object(result) = result else {
        panic!("массив результатов");
    };
    let open_bsl::BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    let actual = result
        .borrow()
        .iter()
        .enumerate()
        .map(|(index, value)| {
            format!(
                "manager.reference.{index}={}",
                format_value(value, None).unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let expected = include_str!(
        "../../../../tests/conformance/measure/1c/measure-manager-references.platform.txt"
    );
    assert_eq!(actual, expected.trim());
}

#[test]
fn undefined_creates_a_reference_and_wrong_argument_types_are_rejected() {
    let factory = factory();
    let mut session = session(&factory);
    assert_eq!(
        evaluate(
            &mut session,
            "ЗначениеЗаполнено(Справочники.ЗамерВременныхТаблиц.GetRef(Неопределено))"
        ),
        Value::Boolean(true)
    );
    for argument in ["1", "Null", "\"12345678-1234-4321-abcd-0123456789ab\""] {
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку({argument}); КонецФункции")).unwrap();
        assert!(
            session.call(&module, "Проверить", vec![]).is_err(),
            "{argument}"
        );
    }
}

#[test]
fn empty_reference_presentation_is_empty_and_unavailable_presentation_is_an_error() {
    let factory = factory();
    let mut session = session(&factory);
    assert_eq!(
        evaluate(
            &mut session,
            "Строка(Справочники.ЗамерВременныхТаблиц.EmptyRef())"
        ),
        Value::Str("".into())
    );
    let reference = evaluate(&mut session, "Справочники.ЗамерВременныхТаблиц.GetRef()");
    assert!(
        format_value(&reference, None)
            .unwrap_err()
            .to_string()
            .contains("представления")
    );
}

#[test]
fn reference_identity_includes_its_catalog_and_hashes_equal_values_equally() {
    let factory = factory();
    let mut session = session(&factory);
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            УИД = Новый УникальныйИдентификатор("12345678-1234-4321-abcd-0123456789ab");
            Ссылка = Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(УИД);
            Ключи = Новый Соответствие;
            Ключи.Вставить(Ссылка, 17);
            Возврат Ключи.Получить(Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(УИД)) = 17
                И Ссылка <> Справочники.Другой.ПолучитьСсылку(УИД)
                И ТипЗнч(Ссылка) <> ТипЗнч(Справочники.Другой.ПустаяСсылка())
                И Ссылка.Метаданные().Имя = "ЗамерВременныхТаблиц";
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn catalog_references_and_uuid_survive_value_storage_in_the_session() {
    let factory = factory();
    let mut session = session(&factory);
    assert_eq!(
        evaluate(
            &mut session,
            "Новый ХранилищеЗначения(Справочники.ЗамерВременныхТаблиц.ПустаяСсылка()).Получить() = Справочники.ЗамерВременныхТаблиц.ПустаяСсылка()"
        ),
        Value::Boolean(true)
    );
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            УИД = Новый УникальныйИдентификатор("12345678-1234-4321-abcd-0123456789ab");
            Ссылка = Справочники.ЗамерВременныхТаблиц.ПолучитьСсылку(УИД);
            Копия = Новый ХранилищеЗначения(Ссылка).Получить();
            Возврат Копия = Ссылка И ТипЗнч(Копия) = ТипЗнч(Ссылка)
                И Копия.УникальныйИдентификатор() = УИД
                И Новый ХранилищеЗначения(УИД).Получить() = УИД;
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
