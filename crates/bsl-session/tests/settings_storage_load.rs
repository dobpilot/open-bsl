use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

#[test]
fn custom_settings_load_matches_native_handler_inputs() {
    let object = MetadataObject::new("probe", "ЗамерХранилища", MetadataKind::SettingsStorage)
        .with_manager_module("ХранилищеНастроек.ЗамерХранилища.МодульМенеджера");
    let code = ConfigurationCodeSnapshot::new(
        "settings-storage-load",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СвязатьПробу() Экспорт Возврат ХранилищаНастроек.ЗамерХранилища.ЗагрузитьВКонтексте(\"probe\"); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                "ХранилищеНастроек.ЗамерХранилища.МодульМенеджера",
                include_str!(
                    "../../../docs/research/fixtures/settings-storage-load/manager-module.bsl"
                ),
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("probe"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("settings-storage-load", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    // Меняем только приёмник вывода; все проверки и обработчики ошибок сохраняются.
    let source = include_str!("../../../docs/research/fixtures/settings-storage-load/measure.bsl")
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
        include_str!("../../../docs/research/fixtures/settings-storage-load/measure.platform.txt")
            .trim()
    );
}
