use bsl_orm::{library, value_storage_from_internal};
use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, LibraryDescriptor,
    RtResult, Value,
};

fn load(context: &mut CallContext<'_>, args: &[Value]) -> RtResult<Value> {
    if let Some(Value::Str(text)) = args.first() {
        return value_storage_from_internal(&text.to_utf8_string(), context);
    }
    value_storage_from_internal(
        r##"{"#",51e7a0d2-530b-11d4-b98a-008048da3034,{1,{"N",17}}}"##,
        context,
    )
}

static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ЗагрузитьСнимок"],
    arity: Arity::range(0, 1),
    kind: FunctionKind::Function,
    call: load,
}];

#[test]
fn decoded_contents_use_the_existing_storage_snapshot_semantics() {
    let engine = Engine::builder()
        .register_library(library())
        .register_library(LibraryDescriptor::new("storage-control", "1").with_functions(FUNCTIONS))
        .build()
        .unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
        Хранилище = ЗагрузитьСнимок();
        Первое = Хранилище.Получить();
        Первое[0] = 19;
        Второе = Хранилище.Get();
        Возврат Второе[0] = 17 И Строка(ТипЗнч(Хранилище)) = "Value storage";
    "#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn malformed_or_unsupported_contents_do_not_create_successful_storage() {
    let engine = Engine::builder()
        .register_library(library())
        .register_library(LibraryDescriptor::new("storage-control", "1").with_functions(FUNCTIONS))
        .build()
        .unwrap();
    for text in [
        r#"{"N",17} trailing"#,
        r#"{"B",2}"#,
        r#"{"Unknown",1}"#,
        r##"{"#",11111111-1111-1111-1111-111111111111,{"N",17}}"##,
    ] {
        let escaped = text.replace('"', "\"\"");
        assert!(
            engine
                .new_state()
                .exec(&format!("Возврат ЗагрузитьСнимок(\"{escaped}\");"))
                .is_err()
        );
    }
}
