use open_bsl::Engine;

fn method_with_sequential_locals() -> String {
    let mut source = String::from(
        "Процедура Заменить(Число)\n Число = 42;\nКонецПроцедуры\n\
         Функция Проверить(Знач Начало, Конец) Экспорт\n",
    );
    for index in 0..256 {
        let value = match index {
            0 => "Начало".to_string(),
            255 => "Конец".to_string(),
            _ => index.to_string(),
        };
        source.push_str(&format!(" Локальная{index} = {value};\n"));
    }
    source
        .push_str(" Заменить(Локальная255);\n Возврат Локальная0 + Локальная255;\nКонецФункции\n");
    source
}

#[test]
fn sparse_large_method_preserves_by_reference_updates() {
    let engine = Engine::builder()
        .common_module("Проба", &method_with_sequential_locals())
        .build()
        .expect("разреженные локальные слоты помещаются в кадр");
    let entry = engine
        .compile_entry("Возврат Проба.Проверить(5, 9);")
        .unwrap();
    let result = engine.new_state().run(&entry).unwrap();
    assert_eq!(open_bsl::format_value(&result, None).unwrap(), "47");
}

#[test]
fn simultaneously_live_locals_still_exceed_the_frame_limit() {
    let mut source = String::from("Функция Проверить() Экспорт\n");
    for index in 0..256 {
        source.push_str(&format!(" Локальная{index} = {index};\n"));
    }
    source.push_str(" Итог = 0;\n");
    for index in 0..256 {
        source.push_str(&format!(" Итог = Итог + Локальная{index};\n"));
    }
    source.push_str(" Возврат Итог;\nКонецФункции\n");
    let error = Engine::builder()
        .common_module("Проба", &source)
        .build()
        .err()
        .expect("все значения живы одновременно");
    assert!(
        error
            .to_string()
            .contains("слишком много локальных переменных в кадре"),
        "{error}"
    );
}

#[test]
fn branch_local_values_do_not_keep_their_predecessors_live() {
    let mut source = String::from("Функция Проверить() Экспорт\n Итог = 0;\n");
    for index in 0..300 {
        source.push_str(&format!(
            " Если Истина Тогда\n  Локальная{index} = {index};\n  Итог = Итог + Локальная{index};\n КонецЕсли;\n"
        ));
    }
    source.push_str(" Возврат Итог;\nКонецФункции\n");
    let engine = Engine::builder()
        .common_module("Проба", &source)
        .build()
        .expect("значения веток не пересекаются");
    let entry = engine.compile_entry("Возврат Проба.Проверить();").unwrap();
    let result = engine.new_state().run(&entry).unwrap();
    assert_eq!(
        open_bsl::format_value(&result, None)
            .unwrap()
            .replace('\u{a0}', ""),
        "44850"
    );
}
