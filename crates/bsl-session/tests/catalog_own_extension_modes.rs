use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogStandardAttributes, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

fn factory_with_source(safe: bool, source: &str) -> SessionFactory {
    let module = "Справочник.CP_ЗамерКода.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new(
        "own-extension-modes",
        vec![
            ConfigurationModule::new(
                "session",
                "SessionModule",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Справочники.CP_ЗамерКода.СоздатьЭлемент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                module,
                source,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("catalog")
            .with_own_extension_source(safe),
        ],
    )
    .unwrap();
    let object = MetadataObject::new("catalog", "CP_ЗамерКода", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_own_extension_object_module(module, safe);
    SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("own-extension-modes", vec![object]).unwrap()),
        "SessionModule",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn replay_native(source: &str, script: &str, expected: [&str; 2]) {
    let split = script.find("Попытка\n    Для НомерПробы").unwrap();
    let script = format!(
        "{}\nФункция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{}\nВозврат РезультатыПробы; КонецФункции",
        &script[..split],
        script[split..].replace("Сообщить(", "РезультатыПробы.Добавить(")
    );
    for (safe, expected) in [false, true].into_iter().zip(expected) {
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
            let text = result
                .borrow()
                .iter()
                .map(|v| format_value(v, None).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(text, expected, "safe={safe}, principal={user}");
        }
    }
}

#[test]
fn own_extension_event_modes_match_the_native_getters_for_each_session() {
    replay_native(
        include_str!(
            "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/own-getters-input-xml/Catalogs/CP_ЗамерКода/Ext/ObjectModule.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/measure-own-getters.bsl"
        ),
        [
            include_str!(
                "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/own-getters-safe-no.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/own-getters-safe-yes.platform.txt"
            ),
        ],
    );
}

#[test]
fn own_entry_errors_match_native_caller_modes_and_object_memory() {
    replay_native(
        include_str!(
            "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/own-error-input-xml/Catalogs/CP_ЗамерКода/Ext/ObjectModule.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/measure-own-error.bsl"
        ),
        [
            include_str!(
                "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/own-error-safe-no.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/catalog-new-code-borrowed-1017/own-error-safe-yes.platform.txt"
            ),
        ],
    );
}

#[path = "catalog_own_extension_modes/mixed_native.rs"]
mod mixed_native;
