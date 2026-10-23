use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn factory(numeric: bool, object_module: bool) -> SessionFactory {
    factory_with_object_source(
        numeric,
        object_module.then_some(include_str!(
            "../../../../tests/conformance/measure/1c/catalog-object-instance-module.bsl"
        )),
    )
}

fn factory_with_object_source(numeric: bool, source: Option<&str>) -> SessionFactory {
    let module_name = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    let mut object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: if numeric {
                CatalogCodeType::Number
            } else {
                CatalogCodeType::String
            },
            code_length: 9,
            description_length: if numeric { 0 } else { 25 },
        })
        .with_catalog_default_presentation(if numeric {
            CatalogDefaultPresentation::AsCode
        } else {
            CatalogDefaultPresentation::AsDescription
        });
    if let Some(source) = source {
        object = object.with_object_module(module_name);
        modules.push(
            ConfigurationModule::new(
                "object",
                module_name,
                source,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("catalog"),
        );
    }
    SessionFactory::new(
        ConfigurationCodeSnapshot::new("object-test", modules).unwrap(),
        Arc::new(MetadataCatalog::new("object-test", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn session(factory: &SessionFactory) -> Session {
    factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap()
}

fn replay(session: &mut Session, source: &str) -> String {
    // Меняется только приёмник сообщений: вычисления остаются нативным сценарием.
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции"
    )).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn string_assignment_and_presentation_match_native_russian_oracle() {
    let factory = factory(false, false);
    let mut session = session(&factory);
    let actual = replay(
        &mut session,
        include_str!("../../../../tests/conformance/measure/1c/measure-catalog-assign-string.bsl"),
    );
    assert_eq!(
        actual,
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-assign-string.ru.platform.txt"
        )
        .trim()
    );
}

#[test]
fn numeric_assignment_matches_native_oracle() {
    let factory = factory(true, false);
    let mut session = session(&factory);
    let actual = replay(
        &mut session,
        include_str!("../../../../tests/conformance/measure/1c/measure-catalog-assign-number.bsl"),
    )
    // Проверяется присваивание числа. Разбиение разрядов зависит от региональных
    // настроек базы; переключение языка интерфейса нативной базы его не меняет.
    .replace('\u{a0}', "");
    assert_eq!(
        actual,
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-assign-number.platform.txt"
        )
        .trim()
    );
}

#[test]
fn concrete_object_module_replays_native_initialization_and_recursive_callbacks() {
    let factory = factory(false, true);
    let expected = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-object-instance.platform.txt"
    )
    .replace("=Yes", "=Да");
    for _ in 0..2 {
        let mut session = session(&factory);
        let actual = replay(
            &mut session,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-catalog-object-instance.bsl"
            ),
        );
        assert_eq!(actual, expected.trim());
    }
}

#[test]
fn numeric_defaults_and_read_only_description_match_native_oracle() {
    let factory = factory(true, false);
    let mut session = session(&factory);
    let actual = replay(
        &mut session,
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-create-item-number.bsl"
        ),
    );
    let expected = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-create-item-number.platform.txt"
    )
    .replace("=Yes", "=Да");
    assert_eq!(actual, expected.trim());
}

#[test]
fn object_metadata_and_type_are_exact_and_live_object_is_not_immutable_storage() {
    let factory = factory(false, true);
    let mut session = session(&factory);
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            Объект = Справочники.ЗамерВременныхТаблиц.CreateItem();
            Если Не Объект.ЭтоНовый() Или ЗначениеЗаполнено(Объект.Ссылка)
                Или Объект.Метаданные().Имя <> "ЗамерВременныхТаблиц"
                Или ТипЗнч(Объект) <> Тип("CatalogObject.ЗамерВременныхТаблиц")
                Или ТипЗнч(Объект) = ТипЗнч(Объект.Ссылка) Тогда
                Возврат Ложь;
            КонецЕсли;
            Попытка
                Копия = Новый ХранилищеЗначения(Объект);
                Возврат Ложь;
            Исключение
                Возврат Истина;
            КонецПопытки;
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn reads_succeed_but_writes_and_is_new_fail_before_selection_matches_native() {
    let native = include_str!(
        "../../../../tests/conformance/measure/1c/catalog-object-properties-module.bsl"
    );
    let (_, body) = native.split_once("ИтогиТелаПробы = Новый Массив;").unwrap();
    let (body, _) = body
        .split_once("Попытка\n    ЗначениеТелаПробы = ОбменДанными.")
        .unwrap();
    // Эта проверка порядка выбора элемента не зависит от ещё реализуемого
    // разрешения голых свойств: получатель указан явно в тех же операциях.
    let body = body
        .replace("= Код;", "= ЭтотОбъект.Код;")
        .replace("    Код =", "    ЭтотОбъект.Код =")
        .replace("= Наименование;", "= ЭтотОбъект.Наименование;")
        .replace("    Наименование =", "    ЭтотОбъект.Наименование =")
        .replace("= Ссылка;", "= ЭтотОбъект.Ссылка;");
    let source = format!(
        "Перем ИтогиТелаПробы;\nФункция ПрочитатьРезультатыТела() Экспорт Возврат ИтогиТелаПробы; КонецФункции\nИтогиТелаПробы = Новый Массив;\n{body}"
    );
    let factory = factory_with_object_source(false, Some(&source));
    let mut session = session(&factory);
    let actual = replay(
        &mut session,
        r#"
        Объект = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
        Для Каждого Итог Из Объект.ПрочитатьРезультатыТела() Цикл
            Сообщить("catalog.property.body." + Итог);
        КонецЦикла;
    "#,
    );
    let expected = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-object-properties.platform.txt"
    )
    .lines()
    .take(6)
    .collect::<Vec<_>>()
    .join("\n");
    assert_eq!(actual, expected);
}

#[test]
fn bare_properties_and_exchange_settings_match_native_russian_oracle() {
    let factory = factory_with_object_source(
        false,
        Some(include_str!(
            "../../../../tests/conformance/measure/1c/catalog-object-properties-module.bsl"
        )),
    );
    let mut session = session(&factory);
    let actual = replay(
        &mut session,
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-object-properties.bsl"
        ),
    );
    assert_eq!(actual, include_str!("../../../../tests/conformance/measure/1c/measure-catalog-object-properties.ru.platform.txt").trim());
}

#[test]
fn dynamic_properties_use_the_same_receiver_and_settings_are_instance_local() {
    let source = r#"
        Функция ИзменитьДинамически() Экспорт
            Выполнить("кОд = ""dynamic""; ДополнительныеСвойства.Вставить(""probe"", 17);");
            Возврат Вычислить("Код");
        КонецФункции
    "#;
    let factory = factory_with_object_source(false, Some(source));
    let mut session = session(&factory);
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            Первый = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Второй = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Первый.ОбменДанными.Загрузка = Истина;
            Первый.ОбменДанными.Получатели.АвтоЗаполнение = Ложь;
            Возврат Первый.ИзменитьДинамически() = "dynamic" И Первый.Код = "dynamic"
                И Второй.Код = "" И Первый.ДополнительныеСвойства.probe = 17
                И Второй.ДополнительныеСвойства.Количество() = 0
                И Не Второй.ОбменДанными.Загрузка И Второй.ОбменДанными.Получатели.АвтоЗаполнение;
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
    assert!(
        session
            .instantiate("Функция Проверить() Экспорт Возврат Код; КонецФункции")
            .is_err()
    );
}

#[test]
fn explicit_variables_keep_priority_over_declared_receiver_properties() {
    let source = r#"
        Перем Код;
        Функция МодульныйКод() Экспорт Возврат Код; КонецФункции
        Функция ЛокальныйКод() Экспорт Перем Код; Код = "local"; Возврат Код; КонецФункции
        Код = "module";
    "#;
    let factory = factory_with_object_source(false, Some(source));
    let mut session = session(&factory);
    let module = session.instantiate(r#"
        Функция Проверить() Экспорт
            Объект = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Возврат Объект.МодульныйКод() = "module" И Объект.ЛокальныйКод() = "local" И Объект.Код = "";
        КонецФункции
    "#).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
