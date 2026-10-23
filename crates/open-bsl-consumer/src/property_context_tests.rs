use open_bsl::*;
use std::{cell::Cell, rc::Rc};

const RECEIVER_MODULE: &str = "МодульПолучателя";

fn read(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    ctx.call_configuration_module(
        RECEIVER_MODULE,
        "Читать",
        vec![],
        Some(Value::new_object(super::Meter {
            value: Rc::new(Cell::new(17)),
        })),
    )
    .map(|(value, _)| value)
}

static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПрочестьКонтекстПолучателя"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: read,
}];

fn recipe() -> ModuleGraphRecipe {
    ModuleGraphRecipe { eager_init: false, modules: vec![
        ModuleRecipe {
            name: RECEIVER_MODULE.into(), visibility: ModuleVisibility::Private, imports: vec![],
            source: "Функция Читать() Экспорт Возврат Значение + Вычислить(\"vAlUe\"); КонецФункции".into(),
        },
        ModuleRecipe {
            name: "Рабочий".into(), visibility: ModuleVisibility::Named, imports: vec![],
            source: "Процедура Проверить() Экспорт Если ПрочестьКонтекстПолучателя() <> 34 Тогда ВызватьИсключение \"потерян контекст свойств\"; КонецЕсли; КонецПроцедуры".into(),
        },
    ] }
}

fn engine() -> Engine {
    Engine::builder()
        .register_library(super::meter_library())
        .register_library(
            LibraryDescriptor::new("receiver-property-facade", "1").with_functions(FUNCTIONS),
        )
        .configuration(recipe())
        .configuration_property_context(RECEIVER_MODULE, super::METER_PROPERTIES)
        .build()
        .unwrap()
}

#[test]
fn declared_properties_and_dynamic_aliases_are_available_through_the_facade() {
    let engine = engine();
    let module = engine
        .compile("Возврат ПрочестьКонтекстПолучателя();")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::number_from_i64(34)
    );
    assert!(engine.compile("Возврат Значение;").is_err());
}

#[test]
fn worker_recipe_preserves_receiver_properties() {
    let engine = engine();
    let module = engine
        .compile(
            r#"
        Задание = ФоновыеЗадания.Выполнить("Рабочий.Проверить");
        Задание.ОжидатьЗавершения();
        Задание = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(Задание.УникальныйИдентификатор);
        Если Задание.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда
            ВызватьИсключение Задание.ИнформацияОбОшибке.Описание;
        КонецЕсли;
        Возврат Задание.Состояние = СостояниеФоновогоЗадания.Завершено;
    "#,
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn unknown_and_duplicate_module_property_contexts_are_build_errors() {
    let unknown = Engine::builder()
        .configuration(recipe())
        .configuration_property_context("НетМодуля", super::METER_PROPERTIES)
        .build();
    assert!(matches!(unknown, Err(Error::Configuration(_))));
    let duplicate = Engine::builder()
        .configuration(recipe())
        .configuration_property_context(RECEIVER_MODULE, super::METER_PROPERTIES)
        .configuration_property_context("мОдУлЬпОлУчАтЕлЯ", super::METER_PROPERTIES)
        .build();
    assert!(matches!(duplicate, Err(Error::Configuration(_))));
}

#[test]
fn owned_property_names_survive_their_source_and_worker_reconstruction() {
    let names = vec![String::from("Значение"), String::from("Value")];
    let builder = Engine::builder()
        .register_library(super::meter_library())
        .register_library(
            LibraryDescriptor::new("receiver-property-facade", "1").with_functions(FUNCTIONS),
        )
        .configuration(recipe())
        .configuration_property_names(RECEIVER_MODULE, names.iter().cloned());
    drop(names);
    let engine = builder.build().unwrap();
    let module = engine
        .compile(
            r#"
        Если ПрочестьКонтекстПолучателя() <> 34 Тогда
            ВызватьИсключение "потеряны собственные имена";
        КонецЕсли;
        Задание = ФоновыеЗадания.Выполнить("Рабочий.Проверить");
        Задание.ОжидатьЗавершения();
        Задание = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(Задание.УникальныйИдентификатор);
        Если Задание.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда
            ВызватьИсключение Задание.ИнформацияОбОшибке.Описание;
        КонецЕсли;
        Возврат Истина;
    "#,
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Возврат Значение;").is_err());
}
