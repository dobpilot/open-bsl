use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    BusinessProcessNumberSettings, CatalogStandardAttributeFillChecking, DocumentAttribute,
    DocumentNumberType, DocumentTabularSection, FillChecking, MetadataCatalog, MetadataKind,
    MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{ColumnType, TypeId, TypeRef, Value};

#[test]
fn creation_reports_unknown_metadata_and_preserves_an_existing_manager_export() {
    for exported in [false, true] {
        let manager = "БизнесПроцесс.Проба.МодульМенеджера";
        let manager_source = if exported {
            "Функция СоздатьБизнесПроцесс() Экспорт Возврат 17; КонецФункции"
        } else {
            ""
        };
        let code = ConfigurationCodeSnapshot::new("business-unknown", vec![
            ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Создать() Экспорт Возврат БизнесПроцессы.Проба.СоздатьБизнесПроцесс(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
            ConfigurationModule::new("manager", manager, manager_source, ModuleKind::Manager, ModuleExecution::new().with_server(true)).with_owner("process"),
        ]).unwrap();
        let catalog = MetadataCatalog::new(
            "business-unknown",
            vec![
                MetadataObject::new("process", "Проба", MetadataKind::BusinessProcess)
                    .with_manager_module(manager),
            ],
        )
        .unwrap();
        let factory = SessionFactory::new(
            code,
            Arc::new(catalog),
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
        let module = session.instantiate("Функция Проверить() Экспорт Возврат БизнесПроцессы.Проба.СоздатьБизнесПроцесс(); КонецФункции").unwrap();
        let result = session.call(&module, "Проверить", vec![]);
        if exported {
            assert_eq!(result.unwrap().0, Value::number_from_i64(17));
        } else {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("настройки номера бизнес-процесса неизвестны")
            );
        }
    }
}

#[test]
fn new_objects_replay_the_native_alias_and_module_controls() {
    let controls = [
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-replace.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-replace.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-replace-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-refusal.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-refusal.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-refusal-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-handler-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-arity.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-arity.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-module-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-alias.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-alias.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-module-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-required.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-required.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-handler-standard.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-handler-standard.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-handler-standard-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-handler.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-handler.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-handler-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-table.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-table.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-empty.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-fill-empty.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-module.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-module.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-module-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-types.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/tabular-section-types.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-isolation.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-isolation.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-module-object.bsl"
            ),
        ),
        (
            include_str!("../../../docs/research/fixtures/business-process-object/own-fields.bsl"),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-module.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-module.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/own-fields-module-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-direct.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-direct.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-direct-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-no-handler.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-no-handler.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-standard-no-handler-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-handler-inspect.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-handler-inspect.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-handler-inspect-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-handler-empty.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-handler-empty.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-handler-inspect-object.bsl"
            ),
        ),
        (
            include_str!("../../../docs/research/fixtures/business-process-object/fill-empty.bsl"),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-empty.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/new-object-alias.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/new-object-alias.platform.txt"
            ),
            "",
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/module-state.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/module-state.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/module-state-object.bsl"
            ),
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/standard-properties.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/standard-properties.platform.txt"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/standard-properties-object.bsl"
            ),
        ),
    ];
    for (source, expected, object_source) in controls {
        let object_module = "БизнесПроцесс.ЗамерБизнесПроцесса.МодульОбъекта";
        let code = ConfigurationCodeSnapshot::new(
            "business-object",
            vec![
                ConfigurationModule::new(
                    "session",
                    "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция ОбъектДляСвязывания() Экспорт Возврат БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс(); КонецФункции",
                    ModuleKind::Session,
                    ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "object",
                    object_module,
                    object_source,
                    ModuleKind::Object,
                    ModuleExecution::new().with_server(true),
                )
                .with_owner("process"),
            ],
        )
        .unwrap();
        let catalog = MetadataCatalog::new(
            "business-object",
            vec![
                MetadataObject::new(
                    "process",
                    "ЗамерБизнесПроцесса",
                    MetadataKind::BusinessProcess,
                )
                .with_object_module(object_module),
                MetadataObject::new("language-ru", "Русский", MetadataKind::Language)
                    .with_string_property("КодЯзыка", "ru"),
            ],
        )
        .unwrap()
        .with_business_process_metadata(
            "process",
            Some(BusinessProcessNumberSettings {
                number_type: DocumentNumberType::String,
                length: 11,
                variable_length: true,
            }),
            Some(
                if source.contains("business.fields.") || source.contains("business.check.") {
                    vec![
                        DocumentAttribute::new(
                            "text",
                            "ТекстПробы",
                            vec![ColumnType {
                                id: TypeRef::Native(TypeId::String),
                                quals: vec!["12".into()],
                            }],
                        )
                        .with_fill_from_filling_value(false)
                        .with_fill_checking(
                            if source.contains("business.check.required.")
                                || source.contains("business.check.message.")
                                || source.contains("business.check.handler.")
                                || source.contains("business.check.handler-refusal.")
                                || source.contains("business.check.handler-replace.")
                            {
                                FillChecking::ShowError
                            } else {
                                FillChecking::DontCheck
                            },
                        )
                        .with_synonym("ru", "Текст пробы"),
                        DocumentAttribute::new(
                            "flag",
                            "ФлагПробы",
                            vec![ColumnType::plain(TypeRef::Native(TypeId::Boolean))],
                        )
                        .with_fill_from_filling_value(false)
                        .with_fill_checking(FillChecking::DontCheck),
                    ]
                } else {
                    vec![]
                },
            ),
            Some(if source.contains("business.section.") {
                vec![
                    DocumentTabularSection::new("section", "СтрокиПробы").with_attributes(vec![
                        DocumentAttribute::new(
                            "row-text",
                            "ТекстСтроки",
                            vec![ColumnType {
                                id: TypeRef::Native(TypeId::String),
                                quals: vec!["12".into()],
                            }],
                        ),
                        DocumentAttribute::new(
                            "row-flag",
                            "ФлагСтроки",
                            vec![ColumnType::plain(TypeRef::Native(TypeId::Boolean))],
                        ),
                    ]),
                ]
            } else {
                vec![]
            }),
        )
        .unwrap();
        let catalog = catalog
            .with_business_process_standard_fill_checking(
                "process",
                [
                    "Started",
                    "HeadTask",
                    "Completed",
                    "Ref",
                    "DeletionMark",
                    "Date",
                    "Number",
                ]
                .into_iter()
                .map(|name| CatalogStandardAttributeFillChecking {
                    name: name.into(),
                    fill_checking: FillChecking::DontCheck,
                })
                .collect(),
            )
            .unwrap()
            .with_configuration(
                "Проба",
                "1.0",
                bsl_orm::ScriptVariant::Russian,
                bsl_orm::CompatibilityMode::DontUse,
                Vec::<String>::new(),
            )
            .unwrap()
            .with_primary_language("language-ru")
            .unwrap();
        let factory = SessionFactory::new(
            code,
            Arc::new(catalog),
            "МодульСеанса",
            SessionKind::ThinClient,
            "8.3.27.2342",
        )
        .unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_system_language("en"),
            )
            .unwrap();
        // Меняется только приёмник вывода; текст самих нативных проб сохраняется.
        let source = format!(
            "Перем ВыводБизнесПробы;\nПроцедура ЗаписатьКонтроль(Текст) ВыводБизнесПробы = ВыводБизнесПробы + Текст + Символ(10); КонецПроцедуры\nФункция Проверить() Экспорт ВыводБизнесПробы = \"\";\n{}\nВозврат ВыводБизнесПробы; КонецФункции",
            source.replace("Сообщить(", "ЗаписатьКонтроль(")
        );
        let module = session.instantiate(&source).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Str(expected.into())
        );
    }
}

#[test]
fn checking_does_not_invent_success_for_unknown_standard_or_own_policy() {
    for standard_known in [false, true] {
        let code = ConfigurationCodeSnapshot::new(
            "unknown-check",
            vec![ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            )],
        )
        .unwrap();
        let mut catalog = MetadataCatalog::new(
            "unknown-check",
            vec![MetadataObject::new(
                "process",
                "Проба",
                MetadataKind::BusinessProcess,
            )],
        )
        .unwrap()
        .with_business_process_metadata(
            "process",
            Some(BusinessProcessNumberSettings {
                number_type: DocumentNumberType::String,
                length: 11,
                variable_length: true,
            }),
            Some(vec![DocumentAttribute::new(
                "text",
                "ТекстПробы",
                vec![ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                }],
            )]),
            Some(vec![]),
        )
        .unwrap();
        if standard_known {
            catalog = catalog
                .with_business_process_standard_fill_checking("process", vec![])
                .unwrap();
        }
        let factory = SessionFactory::new(
            code,
            Arc::new(catalog),
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
        let module = session.instantiate("Функция Проверить() Экспорт Возврат БизнесПроцессы.Проба.СоздатьБизнесПроцесс().ПроверитьЗаполнение(); КонецФункции").unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(if standard_known {
                "настройка проверки собственного реквизита бизнес-процесса неизвестна"
            } else {
                "настройки проверки стандартных реквизитов бизнес-процесса неизвестны"
            }),
            "{error}"
        );
    }
}
