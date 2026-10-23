use open_bsl::*;
use std::cell::Cell;
use std::rc::Rc;

fn getter(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    if let Ok(data) = ctx.host_data() {
        let count = data.downcast_ref::<Cell<i64>>().unwrap();
        count.set(count.get() + 1);
    }
    Ok(Value::new_object(GlobalObject))
}
#[derive(Debug)]
struct GlobalObject;
static TYPE: TypeDescriptor = TypeDescriptor::new("globals-test", "ОбъектГлобала");
fn property(_: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::number_from_i64(42))
}
impl ObjectProtocol for GlobalObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        const PROPS: &[PropertyDescriptor] = &[PropertyDescriptor {
            names: &["Ответ"],
            get: property,
            set: None,
        }];
        PROPS
    }
}
const FUNCTION: FunctionDescriptor = FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПолучитьГлобал"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: Some(getter),
};
const GLOBALS: &[GlobalDescriptor] = &[
    GlobalDescriptor {
        names: &["Справочники", "Catalogs"],
        function: FunctionCode::new(1),
    },
    GlobalDescriptor {
        names: &["Метаданные", "Metadata"],
        function: FunctionCode::new(1),
    },
];
const LIBRARY: LibraryDescriptor = LibraryDescriptor::new("globals-test", "1")
    .with_functions(&[FUNCTION])
    .with_globals(GLOBALS);

pub(super) const fn library() -> LibraryDescriptor {
    LIBRARY
}

#[test]
fn globals_are_getters_with_variable_precedence_and_metadata_fallback() {
    let engine = Engine::builder().register_library(LIBRARY).build().unwrap();
    let count = Rc::new(Cell::new(0_i64));
    let mut state = engine.state_builder().host_data(count.clone()).build();
    for name in [
        "Справочники",
        "сПрАвОчНиКи",
        "cATALOGs",
        "Метаданные",
        "mETADATA",
    ] {
        assert_eq!(
            state.exec(&format!("Возврат {name}.Ответ;")).unwrap(),
            Value::number_from_i64(42)
        );
    }
    assert_eq!(count.get(), 5);
    for source in [
        "Перем Справочники; Справочники = 7; Возврат Справочники;",
        "Функция Проба() Справочники = 7; Возврат Справочники; КонецФункции Возврат Проба();",
        "Перем Метаданные; Метаданные = 7; Возврат Метаданные;",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(7));
    }
    assert_eq!(count.get(), 5);
    assert_eq!(
        Engine::builder()
            .build()
            .unwrap()
            .new_state()
            .exec("Возврат Метаданные.ОбщиеМодули.Найти(\"Отсутствует\");")
            .unwrap(),
        Value::Undefined
    );
}

#[test]
fn duplicate_aliases_reserved_names_and_invalid_targets_fail_build() {
    const SECOND: FunctionDescriptor = FunctionDescriptor {
        names: &["ДругойПолучатель"],
        ..FUNCTION
    };
    const OTHER: LibraryDescriptor = LibraryDescriptor::new("other-globals", "1")
        .with_functions(&[SECOND])
        .with_globals(&[GlobalDescriptor {
            names: &["cATALOGS"],
            function: FunctionCode::new(1),
        }]);
    assert!(matches!(
        Engine::builder()
            .register_library(LIBRARY)
            .register_library(OTHER)
            .build(),
        Err(Error::Registry(RegistryError::DuplicateGlobalName(_)))
    ));
    for name in [
        "ЭтотОбъект",
        "ThisObject",
        "АргументыКоманднойСтроки",
        "XDTOFactory",
        "Символы",
        "ТипЗначенияJSON",
        "FileStreams",
        "BackgroundJobs",
    ] {
        let names = Box::leak(vec![name].into_boxed_slice());
        let globals = Box::leak(
            vec![GlobalDescriptor {
                names,
                function: FunctionCode::new(1),
            }]
            .into_boxed_slice(),
        );
        assert!(
            matches!(
                Engine::builder()
                    .register_library(LIBRARY.with_globals(globals))
                    .build(),
                Err(Error::Registry(RegistryError::ReservedGlobalName(_)))
            ),
            "{name}"
        );
    }
    for function in [
        FunctionDescriptor {
            code: FunctionCode::new(2),
            ..FUNCTION
        },
        FunctionDescriptor {
            arity: Arity::exact(1),
            ..FUNCTION
        },
        FunctionDescriptor {
            kind: FunctionKind::Procedure,
            ..FUNCTION
        },
    ] {
        let functions = Box::leak(vec![function].into_boxed_slice());
        assert!(matches!(
            Engine::builder()
                .register_library(LIBRARY.with_functions(functions))
                .build(),
            Err(Error::Registry(RegistryError::InvalidGlobalFunction { .. }))
        ));
    }
}

#[test]
fn undeclared_global_keeps_the_existing_diagnostic() {
    let engine = Engine::builder().build().unwrap();
    let error = engine.compile("Возврат Справочники;").err().unwrap();
    assert!(
        matches!(&error, Error::Semantic(SemaError::UndefinedVariable(name)) if name == "Справочники")
    );
    assert_eq!(
        error.to_string(),
        "ошибка семантики: переменная «Справочники» читается до присваивания"
    );
}
