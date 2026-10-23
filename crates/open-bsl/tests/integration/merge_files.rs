use open_bsl::{Engine, Value};

#[test]
fn merge_files_has_measured_arity_and_fails_before_writing() {
    let engine = Engine::builder().build().unwrap();
    let path =
        std::env::temp_dir().join(format!("open-bsl-merge-files-{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&path);

    for name in ["ОбъединитьФайлы", "MergeFiles"] {
        let source = format!(
            "Попытка {name}(Новый Массив, \"{}\"); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;",
            path.display()
        );
        let module = engine.compile(&source).unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
        assert!(
            !path.exists(),
            "отказ не должен создавать результирующий файл"
        );
    }

    for call in ["ОбъединитьФайлы(1)", "MergeFiles(1, 2, 3)"] {
        let error = engine
            .compile(&format!("{call};"))
            .err()
            .expect("неверная арность запрещена");
        assert!(error.to_string().contains("аргумент"), "{error}");
    }
}
