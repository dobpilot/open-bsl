use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogStandardAttributes, DataAccess, MetadataCatalog, MetadataKind,
    MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, CallContext, RtResult, Value, format_value};
use std::{cell::Cell, rc::Rc, sync::Arc};

fn factory(source: &str) -> SessionFactory {
    factory_with_origin(source, false)
}
fn factory_with_origin(source: &str, extension: bool) -> SessionFactory {
    typed_factory(source, extension, CatalogCodeType::String)
}
fn typed_factory(source: &str, extension: bool, code_type: CatalogCodeType) -> SessionFactory {
    let name = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("catalog-new-code", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", name, source, ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("catalog").with_extension_source(extension),
    ]).unwrap();
    let object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type,
            code_length: 9,
            description_length: 25,
        });
    let object = if extension {
        object.with_extension_object_module(name)
    } else {
        object.with_object_module(name)
    };
    let catalog = MetadataCatalog::new("catalog-new-code", vec![object]).unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}
struct Adapter {
    calls: Cell<usize>,
}
impl DataAccess for Adapter {
    fn allocate_catalog_code(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        prefix: &str,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        assert_eq!(metadata.id(), "catalog");
        assert!(["first", "second"].contains(&principal.id()));
        assert_eq!(prefix, "H-");
        self.calls.set(self.calls.get() + 1);
        Ok(Value::Str("H-0000001".into()))
    }

    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("код не читает константы")
    }
}
#[test]
fn native_new_code_event_controls_replay_with_a_session_allocator() {
    replay_native_event(
        include_str!("../../../docs/research/fixtures/catalog-new-code/ObjectModule.bsl"),
        include_str!("../../../docs/research/fixtures/catalog-new-code/measure.platform.txt"),
    );
}
#[test]
fn native_english_new_code_event_controls_use_the_same_session_bridge() {
    replay_native_event(
        include_str!("../../../docs/research/fixtures/catalog-new-code/ObjectModule-en.bsl"),
        include_str!("../../../docs/research/fixtures/catalog-new-code/measure-en.platform.txt"),
    );
}
fn replay_native_event(source: &str, expected: &str) {
    let factory = factory(source);
    let script = include_str!("../../../docs/research/fixtures/catalog-new-code/measure.bsl");
    let split = script.find("Попытка\n    ОбъектКодаПробы").unwrap();
    let script = format!(
        "{}\nФункция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{}\nВозврат РезультатыПробы; КонецФункции",
        &script[..split],
        script[split..].replace("Сообщить(", "РезультатыПробы.Добавить(")
    );
    for user in ["first", "second"] {
        let adapter = Rc::new(Adapter {
            calls: Cell::new(0),
        });
        let mut session = factory
            .open(
                Principal::new(user, "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
            )
            .unwrap();
        let module = session.instantiate(&script).unwrap();
        let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
        else {
            panic!("массив")
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив")
        };
        let output = result
            .borrow()
            .iter()
            .map(|v| format_value(v, None).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(output, expected);
        assert_eq!(adapter.calls.get(), 1);
    }
}

struct Unsupported;
impl DataAccess for Unsupported {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("код не читает константы")
    }
}
struct Invalid;
impl DataAccess for Invalid {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("код не читает константы")
    }
    fn allocate_catalog_code(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &str,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Ok(Value::Undefined)
    }
}
#[test]
fn unsupported_and_invalid_allocators_fail_without_a_fabricated_code() {
    let factory = factory("");
    for (adapter, expected) in [
        (
            Rc::new(Unsupported) as Rc<dyn DataAccess>,
            "выдача кода справочника не поддерживается",
        ),
        (
            Rc::new(Invalid) as Rc<dyn DataAccess>,
            "адаптер выдачи кода вернул значение другого типа",
        ),
    ] {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_data_access(adapter),
            )
            .unwrap();
        let module = session.instantiate(r#"Функция Проверить() Экспорт ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Попытка ОбъектПробы.SetNewCode(); Исключение Возврат ОписаниеОшибки() + "|" + ОбъектПробы.Код; КонецПопытки; ВызватьИсключение "ожидалась ошибка"; КонецФункции"#).unwrap();
        let result =
            format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap();
        assert!(result.contains(expected), "{result}");
        assert!(result.ends_with('|'), "{result}");
        let bad_arity = session.instantiate(r#"Функция Проверить() Экспорт ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); ОбъектПробы.SetNewCode(1, 2); КонецФункции"#).unwrap();
        assert!(session.call(&bad_arity, "Проверить", vec![]).is_err());
    }
}
#[test]
fn event_origin_and_errors_preserve_the_callers_security_mode() {
    let source = r#"Процедура ПриУстановкеНовогоКода(СтандартнаяОбработка, Префикс) СтандартнаяОбработка = Ложь; Код = ?(БезопасныйРежим(), "yes", "no"); Если Префикс = "throw" Тогда ВызватьИсключение "ошибка обработчика"; КонецЕсли; КонецПроцедуры"#;
    for extension in [false, true] {
        let mut session = factory_with_origin(source, extension)
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate(r#"Функция Проверить() Экспорт ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); УстановитьБезопасныйРежим(Истина); ОбъектПробы.УстановитьНовыйКод(); Если Не БезопасныйРежим() Тогда ВызватьИсключение "режим после успеха"; КонецЕсли; ОшибкаПерехвачена = Ложь; Попытка ОбъектПробы.SetNewCode("throw"); Исключение ОшибкаПерехвачена = Истина; КонецПопытки; Если Не ОшибкаПерехвачена Или Не БезопасныйРежим() Тогда ВызватьИсключение "режим после ошибки"; КонецЕсли; Возврат ОбъектПробы.Код; КонецФункции"#).unwrap();
        assert_eq!(
            format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap(),
            if extension { "yes" } else { "no" }
        );
    }
}
#[test]
fn custom_new_code_exports_keep_their_own_arity_and_result() {
    for name in ["УстановитьНовыйКод", "SetNewCode"] {
        let factory = factory(&format!(
            "Функция {name}(Знач Первый, Знач Второй) Экспорт Возврат Первый + Второй; КонецФункции"
        ));
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate(&format!("Функция Проверить() Экспорт ОбъектПробы = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); Возврат ОбъектПробы.{name}(3, 4); КонецФункции")).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(7)
        );
    }
}

#[test]
fn numeric_one_parameter_event_replays_native_cancellation_without_an_allocator() {
    let factory = typed_factory(
        include_str!("../../../docs/research/fixtures/catalog-new-code/ObjectModule-numeric.bsl"),
        false,
        CatalogCodeType::Number,
    );
    let mut session = factory
        .open(
            Principal::new("numeric", "Проба", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let script =
        include_str!("../../../docs/research/fixtures/catalog-new-code/measure-numeric.bsl")
            .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {script} Возврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив")
    };
    let output = result
        .borrow()
        .iter()
        .map(|v| format_value(v, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(
        output,
        include_str!(
            "../../../docs/research/fixtures/catalog-new-code/measure-numeric.platform.txt"
        )
    );
}
