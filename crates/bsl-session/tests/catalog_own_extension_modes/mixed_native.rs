use super::*;

// Проверяется только собственное событие из смешанного нативного контроля.
// Этот тест не подтверждает исполнение заимствованных перехватчиков.
#[test]
fn own_event_matches_native_observations_with_both_extensions_applied() {
    let source = include_str!(
        "../../../../docs/research/fixtures/catalog-new-code-mixed-1018/unique-role-effective-CodeProbe-xml/Catalogs/CP_ЗамерКода/Ext/ObjectModule.bsl"
    );
    let measure =
        include_str!("../../../../docs/research/fixtures/catalog-new-code-mixed-1018/measure.bsl");
    let helper = measure.split("\nПопытка\n").next().unwrap();
    let script = format!(
        r#"{helper}
Функция Проверить() Экспорт
    РезультатыПробы = Новый Массив;
    Для НомерПробы = 0 По 3 Цикл
        РежимПробы = НомерПробы = 1 Или НомерПробы = 3;
        ПодавлениеПробы = НомерПробы >= 2;
        ОбъектПробы = Справочники.CP_ЗамерКода.СоздатьЭлемент();
        МеткаПробы = "own." + Строка(НомерПробы);
        РезультатыПробы.Добавить(МеткаПробы + ".restored=" + ПроверитьКодПодавление(ОбъектПробы, РежимПробы, ПодавлениеПробы));
        Для Каждого СтрокаПробы Из ОбъектПробы.ПрочитатьКод() Цикл
            РезультатыПробы.Добавить(МеткаПробы + ".handler=" + СтрокаПробы);
        КонецЦикла;
        РезультатыПробы.Добавить(МеткаПробы + ".code=" + ОбъектПробы.Код);
        РезультатыПробы.Добавить(МеткаПробы + ".description=" + ОбъектПробы.Наименование);
        РезультатыПробы.Добавить(МеткаПробы + ".outer=" + ?(БезопасныйРежим(), "YES", "NO") + ":" + ?(ПолучитьОтключениеБезопасногоРежима(), "YES", "NO"));
    КонецЦикла;
    Возврат РезультатыПробы;
КонецФункции"#
    );
    let profiles = [
        (
            false,
            include_str!(
                "../../../../docs/research/fixtures/catalog-new-code-mixed-1018/unique-role-own-no-borrowed-no.platform.txt"
            ),
        ),
        (
            false,
            include_str!(
                "../../../../docs/research/fixtures/catalog-new-code-mixed-1018/unique-role-own-no-borrowed-yes.platform.txt"
            ),
        ),
        (
            true,
            include_str!(
                "../../../../docs/research/fixtures/catalog-new-code-mixed-1018/unique-role-own-yes-borrowed-no.platform.txt"
            ),
        ),
        (
            true,
            include_str!(
                "../../../../docs/research/fixtures/catalog-new-code-mixed-1018/unique-role-own-yes-borrowed-yes.platform.txt"
            ),
        ),
    ];
    for (safe, native) in profiles {
        let expected = native
            .lines()
            .filter(|line| line.starts_with("own."))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(expected.lines().count(), 20);
        let factory = factory_with_source(safe, source);
        for user in ["first", "second"] {
            let mut session = factory
                .open(
                    Principal::new(user, "Тест", vec![]),
                    SessionResources::new(Box::new(()), 1),
                )
                .unwrap();
            let module = session.instantiate(&script).unwrap();
            let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
            else {
                panic!("ожидался массив результатов");
            };
            let BslObject::Array(result) = result.as_ref() else {
                panic!("ожидался массив результатов");
            };
            let actual = result
                .borrow()
                .iter()
                .map(|value| format_value(value, None).unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(actual, expected, "safe={safe}, principal={user}");
        }
    }
}
