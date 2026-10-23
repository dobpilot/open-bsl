use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogStandardAttributeFillChecking, DocumentNumberSettings, DocumentNumberType, FillChecking,
    MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use std::sync::Arc;

#[test]
fn required_document_attribute_replays_the_native_checked_path_order() {
    use bsl_orm::DocumentAttribute;
    use open_bsl::{ColumnType, TypeId, TypeRef, Value};

    let factory = factory(document().with_document_attributes(vec![
        DocumentAttribute::new(
            "required",
            "Заполняемый",
            vec![ColumnType::plain(TypeRef::Native(TypeId::String))],
        )
        .with_fill_checking(FillChecking::ShowError),
    ]));
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_system_language("en"),
        )
        .unwrap();
    // Меняются только имя объекта стенда и приёмник контрольного вывода.
    let source = include_str!(
        "../../../../docs/research/fixtures/document-check-filling/measure-own-attribute.bsl"
    )
    .replace("Документы.ЗамерДокумента", "Документы.Проба")
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session
        .instantiate(&format!(
            "Функция Проверить() Экспорт\nРезультатыПробы = Новый Массив;\n{source}\nВозврат СтрСоединить(РезультатыПробы, Символы.ПС);\nКонецФункции"
        ))
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(
            include_str!(
                "../../../../docs/research/fixtures/document-check-filling/measure-own-attribute.platform.txt"
            )
            .trim_end()
            .into()
        )
    );
}

fn factory(document: MetadataObject) -> SessionFactory {
    factory_with_source(
        document,
        include_str!(
            "../../../../docs/research/fixtures/document-check-filling/path-mutation-object.bsl"
        ),
    )
}

fn factory_with_source(document: MetadataObject, source: &str) -> SessionFactory {
    let object_module = "Документ.Проба.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new(
        "check-filling",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат Документы.Проба.СоздатьДокумент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                object_module,
                source,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("doc"),
        ],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "check-filling",
        vec![document.with_object_module(object_module)],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn document() -> MetadataObject {
    MetadataObject::new("doc", "Проба", MetadataKind::Document)
        .with_document_attributes(vec![])
        .with_document_tabular_sections(vec![])
        .with_document_standard_fill_checking(vec![CatalogStandardAttributeFillChecking {
            name: "Date".into(),
            fill_checking: FillChecking::ShowError,
        }])
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
}

#[test]
fn document_exported_variables_are_isolated_between_objects_and_sessions() {
    use open_bsl::Value;

    let factory = factory_with_source(
        document(),
        include_str!("../../../../docs/research/fixtures/document-copy/object-module.bsl"),
    );
    let mut first = factory
        .open(
            Principal::new("first", "Первый", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let mut second = factory
        .open(
            Principal::new("second", "Второй", vec![]),
            SessionResources::new(Box::new(()), 2),
        )
        .unwrap();
    let source = r#"
        Перем ОбъектПробы;
        Функция Прочитать() Экспорт
            Возврат ОбъектПробы.ЗНАЧЕНИЕМОДУЛЯПРОБЫ;
        КонецФункции
        Процедура Изменить(Значение) Экспорт
            ОбъектПробы.значениемодуляпробы = Значение;
        КонецПроцедуры
        Функция ДругойОбъект() Экспорт
            Возврат Документы.Проба.СоздатьДокумент().ЗначениеМодуляПробы;
        КонецФункции
        ОбъектПробы = Документы.Проба.СоздатьДокумент();
    "#;
    let first_module = first.instantiate(source).unwrap();
    let second_module = second.instantiate(source).unwrap();
    first
        .call(&first_module, "Изменить", vec![Value::number_from_i64(99)])
        .unwrap();
    assert_eq!(
        first.call(&first_module, "Прочитать", vec![]).unwrap().0,
        Value::number_from_i64(99)
    );
    assert_eq!(
        first.call(&first_module, "ДругойОбъект", vec![]).unwrap().0,
        Value::number_from_i64(5)
    );
    assert_eq!(
        second.call(&second_module, "Прочитать", vec![]).unwrap().0,
        Value::number_from_i64(5)
    );
}

#[test]
fn document_copy_date_matches_the_native_receipt_and_posted_state_is_fresh() {
    use open_bsl::Value;

    let factory = factory_with_source(
        document(),
        include_str!("../../../../docs/research/fixtures/document-copy/object-module.bsl"),
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!("../../../../docs/research/fixtures/document-copy/measure-date.bsl")
        .replace("Документы.ЗамерДокумента", "Документы.Проба")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session
        .instantiate(&format!(
            "Функция Проверить() Экспорт\nРезультатыПробы = Новый Массив;\n{source}\nВозврат СтрСоединить(РезультатыПробы, Символы.ПС);\nКонецФункции\n\
             Функция ПроверитьПроведение() Экспорт\n\
             Исходник = Документы.Проба.СоздатьДокумент();\n\
             Исходник.Проведен = Истина;\n\
             Копия = Исходник.Copy();\n\
             Возврат Исходник.Проведен И Не Копия.Проведен;\n\
             КонецФункции"
        ))
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(
            include_str!(
                "../../../../docs/research/fixtures/document-copy/measure-date.platform.txt"
            )
            .trim_end()
            .into()
        )
    );
    assert_eq!(
        session
            .call(&module, "ПроверитьПроведение", vec![])
            .unwrap()
            .0,
        Value::Boolean(true)
    );
}

#[test]
fn document_copy_replays_the_complete_native_standard_state_receipt() {
    use bsl_orm::DocumentAttribute;
    use open_bsl::{ColumnType, TypeId, TypeRef, Value};

    let metadata = document().with_document_attributes(vec![DocumentAttribute::new(
        "attribute",
        "Заполняемый",
        vec![ColumnType::plain(TypeRef::Native(TypeId::String))],
    )]);
    let factory = factory_with_source(
        metadata,
        include_str!("../../../../docs/research/fixtures/document-copy/object-module.bsl"),
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source =
        include_str!("../../../../docs/research/fixtures/document-copy/measure-baseline.bsl")
            .replace("Документы.ЗамерДокумента", "Документы.Проба")
            .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session
        .instantiate(&format!(
            "Функция Проверить() Экспорт\nРезультатыПробы = Новый Массив;\n{source}\nВозврат СтрСоединить(РезультатыПробы, Символы.ПС);\nКонецФункции"
        ))
        .unwrap();
    // Нормализуем язык представления типа и имя объекта тестового каталога.
    let expected = include_str!(
        "../../../../docs/research/fixtures/document-copy/measure-baseline.platform.txt"
    )
    .replace("Document Object: ЗамерДокумента", "Документ объект: Проба");
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(expected.trim_end().into())
    );
}

#[test]
fn document_tabular_checked_paths_replay_both_complete_native_receipts() {
    use bsl_orm::{DocumentAttribute, DocumentTabularSection};
    use open_bsl::{ColumnType, TypeId, TypeRef, Value};

    for (policy, source, expected) in [
        (
            FillChecking::DontCheck,
            include_str!(
                "../../../../docs/research/fixtures/document-check-filling/measure-tabular-column.bsl"
            ),
            include_str!(
                "../../../../docs/research/fixtures/document-check-filling/measure-tabular-column.platform.txt"
            ),
        ),
        (
            FillChecking::ShowError,
            include_str!(
                "../../../../docs/research/fixtures/document-check-filling/measure-tabular-required.bsl"
            ),
            include_str!(
                "../../../../docs/research/fixtures/document-check-filling/measure-tabular-required.platform.txt"
            ),
        ),
    ] {
        let section = DocumentTabularSection::new("section", "Запасы")
            .with_fill_checking(policy)
            .with_attributes(vec![
                DocumentAttribute::new(
                    "column",
                    "НомерВариантаКП",
                    vec![ColumnType::plain(TypeRef::Native(TypeId::Number))],
                )
                .with_fill_checking(FillChecking::ShowError),
            ]);
        let factory = factory(document().with_document_tabular_sections(vec![section]));
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_system_language("en"),
            )
            .unwrap();
        // Сохраняем весь нативный сценарий, включая наблюдение массива обработчика.
        let source = source
            .replace("Документы.ЗамерДокумента", "Документы.Проба")
            .replace("Сообщить(", "РезультатыПробы.Добавить(");
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт\nРезультатыПробы = Новый Массив;\n{source}\nВозврат СтрСоединить(РезультатыПробы, Символы.ПС);\nКонецФункции"
            ))
            .unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Str(expected.trim_end().into())
        );
    }
}

#[test]
fn document_check_filling_matches_the_native_standard_field_and_handler_control() {
    let factory = factory(document());
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_system_language("en"),
        )
        .unwrap();
    let module = session.instantiate(r#"
Функция Проверить() Экспорт
    ОбъектПробы = Документы.Проба.СоздатьДокумент();
    Пустой = ОбъектПробы.ПроверитьВКонтексте();
    ОбъектПробы.Дата = Дата(2026, 10, 11);
    Заполненный = ОбъектПробы.CheckFilling();
    ОбъектПробы.НастроитьПроверку(Истина);
    Отказанный = ОбъектПробы.ПроверитьЗаполнение();
    Возврат ?(Пустой, "1", "0") + ?(Заполненный, "1", "0") + ?(Отказанный, "1", "0") + "|" + ОбъектПробы.ПрочитатьПроверку();
КонецФункции
"#).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str("010|3|Date;".into())
    );
}

#[test]
fn tabular_checks_match_the_native_column_and_section_controls() {
    use bsl_orm::{DocumentAttribute, DocumentTabularSection};
    use open_bsl::{ColumnType, TypeId, TypeRef, Value};
    for (policy, expected) in [
        (FillChecking::DontCheck, "1011"),
        (FillChecking::ShowError, "0011"),
    ] {
        let section = DocumentTabularSection::new("section", "Запасы")
            .with_fill_checking(policy)
            .with_attributes(vec![
                DocumentAttribute::new(
                    "column",
                    "НомерВариантаКП",
                    vec![ColumnType::plain(TypeRef::Native(TypeId::Number))],
                )
                .with_fill_checking(FillChecking::ShowError),
            ]);
        let factory = factory(document().with_document_tabular_sections(vec![section]));
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_system_language("en"),
            )
            .unwrap();
        let module = session
            .instantiate(
                r#"
Функция Проверить() Экспорт
    ОбъектПробы = Документы.Проба.СоздатьДокумент();
    ОбъектПробы.Дата = Дата(2026, 10, 11);
    Итог = ?(ОбъектПробы.ПроверитьЗаполнение(), "1", "0");
    СтрокаПробы = ОбъектПробы.Запасы.Добавить();
    Итог = Итог + ?(ОбъектПробы.ПроверитьЗаполнение(), "1", "0");
    СтрокаПробы.НомерВариантаКП = 1;
    Итог = Итог + ?(ОбъектПробы.ПроверитьЗаполнение(), "1", "0");
    СтрокаПробы.НомерВариантаКП = 0;
    ОбъектПробы.НастроитьСписок("clear");
    Возврат Итог + ?(ОбъектПробы.ПроверитьЗаполнение(), "1", "0");
КонецФункции
"#,
            )
            .unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Str(expected.into())
        );
    }
}

#[test]
fn handler_mutation_preserves_the_original_checked_array() {
    let factory = factory(document());
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_system_language("en"),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
Функция Проверить() Экспорт
    ОбъектПробы = Документы.Проба.СоздатьДокумент();
    ОбъектПробы.НастроитьСписок("clear");
    Итог = ?(ОбъектПробы.ПроверитьЗаполнение(), "1", "0");
    ОбъектПробы.НастроитьСписок("replace");
    Возврат Итог + ?(ОбъектПробы.ПроверитьЗаполнение(), "1", "0");
КонецФункции
"#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str("10".into())
    );
}

#[test]
fn unknown_standard_policy_does_not_become_an_empty_check() {
    let document = MetadataObject::new("doc", "Проба", MetadataKind::Document)
        .with_document_attributes(vec![])
        .with_document_tabular_sections(vec![])
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    let factory = factory(document);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().ПроверитьЗаполнение(); КонецФункции").unwrap();
    let error = session.call(&module, "Проверить", vec![]).unwrap_err();
    assert!(
        format!("{error:?}")
            .contains("настройки проверки заполнения документа не приобретены полностью")
    );
}

#[test]
fn existing_matching_object_module_export_remains_callable() {
    let factory = factory_with_source(
        document(),
        "Функция ПроверитьЗаполнение() Экспорт Возврат \"export\"; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().ПроверитьЗаполнение(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str("export".into())
    );
}

#[test]
fn russian_system_language_supports_the_measured_standard_check() {
    let factory = factory(document());
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_system_language("ru"),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().ПроверитьЗаполнение(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Boolean(false)
    );
}

#[test]
fn document_copy_from_its_object_module_resets_native_standard_fields() {
    let source = include_str!("../../../../docs/research/fixtures/document-copy/object-module.bsl")
        .replace("ЗначениеМодуляПробы = 5;", "Функция ЗначениеПробы() Экспорт Возврат ЗначениеМодуляПробы; КонецФункции Процедура ИзменитьПробу(Значение) Экспорт ЗначениеМодуляПробы = Значение; КонецПроцедуры ЗначениеМодуляПробы = 5;");
    let factory = factory_with_source(document(), &source);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(r#"
Функция Проверить() Экспорт
    Исходник = Документы.Проба.СоздатьДокумент();
    Исходник.Номер = "original";
    Исходник.Дата = Дата(2020, 1, 2);
    Исходник.ИзменитьПробу(8);
    Копия = Исходник.СкопироватьВКонтексте();
    Возврат Копия.Номер + "|" + ?(Копия.Дата = Дата(1, 1, 1), "1", "0") + "|" + Строка(Копия.ЗначениеПробы()) + "|" + Строка(Исходник.ЗначениеПробы());
КонецФункции
"#).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str("|1|5|8".into())
    );
}

#[test]
fn document_copy_rows_match_the_native_independence_control() {
    use bsl_orm::{DocumentAttribute, DocumentTabularSection};
    use open_bsl::{ColumnType, TypeId, TypeRef, Value};
    let section = DocumentTabularSection::new("section", "Запасы").with_attributes(vec![
        DocumentAttribute::new(
            "column",
            "НомерВариантаКП",
            vec![ColumnType::plain(TypeRef::Native(TypeId::Number))],
        ),
    ]);
    let factory = factory_with_source(
        document().with_document_tabular_sections(vec![section]),
        include_str!("../../../../docs/research/fixtures/document-copy/object-module.bsl"),
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
Функция Проверить() Экспорт
    Исходник = Документы.Проба.СоздатьДокумент();
    Исходник.Запасы.Добавить().НомерВариантаКП = 7;
    Копия = Исходник.Copy();
    Результат = Строка(Копия.Запасы.Количество()) + "|" + Строка(Копия.Запасы[0].НомерВариантаКП);
    Копия.Запасы[0].НомерВариантаКП = 9;
    Результат = Результат + "|" + Строка(Исходник.Запасы[0].НомерВариантаКП);
    Копия.Запасы.Очистить();
    Возврат Результат + "|" + Строка(Исходник.Запасы.Количество());
КонецФункции
"#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str("1|7|7|1".into())
    );
}

#[test]
fn document_copy_handler_matches_the_native_source_and_fresh_module_control() {
    use bsl_orm::DocumentAttribute;
    use open_bsl::{ColumnType, TypeId, TypeRef, Value};
    let metadata = document().with_document_attributes(vec![DocumentAttribute::new(
        "attribute",
        "Заполняемый",
        vec![ColumnType::plain(TypeRef::Native(TypeId::String))],
    )]);
    let source = include_str!("../../../../docs/research/fixtures/document-copy/handler-object.bsl")
        .replace("ЗначениеМодуляПробы = 5;", "Функция СледПробы() Экспорт Возврат СледКопированияПробы; КонецФункции ЗначениеМодуляПробы = 5;");
    let factory = factory_with_source(metadata, &source);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
Функция Проверить() Экспорт
    Исходник = Документы.Проба.СоздатьДокумент();
    Исходник.Заполняемый = "source";
    Копия = Исходник.Скопировать();
    Возврат Копия.СледПробы() + "|" + Копия.Заполняемый + "|" + Исходник.Заполняемый;
КонецФункции
"#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str("source|source|5|0|handler|source".into())
    );
}

#[test]
fn matching_document_copy_export_retains_dispatch_precedence() {
    let factory = factory_with_source(
        document(),
        "Функция Скопировать() Экспорт Возврат 42; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().Скопировать(); КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Number(open_bsl::BslNumber::from_i64(42))
    );
}

#[test]
fn document_exported_variables_match_the_native_copy_control() {
    let factory = factory_with_source(
        document(),
        include_str!("../../../../docs/research/fixtures/document-copy/object-module.bsl"),
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
Функция Проверить() Экспорт
    Исходник = Документы.Проба.СоздатьДокумент();
    Исходник.ЗНАЧЕНИЕМОДУЛЯПРОБЫ = 8;
    Копия = Исходник.Скопировать();
    Возврат Строка(Копия.ЗначениеМодуляПробы) + "|" + Строка(Исходник.ЗначениеМодуляПробы);
КонецФункции
"#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str("5|8".into())
    );
}

#[test]
fn document_template_own_context_compiles_and_invocation_fails_explicitly() {
    let factory = factory_with_source(
        document(),
        r#"Функция МакетПробы() Экспорт Возврат ПолучитьМакет("Template"); КонецФункции"#,
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().МакетПробы(); КонецФункции").unwrap();
    let error = session.call(&module, "Проверить", vec![]).unwrap_err();
    assert!(
        format!("{error:?}")
            .contains("Документ.ПолучитьМакет: получение макета пока не поддерживается")
    );
}

#[test]
fn document_template_existing_export_remains_callable() {
    let factory = factory_with_source(
        document(),
        "Функция ПолучитьМакет(Имя) Экспорт Возврат Имя; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(r#"Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().ПолучитьМакет("custom"); КонецФункции"#).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str("custom".into())
    );
}
