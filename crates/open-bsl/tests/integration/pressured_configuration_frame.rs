use bsl_compiler::Optimizations;
use open_bsl::Engine;

const SOURCE: &str = include_str!(
    "../../../../docs/research/fixtures/buh-register-pressure-1016/configuration-module.bsl"
);

fn native(key: &str) -> &'static str {
    include_str!(
        "../../../../docs/research/fixtures/buh-register-pressure-1016/native/platform.oracle"
    )
    .trim_start_matches('\u{feff}')
    .lines()
    .find_map(|line| line.strip_prefix(key).and_then(|s| s.strip_prefix('=')))
    .unwrap()
}

fn execute(engine: &Engine, source: &str) -> String {
    let entry = engine.compile_entry(source).unwrap();
    let value = engine.new_state().run(&entry).unwrap();
    open_bsl::format_value(&value, None).unwrap()
}

#[test]
fn pressured_configuration_matches_native_arithmetic_byref_calls_and_shapes() {
    for optimizations in [
        Optimizations::default(),
        Optimizations {
            ssa_regalloc: true,
            ..Optimizations::default()
        },
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("Проба", SOURCE)
            .build()
            .expect("разреженные локали оставляют место временным регистрам");
        assert_eq!(
            execute(&engine, "Возврат Проба.ПроверитьЧисло();"),
            native("arithmetic")
        );
        assert_eq!(
            execute(
                &engine,
                "З=1; Р=Проба.ПроверитьСсылку(З); Возврат Строка(Р)+Символы.ПС+Строка(З);"
            ),
            format!("{}\n{}", native("byref.result"), native("byref.after"))
        );
        assert_eq!(
            execute(&engine, "Возврат Проба.ПроверитьПорядокВызовов();"),
            native("calls")
        );
        assert_eq!(
            execute(&engine, "Возврат Проба.ПроверитьФормы();"),
            native("shapes")
        );
    }
}

#[test]
fn dynamic_debug_and_exception_local_materialization_keeps_the_frame_limit() {
    let source = SOURCE.split("Функция ПроверитьСсылку").next().unwrap();
    let chain = format!("Р={};", ["А0"; 14].join("+"));
    let with_handler = source.replace(
        &chain,
        &format!("Попытка {chain} Исключение Возврат 0; КонецПопытки;"),
    );
    let dynamic = source.replace("А0=3;", "Выполнить(\"А0=3;\"); А0=3;");
    for (source, debug) in [
        (source, true),
        (dynamic.as_str(), false),
        (with_handler.as_str(), false),
    ] {
        let error = Engine::builder()
            .debug_info(debug)
            .common_module("Проба", source)
            .build()
            .err()
            .expect("материализованный кадр нельзя уплотнить");
        assert!(
            error.to_string().contains("слишком много регистров"),
            "{error}"
        );
    }
}

#[test]
fn standalone_register_limit_is_unchanged() {
    let engine = Engine::builder().build().unwrap();
    let error = engine
        .compile(SOURCE)
        .err()
        .expect("прежний предел standalone");
    assert_eq!(
        error.to_string(),
        "ошибка компиляции: слишком много регистров в кадре"
    );
}

#[test]
fn simultaneous_locals_and_scratch_still_fail_explicitly() {
    let mut source = String::from("Функция Проверить() Экспорт\n");
    for index in 0..242 {
        source.push_str(&format!("А{index}={index};\n"));
    }
    source.push_str(&format!("Р={};\n", ["А0"; 14].join("+")));
    for index in 0..242 {
        source.push_str(&format!("Р=Р+А{index};\n"));
    }
    source.push_str("Возврат Р; КонецФункции\n");
    let error = Engine::builder()
        .common_module("Проба", &source)
        .build()
        .err()
        .expect("одновременные значения и временные регистры не помещаются");
    assert!(
        error.to_string().contains("слишком много регистров"),
        "{error}"
    );
}
