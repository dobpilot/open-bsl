use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

#[test]
fn characteristic_types_are_registered_in_the_session_through_the_facade() {
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-types",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "characteristic-types",
            vec![MetadataObject::new(
                "probe",
                "ЗамерХарактеристик",
                MetadataKind::ChartOfCharacteristicTypes,
            )],
        )
        .unwrap(),
    );
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"Функция Проверить() Экспорт
Объект = Тип("ПланВидовХарактеристикОбъект.ЗамерХарактеристик");
Ссылка = Тип("ПланВидовХарактеристикСсылка.ЗамерХарактеристик");
Возврат Объект = Тип("ChartOfCharacteristicTypesObject.ЗамерХарактеристик")
    И Ссылка = Тип("ChartOfCharacteristicTypesRef.ЗамерХарактеристик")
    И Объект <> Ссылка;
КонецФункции"#,
        )
        .unwrap();
    assert!(matches!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    ));
    let unknown_settings = session
        .instantiate("Функция Создать() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент(); КонецФункции")
        .unwrap();
    let error = session
        .call(&unknown_settings, "Создать", vec![])
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("настройки плана видов характеристик неизвестны")
    );
}

#[test]
fn characteristic_creation_matches_the_native_receipt() {
    use bsl_orm::CharacteristicSettings;
    use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, format_value};
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: [
            TypeId::Boolean,
            TypeId::String,
            TypeId::Date,
            TypeId::Number,
        ]
        .into_iter()
        .map(|id| ColumnType::plain(TypeRef::Native(id)))
        .collect(),
    });
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-create",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-create", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!("../../../docs/research/fixtures/characteristic-manager-create/measure-characteristic-create.bsl")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    // Ошибка пробы должна выйти в Rust, а не скрыться за сообщением обработчика.
    let source = source
        .split("Исключение")
        .next()
        .unwrap()
        .replace("Попытка", "");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual, include_str!("../../../docs/research/fixtures/characteristic-manager-create/measure-characteristic-create.platform.txt").trim());
}

#[test]
fn characteristic_body_matches_the_native_receipt() {
    use bsl_orm::CharacteristicSettings;
    use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, format_value};
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_object_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта")
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: [
            TypeId::Boolean,
            TypeId::String,
            TypeId::Date,
            TypeId::Number,
        ]
        .into_iter()
        .map(|id| ColumnType::plain(TypeRef::Native(id)))
        .collect(),
    });
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-body",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                "ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта",
                include_str!(
                    "../../../docs/research/fixtures/characteristic-manager-body/ObjectModule.bsl"
                ),
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-body", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!("../../../docs/research/fixtures/characteristic-manager-body/measure-characteristic-body.bsl")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let source = source
        .split("Исключение")
        .next()
        .unwrap()
        .replace("Попытка", "");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual, include_str!("../../../docs/research/fixtures/characteristic-manager-body/measure-characteristic-body.platform.txt").trim());
}

#[test]
fn characteristic_manager_context_and_instances_are_isolated() {
    use bsl_orm::CharacteristicSettings;
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_manager_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульМенеджера")
    .with_object_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта")
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: vec![],
    });
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-isolation",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЧерезКонтекст(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager",
                "ПланВидовХарактеристик.ЗамерХарактеристик.МодульМенеджера",
                include_str!("../../../docs/research/fixtures/characteristic-manager-qualification/ManagerModule.bsl"),
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            ).with_owner("probe"),
            ConfigurationModule::new(
                "object",
                "ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта",
                "Перем Счетчик; Функция Следующий() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции Счетчик = 0;",
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            ).with_owner("probe"),
        ],
    ).unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-isolation", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    for _ in 0..2 {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session
            .instantiate(
                r#"Функция Проверить() Экспорт
Первый = ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЧерезКонтекст();
Второй = ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЧерезКонтекст();
Первый.Наименование = "first";
Первый.ОбменДанными.Загрузка = Истина;
Возврат Первый.Следующий() = 1 И Первый.Следующий() = 2
    И Второй.Следующий() = 1 И Второй.Наименование = ""
    И Первый.ОбменДанными.Загрузка И НЕ Второй.ОбменДанными.Загрузка
    И Второй.ОбменДанными.Получатели.АвтоЗаполнение
    И ТипЗнч(Первый) = Тип("ПланВидовХарактеристикОбъект.ЗамерХарактеристик");
КонецФункции"#,
            )
            .unwrap();
        assert!(matches!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Boolean(true)
        ));
    }
}

#[test]
fn characteristic_own_attribute_matches_the_native_receipt() {
    use bsl_orm::CharacteristicSettings;
    use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, format_value};
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_object_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта")
    .with_characteristic_attributes(vec![bsl_orm::DocumentAttribute::new(
        "attribute",
        "УникальныйКодДляПоля",
        vec![ColumnType {
            id: TypeRef::Native(TypeId::String),
            quals: vec!["12".into()],
        }],
    )])
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: [
            TypeId::Boolean,
            TypeId::String,
            TypeId::Date,
            TypeId::Number,
        ]
        .into_iter()
        .map(|id| ColumnType::plain(TypeRef::Native(id)))
        .collect(),
    });
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-body",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                "ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта",
                include_str!(
                    "../../../docs/research/fixtures/characteristic-manager-attributes/ObjectModule.bsl"
                ),
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-body", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!("../../../docs/research/fixtures/characteristic-manager-attributes/measure-characteristic-attributes.bsl")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let source = source
        .split("Исключение")
        .next()
        .unwrap()
        .replace("Попытка", "");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual, include_str!("../../../docs/research/fixtures/characteristic-manager-attributes/measure-characteristic-attributes.platform.txt").trim());
}

#[test]
fn characteristic_qualification_matches_the_native_receipt() {
    use bsl_orm::CharacteristicSettings;
    use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, format_value};
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_manager_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульМенеджера")
    .with_object_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта")
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: [
            TypeId::Boolean,
            TypeId::String,
            TypeId::Date,
            TypeId::Number,
        ]
        .into_iter()
        .map(|id| ColumnType::plain(TypeRef::Native(id)))
        .collect(),
    });
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-body",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "manager", "ПланВидовХарактеристик.ЗамерХарактеристик.МодульМенеджера",
                include_str!("../../../docs/research/fixtures/characteristic-manager-qualification/ManagerModule.bsl"),
                ModuleKind::Manager, ModuleExecution::new().with_server(true),
            ).with_owner("probe"),
            ConfigurationModule::new(
                "object",
                "ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта",
                include_str!(
                    "../../../docs/research/fixtures/characteristic-manager-body/ObjectModule.bsl"
                ),
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-body", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!("../../../docs/research/fixtures/characteristic-manager-qualification/measure-characteristic-qualification.bsl")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual, include_str!("../../../docs/research/fixtures/characteristic-manager-qualification/measure-characteristic-qualification.platform.txt").trim());
}

#[test]
fn characteristic_value_type_qualifiers_survive_facade_calls_assignment_and_reset() {
    use bsl_orm::CharacteristicSettings;
    use open_bsl::{BslObject, ColumnType, TypeId, TypeRef};
    let code = ConfigurationCodeSnapshot::new(
        "qualified-characteristic",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: vec![
            ColumnType::plain(TypeRef::Native(TypeId::Boolean)),
            ColumnType {
                id: TypeRef::Native(TypeId::String),
                quals: vec!["100".into(), "1".into()],
            },
            ColumnType::plain(TypeRef::Native(TypeId::Date)),
            ColumnType {
                id: TypeRef::Native(TypeId::Number),
                quals: vec!["15".into(), "2".into(), "0".into()],
            },
        ],
    });
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("qualified-characteristic", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
Функция Проверить(ДлиннаяСтрока) Экспорт
    Первый = ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент();
    Второй = ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент();
    Второй.ТипЗначения = Первый.ТипЗначения;
    Ответ = Новый Массив;
    Ответ.Добавить(СтрДлина(Второй.ТипЗначения.ПривестиЗначение(ДлиннаяСтрока)));
    Второй.ТипЗначения = Новый ОписаниеТипов("Строка");
    Второй.ТипЗначения = Неопределено;
    Ответ.Добавить(Второй.ТипЗначения.ПривестиЗначение(1.234));
    Копия = ЗначениеИзСтрокиВнутр(ЗначениеВСтрокуВнутр(Первый.ТипЗначения));
    Ответ.Добавить(Копия.ПривестиЗначение(-1.235));
    Ответ.Добавить(ТипЗнч(Второй.ТипЗначения) = Тип("ОписаниеТипов"));
    Ответ.Добавить(Второй.ТипЗначения.Типы().Количество());
    Возврат Ответ;
КонецФункции
"#,
        )
        .unwrap();
    let (Value::Object(result), _) = session
        .call(
            &module,
            "Проверить",
            vec![Value::Str(open_bsl::BslString::from_str(&"a".repeat(120)))],
        )
        .unwrap()
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(items) = result.as_ref() else {
        panic!("массив результатов")
    };
    assert_eq!(
        *items.borrow(),
        vec![
            Value::Number(open_bsl::BslNumber::from_i64(100)),
            Value::Number(open_bsl::BslNumber::parse_canonical("1.23").unwrap()),
            Value::Number(open_bsl::BslNumber::parse_canonical("-1.24").unwrap()),
            Value::Boolean(true),
            Value::Number(open_bsl::BslNumber::from_i64(4)),
        ]
    );
}

#[test]
fn characteristic_additional_properties_match_native_initialization_and_instance_isolation() {
    use bsl_orm::CharacteristicSettings;
    use open_bsl::{BslObject, format_value};
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_object_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта")
    .with_characteristic_settings(CharacteristicSettings {
        code_length: 9,
        description_length: 25,
        value_types: vec![],
    });
    let code = ConfigurationCodeSnapshot::new(
        "characteristic-additional",
        vec![
            ConfigurationModule::new(
                "session", "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЭлемент(); КонецФункции",
                ModuleKind::Session, ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object", "ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта",
                include_str!("../../../docs/research/fixtures/characteristic-additional-properties/ObjectModule.bsl"),
                ModuleKind::Object, ModuleExecution::new().with_server(true),
            ).with_owner("probe"),
        ],
    ).unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-additional", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    // Квитанция формы содержит сообщения драйвера; сообщения тела модуля
    // остаются в обычном выводе и не подменяют проверяемые результаты.
    let source = include_str!(
        "../../../docs/research/fixtures/characteristic-additional-properties/measure.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    // Внешний обработчик измерительного драйвера не должен скрывать причину
    // отказа. Внутренняя проверка запрещённого присваивания сохраняется.
    let source = source.strip_prefix("Попытка\n").unwrap();
    let source = source.rsplit_once("Исключение\n").unwrap().0;
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual, include_str!("../../../docs/research/fixtures/characteristic-additional-properties/measure.platform.txt").trim());
}
