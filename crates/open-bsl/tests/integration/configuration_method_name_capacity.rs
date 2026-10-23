use open_bsl::Engine;

#[test]
fn large_catalog_preserves_open_calls_and_cross_module_structure_fields() {
    let mut source = String::from("Функция НеВызывать(О) Экспорт\n");
    for index in 0..65_536 {
        source.push_str(&format!("Р=О.Поле{index};\n"));
    }
    source.push_str(
        "Возврат Р; КонецФункции\n\
         Функция Создать() Экспорт Возврат Новый Структура(\"ОбщееПоле\",42); КонецФункции",
    );
    let engine = Engine::builder()
        .common_module("Первый", &source)
        .common_module(
            "Второй",
            "Функция Прочитать(О) Экспорт Возврат О.Количество()+О.ОбщееПоле; КонецФункции",
        )
        .build()
        .expect("имена свойств другого модуля не исчерпывают индексы методов");
    let entry = engine
        .compile_entry("О=Первый.Создать(); Возврат Второй.Прочитать(О);")
        .unwrap();
    let value = engine.new_state().run(&entry).unwrap();
    assert_eq!(open_bsl::format_value(&value, None).unwrap(), "43");
    let dynamic = engine
        .compile_entry("О=Первый.Создать(); Возврат Вычислить(\"О.ОбщееПоле\");")
        .unwrap();
    let value = engine.new_state().run(&dynamic).unwrap();
    assert_eq!(open_bsl::format_value(&value, None).unwrap(), "42");
}
