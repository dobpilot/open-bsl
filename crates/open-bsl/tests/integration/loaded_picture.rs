use open_bsl::{Engine, Value};

#[test]
fn loaded_picture_forms_resolve_but_do_not_return_an_empty_picture() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Возврат Новый Картинка(\"image.png\");",
        "Возврат New Picture(\"image.png\", False);",
    ] {
        engine.compile(source).unwrap();
        let error = engine.new_state().exec(source).unwrap_err().to_string();
        assert!(error.contains("загрузка картинки из файла или двоичных данных не реализована"));
    }
    assert!(engine.compile("Возврат Новый Картинка(1, 2, 3);").is_err());

    let empty = engine
        .new_state()
        .exec("Возврат Новый Картинка();")
        .unwrap();
    let filled = engine
        .new_state()
        .exec("Возврат ЗначениеЗаполнено(Новый Картинка());")
        .unwrap();
    assert_eq!(filled, Value::Boolean(false));
    assert!(matches!(empty, Value::Object(_)));
}
