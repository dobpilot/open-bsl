use open_bsl::*;

const MANAGER_MODULE: &str = "МенеджерСправочника";

fn bridge(ctx: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let (_, final_args) = ctx.call_configuration_module(
        MANAGER_MODULE,
        "Изменить",
        args.to_vec(),
        Some(Value::new_object(TestManager)),
    )?;
    Ok(final_args[0].clone())
}

fn manager(_: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(TestManager))
}

#[derive(Debug)]
struct TestManager;

static MANAGER_TYPE: TypeDescriptor = TypeDescriptor::new("configuration-call-test", "Менеджер");

impl ObjectProtocol for TestManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MANAGER_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        if folded_eq(name, "Маркер") {
            Ok(Value::number_from_i64(7))
        } else {
            Err(RtError::UnknownProperty(name.into()))
        }
    }

    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        ctx: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        if folded_eq(name, "Статический") {
            return Ok(Value::number_from_i64(99));
        }
        ctx.call_configuration_module(
            MANAGER_MODULE,
            name,
            args.to_vec(),
            Some(Value::new_object(TestManager)),
        )
        .map(|(value, _)| value)
    }

    fn has_method(&self, _: &str) -> bool {
        true
    }
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["Мост"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(bridge),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ТестовыйМенеджер"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(manager),
    },
];

const TYPES: &[&TypeDescriptor] = &[&MANAGER_TYPE];
const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("configuration-call-test", "1")
    .with_functions(FUNCTIONS)
    .with_types(TYPES);

fn engine() -> Engine {
    Engine::builder()
        .register_library(LIBRARY)
        .common_module(
            MANAGER_MODULE,
            r#"
            Перем Счетчик;
            Счетчик = 0;

            Процедура Изменить(Значение) Экспорт
                Счетчик = Счетчик + 1;
                Значение = Значение + Счетчик;
            КонецПроцедуры

            Функция Прибавить(Значение) Экспорт
                Если ЭтотОбъект.Маркер <> 7 Тогда
                    ВызватьИсключение "неверный ЭтотОбъект";
                КонецЕсли;
                Счетчик = Счетчик + 1;
                Возврат Значение + Счетчик;
            КонецФункции

            Функция Статический() Экспорт
                Возврат 0;
            КонецФункции

            Процедура Ошибка() Экспорт
                Счетчик = Счетчик + 1;
                ВызватьИсключение "ошибка менеджера";
            КонецПроцедуры
            "#,
        )
        .build()
        .unwrap()
}

#[test]
fn component_and_dynamic_object_call_the_same_configuration_module_instance() {
    let engine = engine();
    let entry = engine
        .compile(
            r#"
            Функция ЧерезКомпонент() Экспорт
                Возврат Мост(10);
            КонецФункции
            Функция ЧерезМенеджер() Экспорт
                Возврат ТестовыйМенеджер().Прибавить(10);
            КонецФункции
            Функция СтатическаяПлатформа() Экспорт
                Возврат ТестовыйМенеджер().Статический();
            КонецФункции
            Процедура ОшибкаМенеджера() Экспорт
                ТестовыйМенеджер().Ошибка();
            КонецПроцедуры
            "#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&entry).unwrap();

    assert_eq!(
        instance
            .call(&mut state, "ЧерезКомпонент", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(11)
    );
    assert_eq!(
        instance
            .call(&mut state, "ЧерезМенеджер", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(12)
    );
    assert_eq!(
        instance
            .call(&mut state, "СтатическаяПлатформа", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(99)
    );
    assert!(
        instance
            .call(&mut state, "ОшибкаМенеджера", vec![])
            .unwrap_err()
            .to_string()
            .contains("ошибка менеджера")
    );
    assert_eq!(
        instance
            .call(&mut state, "ЧерезКомпонент", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(14)
    );

    let mut other = engine.new_state();
    let other_instance = other.instantiate(&entry).unwrap();
    assert_eq!(
        other_instance
            .call(&mut other, "ЧерезКомпонент", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(11)
    );
}

#[test]
fn missing_configuration_module_is_an_execution_error() {
    fn missing(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
        ctx.call_configuration_module("НетТакогоМодуля", "Метод", Vec::new(), None)
            .map(|(value, _)| value)
    }
    const FUNCTION: FunctionDescriptor = FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["НетМодуля"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(missing),
    };
    const LIBRARY: LibraryDescriptor =
        LibraryDescriptor::new("missing-module-test", "1").with_functions(&[FUNCTION]);
    let engine = Engine::builder()
        .register_library(LIBRARY)
        .common_module(MANAGER_MODULE, "")
        .build()
        .unwrap();

    let error = engine.new_state().exec("Возврат НетМодуля();").unwrap_err();
    assert!(error.to_string().contains("НетТакогоМодуля"));
}
