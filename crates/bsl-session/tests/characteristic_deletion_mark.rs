use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{CharacteristicSettings, MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, Value, format_value};
use std::sync::Arc;

#[test]
fn characteristic_deletion_mark_matches_both_native_phases() {
    let object = MetadataObject::new(
        "probe",
        "ЗамерХарактеристик",
        MetadataKind::ChartOfCharacteristicTypes,
    )
    .with_object_module("ПланВидовХарактеристик.ЗамерХарактеристик.МодульОбъекта")
    .with_characteristic_attributes(vec![])
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
        "characteristic-deletion-mark",
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
                include_str!("../../../docs/research/fixtures/characteristic-deletion-mark/ObjectModule.bsl"),
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            ).with_owner("probe"),
        ],
    ).unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("characteristic-deletion-mark", vec![object]).unwrap()),
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
    // Меняем только приёмник вывода; все проверки и обработчики ошибок сохраняются.
    let source =
        include_str!("../../../docs/research/fixtures/characteristic-deletion-mark/measure.bsl")
            .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции"
    )).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("ожидался массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("ожидался массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        actual,
        include_str!(
            "../../../docs/research/fixtures/characteristic-deletion-mark/measure.platform.txt"
        )
        .trim()
    );
}
