use std::sync::Arc;

use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, GraphLimits,
    LibraryDescriptor, ObjectProtocol, RegistryError, RtError, RtResult, SerializedValueGraph,
    TypeDescriptor, TypeId, TypeRef, Value, ValueTypeDescriptor,
};

static OBJECT_TYPE: TypeDescriptor = TypeDescriptor::new("typed-host", "ЗначениеХоста");
const TYPES: &[&TypeDescriptor] = &[&OBJECT_TYPE];

#[derive(Debug)]
struct TypedValue(TypeRef);

impl ObjectProtocol for TypedValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &OBJECT_TYPE
    }

    fn value_type(&self) -> TypeRef {
        self.0.clone()
    }
}

fn make_value(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let ty = context
        .runtime_shapes()
        .resolve_type("CatalogRef.Cat")
        .ok_or_else(|| RtError::UnknownType("CatalogRef.Cat".into()))?;
    Ok(Value::new_object(TypedValue(ty)))
}

fn copy_type(context: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    let shapes = context.runtime_shapes();
    let graph = SerializedValueGraph::capture(args, shapes, &GraphLimits::default())?;
    Ok(graph.materialize(shapes)?.remove(0))
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ЗначениеКонфигурации"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: Some(make_value),
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["КопияТипа"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: Some(copy_type),
    },
];

fn description() -> Arc<ValueTypeDescriptor> {
    Arc::new(ValueTypeDescriptor::new(
        "typed-host",
        "catalog:cat:ref",
        "Cat",
        ["СправочникСсылка.Cat", "CatalogRef.Cat"],
    ))
}

fn engine(ty: Arc<ValueTypeDescriptor>) -> Engine {
    Engine::builder()
        .register_library(
            LibraryDescriptor::new("typed-host", "1")
                .with_functions(FUNCTIONS)
                .with_types(TYPES),
        )
        .register_value_type(ty)
        .build()
        .unwrap()
}

#[test]
fn object_reports_its_registered_value_type_and_explicit_aliases() {
    let engine = engine(description());
    for name in [
        "СправочникСсылка.Cat",
        "CatalogRef.Cat",
        "справочникссылка.cat",
    ] {
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Возврат ТипЗнч(ЗначениеКонфигурации()) = Тип(\"{name}\");"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert_eq!(
        engine
            .new_state()
            .exec("Возврат Строка(ТипЗнч(ЗначениеКонфигурации()));")
            .unwrap(),
        Value::Str("Cat".into())
    );
    for name in ["Cat", "CatalogRef. Cat"] {
        assert!(
            engine
                .new_state()
                .exec(&format!("Возврат Тип(\"{name}\");"))
                .is_err()
        );
    }
    assert_eq!(
        engine.new_state().exec("Возврат Тип(\"Число\");").unwrap(),
        Value::Type(TypeRef::Native(TypeId::Number))
    );
}

#[test]
fn type_value_retains_its_description_after_the_engine_is_dropped() {
    let description = description();
    let weak = Arc::downgrade(&description);
    let value = {
        let engine = engine(description);
        engine
            .new_state()
            .exec("Возврат ТипЗнч(ЗначениеКонфигурации());")
            .unwrap()
    };
    assert!(weak.upgrade().is_some());
    assert_eq!(value.to_string(), "Cat");
    drop(value);
    assert!(weak.upgrade().is_none());
}

#[test]
fn graph_round_trip_uses_a_lookup_name_instead_of_the_printed_name() {
    let engine = engine(description());
    assert_eq!(
        engine
            .new_state()
            .exec("Возврат КопияТипа(ТипЗнч(ЗначениеКонфигурации())) = Тип(\"CatalogRef.Cat\");",)
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn value_type_declarations_cannot_shadow_or_conflict() {
    let cases = [
        ("other", "Число"),
        ("other", "CatalogRef.Cat"),
        ("catalog:cat:ref", "CatalogRef.Other"),
    ];
    for (identity, name) in cases {
        let error = Engine::builder()
            .register_value_type(description())
            .register_value_type(Arc::new(ValueTypeDescriptor::new(
                "typed-host",
                identity,
                "Other",
                [name],
            )))
            .build()
            .err()
            .unwrap();
        assert!(matches!(
            error,
            open_bsl::Error::Registry(
                RegistryError::DuplicateValueTypeName(_)
                    | RegistryError::DuplicateValueTypeIdentity(_)
            )
        ));
    }
    assert!(
        Engine::builder()
            .register_value_type(Arc::new(ValueTypeDescriptor::new(
                "typed-host",
                "",
                "Cat",
                ["CatalogRef.Cat"]
            ),))
            .build()
            .is_err()
    );
}

#[test]
fn declared_value_type_does_not_manufacture_a_constructor() {
    let engine = engine(description());
    let error = engine
        .new_state()
        .exec("Возврат Новый(Тип(\"CatalogRef.Cat\"));")
        .unwrap_err();
    assert!(
        error.to_string().contains("конструктор недоступен"),
        "{error}"
    );
}

#[test]
fn table_column_round_trip_preserves_a_declared_type() {
    let engine = engine(description());
    let value = engine
        .new_state()
        .exec(
            r#"
        Таблица = Новый ТаблицаЗначений;
        Таблица.Колонки.Добавить("Ссылка", Новый ОписаниеТипов("CatalogRef.Cat"));
        Копия = КопияТипа(Таблица);
        Возврат Копия.Колонки[0].ТипЗначения;
    "#,
        )
        .unwrap();
    let Value::Object(object) = value else {
        panic!("описание типов");
    };
    let open_bsl::BslObject::TypeDescription(types) = object.as_ref() else {
        panic!("описание типов");
    };
    assert_eq!(types.len(), 1);
    assert_eq!(types[0], TypeRef::Value(description()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn background_worker_retains_the_engine_type_declarations() {
    let engine = Engine::builder()
        .register_value_type(description())
        .common_module(
            "Работы",
            r#"
            Процедура Проверить(ПереданныйТип) Экспорт
                Если ПереданныйТип <> Тип("CatalogRef.Cat") Тогда
                    ВызватьИсключение "тип не сохранился";
                КонецЕсли;
            КонецПроцедуры
        "#,
        )
        .build()
        .unwrap();
    let module = engine
        .compile_entry(
            r#"
        Аргументы = Новый Массив;
        Аргументы.Добавить(Тип("CatalogRef.Cat"));
        Задание = ФоновыеЗадания.Выполнить("Работы.Проверить", Аргументы);
        Задание = Задание.ОжидатьЗавершенияВыполнения();
        Если Задание.Состояние = СостояниеФоновогоЗадания.ЗавершеноАварийно Тогда
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
