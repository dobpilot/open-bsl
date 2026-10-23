use super::*;
use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};

const TASK: &str = "2adee501-a0c9-4849-9fd0-a3d10679d4f7";

const BP: &str = "6db4bbe0-ce4c-4389-ad25-9a10c1857192";

fn projected(id: &str, kind: MetadataKind, bytes: &[u8], unknown: bool) -> MetadataCatalog {
    projected_with_module(id, kind, bytes, unknown, false)
}

fn projected_with_module(
    id: &str,
    kind: MetadataKind,
    bytes: &[u8],
    unknown: bool,
    module: bool,
) -> MetadataCatalog {
    projected_with_task(id, kind, bytes, unknown, module, false)
}

fn projected_with_task(
    id: &str,
    kind: MetadataKind,
    bytes: &[u8],
    unknown: bool,
    module: bool,
    task: bool,
) -> MetadataCatalog {
    let mut descriptors = parse_config_descriptors(id, bytes).unwrap();
    if unknown {
        let owner = descriptors
            .iter_mut()
            .find(|d| d.object_guid.as_str() == id)
            .unwrap();
        owner.business_process_number = None;
        owner.business_process_attributes = None;
        owner.business_process_tabular_sections = None;
        owner.business_process_standard_fill_checking = None;
        owner.accumulation_register_kind = None;
        owner.accumulation_register_dimensions = None;
        owner.accumulation_register_resources = None;
    }
    let metadata = resolve_metadata(
        DbNames::from_entries(if task {
            vec![
                open_sdbl::metadata::DbNameEntry {
                    guid: id.parse().unwrap(),
                    alias: "BPr".into(),
                    number: 1,
                },
                open_sdbl::metadata::DbNameEntry {
                    guid: TASK.parse().unwrap(),
                    alias: "Task".into(),
                    number: 2,
                },
            ]
        } else {
            vec![]
        }),
        descriptors,
        if task {
            head_task_schema(vec![open_sdbl::metadata::SchemaColumn {
                name: "HeadTask".into(),
                types: vec![open_sdbl::metadata::ColumnType {
                    tag: "R".into(),
                    reference_target: Some("Task2".into()),
                }],
            }])
        } else {
            SchemaStorage {
                tables: vec![],
                anomalies: vec![],
            }
        },
        vec![],
    )
    .snapshot;
    let mut objects = vec![
        {
            let object = bsl_orm::MetadataObject::new(
                id,
                metadata
                    .descriptors()
                    .iter()
                    .find(|descriptor| descriptor.object_guid.as_str() == id)
                    .unwrap()
                    .name
                    .as_str(),
                kind,
            );
            if module {
                object.with_object_module("БизнесПроцесс.ЗамерБизнесПроцесса.МодульОбъекта")
            } else {
                object
            }
        },
        bsl_orm::MetadataObject::new("language-ru", "Русский", MetadataKind::Language)
            .with_string_property("КодЯзыка", "ru"),
    ];
    if task {
        objects.push(bsl_orm::MetadataObject::new(
            TASK,
            "ЗамерЗадачи",
            MetadataKind::Task,
        ));
    }
    let catalog = MetadataCatalog::new("native-099", objects)
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
    project(catalog, &metadata).unwrap()
}

#[test]
fn native_business_process_projection_preserves_members_defaults_and_number() {
    let bytes = include_bytes!(
        "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
    );
    let catalog = projected(BP, MetadataKind::BusinessProcess, bytes, false);
    let owner = &catalog.objects()[0];
    let number = owner.business_process_number_settings().unwrap();
    assert_eq!(number.number_type, bsl_orm::DocumentNumberType::String);
    assert_eq!(number.length, 11);
    assert!(number.variable_length);
    let attributes = owner.business_process_attributes().unwrap();
    assert_eq!(
        attributes
            .iter()
            .map(DocumentAttribute::name)
            .collect::<Vec<_>>(),
        ["ТекстПробы", "ФлагПробы"]
    );
    assert_eq!(attributes[0].types()[0].quals, ["12", "1"]);
    assert_eq!(attributes[0].types()[0].id, TypeRef::Native(TypeId::String));
    assert_eq!(
        attributes[1].types()[0].id,
        TypeRef::Native(TypeId::Boolean)
    );
    for attribute in attributes {
        assert_eq!(attribute.fill_from_filling_value(), Some(true));
        let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
        assert_eq!(
            attribute
                .fill_value()
                .unwrap()
                .materialize(&mut shapes)
                .unwrap(),
            [open_bsl::Value::Undefined]
        );
        assert!(attribute.fill_checking().is_some());
    }
    assert!(
        owner
            .business_process_tabular_sections()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        owner
            .business_process_standard_fill_checking()
            .unwrap()
            .len(),
        7
    );
    let unknown = projected(BP, MetadataKind::BusinessProcess, bytes, true);
    let owner = &unknown.objects()[0];
    assert!(owner.business_process_number_settings().is_none());
    assert!(owner.business_process_attributes().is_none());
    assert!(owner.business_process_tabular_sections().is_none());
    assert!(owner.business_process_standard_fill_checking().is_none());
    let numeric = projected(
        BP,
        MetadataKind::BusinessProcess,
        include_bytes!("../tests/fixtures/runtime-object-099/number.deflate"),
        false,
    );
    let settings = numeric.objects()[0]
        .business_process_number_settings()
        .unwrap();
    assert_eq!(settings.number_type, bsl_orm::DocumentNumberType::Number);
    assert_eq!(settings.length, 8);
    assert!(!settings.variable_length);
}

#[test]
fn native_business_process_section_members_and_empty_section_survive_projection() {
    let catalog = projected(
        BP,
        MetadataKind::BusinessProcess,
        include_bytes!("../tests/fixtures/runtime-object-099/membership.deflate"),
        false,
    );
    let sections = catalog.objects()[0]
        .business_process_tabular_sections()
        .unwrap();
    let expected = include_str!("../tests/fixtures/runtime-object-099/sections.tsv");
    assert_eq!(sections.len(), expected.lines().count());
    for (section, row) in sections.iter().zip(expected.lines()) {
        let row = row.split('\t').collect::<Vec<_>>();
        assert_eq!((section.id(), section.name()), (row[0], row[1]));
        let ids = row[2]
            .split(',')
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(
            section
                .attributes()
                .unwrap()
                .iter()
                .map(DocumentAttribute::id)
                .collect::<Vec<_>>(),
            ids
        );
    }
}

#[test]
fn native_register_kinds_order_types_and_known_empty_collections_survive_projection() {
    let cases: &[(&str, &[u8], bsl_orm::AccumulationRegisterKind)] = &[
        (
            "c72d739a-05e3-4e9e-bcc7-3bf8baf5dc63",
            include_bytes!("../tests/fixtures/runtime-object-099/ОстаткиПорядок.deflate"),
            bsl_orm::AccumulationRegisterKind::Balance,
        ),
        (
            "8453c268-062b-408a-a0cf-bf8d8b26e839",
            include_bytes!("../tests/fixtures/runtime-object-099/ОборотыПорядок.deflate"),
            bsl_orm::AccumulationRegisterKind::Turnovers,
        ),
        (
            "301e85ee-efb1-4887-9a6b-701362a52212",
            include_bytes!("../tests/fixtures/runtime-object-099/ОборотыПусто.deflate"),
            bsl_orm::AccumulationRegisterKind::Turnovers,
        ),
    ];
    for &(id, bytes, kind) in cases {
        let catalog = projected(id, MetadataKind::AccumulationRegister, bytes, false);
        let owner = &catalog.objects()[0];
        assert_eq!(owner.accumulation_register_kind(), Some(kind));
        let row = include_str!("../tests/fixtures/runtime-object-099/registers.tsv")
            .lines()
            .find(|row| row.starts_with(id))
            .unwrap()
            .split('\t')
            .collect::<Vec<_>>();
        for (attributes, expected) in [
            (owner.accumulation_register_dimensions().unwrap(), row[3]),
            (owner.accumulation_register_resources().unwrap(), row[4]),
        ] {
            assert_eq!(
                attributes
                    .iter()
                    .map(DocumentAttribute::id)
                    .collect::<Vec<_>>(),
                expected
                    .split(',')
                    .filter(|id| !id.is_empty())
                    .collect::<Vec<_>>()
            );
            assert!(
                attributes
                    .iter()
                    .all(|attribute| !attribute.types().is_empty())
            );
        }
        for attribute in owner.accumulation_register_dimensions().unwrap() {
            assert_eq!(attribute.types()[0].id, TypeRef::Native(TypeId::String));
            assert_eq!(attribute.types()[0].quals, ["12", "1"]);
        }
        for attribute in owner.accumulation_register_resources().unwrap() {
            assert_eq!(attribute.types()[0].id, TypeRef::Native(TypeId::Number));
            assert_eq!(attribute.types()[0].quals, ["15", "2", "0"]);
        }
        let unknown = projected(id, MetadataKind::AccumulationRegister, bytes, true);
        assert!(unknown.objects()[0].accumulation_register_kind().is_none());
        assert!(
            unknown.objects()[0]
                .accumulation_register_dimensions()
                .is_none()
        );
        assert!(
            unknown.objects()[0]
                .accumulation_register_resources()
                .is_none()
        );
    }
}

#[test]
fn native_business_process_controls_replay_with_sdk_acquired_attributes() {
    let controls: &[(&[u8], &str, &str)] = &[
        (
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.platform.txt"
            ),
        ),
        (
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/check-filling-required.native.deflate"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-required.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/check-filling-required.platform.txt"
            ),
        ),
    ];
    for &(bytes, source, expected) in controls {
        let catalog = projected(BP, MetadataKind::BusinessProcess, bytes, false);
        replay_native_control(catalog, source, expected);
    }
}

fn replay_native_control(catalog: MetadataCatalog, source: &str, expected: &str) {
    replay_native_control_with_module(catalog, source, expected, None);
}

fn replay_native_control_with_module(
    catalog: MetadataCatalog,
    source: &str,
    expected: &str,
    object_source: Option<&str>,
) {
    use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
    use bsl_session::{SessionFactory, SessionKind, SessionResources};
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс(); КонецФункции",
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    if let Some(source) = object_source {
        modules.push(
            ConfigurationModule::new(
                "object",
                "БизнесПроцесс.ЗамерБизнесПроцесса.МодульОбъекта",
                source,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner(BP),
        );
    }
    let code = ConfigurationCodeSnapshot::new("native-099", modules).unwrap();
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
            bsl_orm::Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_system_language("ru"),
        )
        .unwrap();
    // Нативное тело сохраняется; заменяется только приёмник вывода.
    let source = format!(
        r#"Перем ВыводПробы; Процедура ЗаписатьКонтроль(Текст) ВыводПробы = ВыводПробы + Текст + Символ(10); КонецПроцедуры Функция Проверить() Экспорт ВыводПробы = ""; {} Возврат ВыводПробы; КонецФункции"#,
        source.replace("Сообщить(", "ЗаписатьКонтроль(")
    );
    let module = session.instantiate(&source).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        open_bsl::Value::Str(expected.into())
    );
}

#[test]
fn native_business_process_read_resets_fields_and_replaces_sections() {
    let controls: &[(&[u8], &str, &str)] = &[
        (
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/read-new-assigned.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/read-new-assigned.platform.txt"
            ),
        ),
        (
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/read-new-standard.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/read-new-standard.platform.txt"
            ),
        ),
        (
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/tabular-section.native.deflate"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/read-new-section.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/read-new-section.platform.txt"
            ),
        ),
    ];
    for &(bytes, source, expected) in controls {
        replay_native_control(
            projected(BP, MetadataKind::BusinessProcess, bytes, false),
            source,
            expected,
        );
    }
}

#[test]
fn native_business_process_read_preserves_private_module_and_extra_properties() {
    replay_native_control_with_module(
        projected_with_module(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            true,
        ),
        include_str!("../../../docs/research/fixtures/business-process-object/read-new-module.bsl"),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/read-new-module.platform.txt"
        ),
        Some(include_str!(
            "../../../docs/research/fixtures/business-process-object/read-module-object.bsl"
        )),
    );
    replay_native_control_with_module(
        projected_with_module(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            true,
        ),
        include_str!("../../../docs/research/fixtures/business-process-object/read-own-module.bsl"),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/read-own-module.platform.txt"
        ),
        Some(include_str!(
            "../../../docs/research/fixtures/business-process-object/read-own-module-object.bsl"
        )),
    );
}

#[test]
fn native_head_task_controls_preserve_type_conversion_and_initialization_rules() {
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            true,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-identity.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-identity.platform.txt"
        ),
        None,
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            true,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-reference-identity.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-reference-identity.platform.txt"
        ),
        None,
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            true,
            true,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-module-identity.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-module-identity.platform.txt"
        ),
        Some(include_str!(
            "../../../docs/research/fixtures/business-process-object/head-task-module-identity-object.bsl"
        )),
    );
}

fn head_task_schema(columns: Vec<open_sdbl::metadata::SchemaColumn>) -> SchemaStorage {
    SchemaStorage {
        tables: vec![open_sdbl::metadata::SchemaTable {
            name: "BPr1".into(),
            number: 1,
            owner: None,
            inline_name: None,
            columns,
            indexes: vec![],
        }],
        anomalies: vec![],
    }
}

#[test]
fn missing_ambiguous_and_non_task_head_task_targets_remain_unknown() {
    use open_sdbl::metadata::{ColumnType, DbNameEntry, SchemaColumn};
    let column = |tag: &str, target: Option<&str>| SchemaColumn {
        name: "HeadTask".into(),
        types: vec![ColumnType {
            tag: tag.into(),
            reference_target: target.map(str::to_owned),
        }],
    };
    for columns in [
        vec![],
        vec![column("R", None)],
        vec![column("S", Some("Task2"))],
        vec![column("R", Some("Reference3"))],
        vec![column("R", Some("Task2")), column("R", Some("Task2"))],
        vec![SchemaColumn {
            name: "HeadTask".into(),
            types: vec![
                ColumnType {
                    tag: "R".into(),
                    reference_target: Some("Task2".into())
                };
                2
            ],
        }],
    ] {
        let snapshot = resolve_metadata(
            DbNames::from_entries(vec![
                DbNameEntry {
                    guid: BP.parse().unwrap(),
                    alias: "BPr".into(),
                    number: 1,
                },
                DbNameEntry {
                    guid: TASK.parse().unwrap(),
                    alias: "Task".into(),
                    number: 2,
                },
                DbNameEntry {
                    guid: "43b8c84d-931b-4e28-a6f0-01ddc0467d7d".parse().unwrap(),
                    alias: "Reference".into(),
                    number: 3,
                },
            ]),
            vec![],
            head_task_schema(columns),
            vec![],
        )
        .snapshot;
        assert_eq!(head_task_target(&snapshot, BP), None);
    }
}

#[test]
fn native_business_process_modified_fields_sections_and_own_context_match() {
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            false,
        ),
        include_str!("../../../docs/research/fixtures/business-process-object/modified.bsl"),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/modified.platform.txt"
        ),
        None,
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/tabular-section.native.deflate"
            ),
            false,
            false,
            false,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/modified-section.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/modified-section.platform.txt"
        ),
        None,
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            true,
            false,
        ),
        include_str!("../../../docs/research/fixtures/business-process-object/modified-module.bsl"),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/modified-module.platform.txt"
        ),
        Some(include_str!(
            "../../../docs/research/fixtures/business-process-object/modified-module-object.bsl"
        )),
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            true,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/modified-failure.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/modified-failure.platform.txt"
        ),
        None,
    );
}

#[test]
fn native_business_process_data_exchange_preserves_aliases_and_phase_guard() {
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            false,
        ),
        include_str!("../../../docs/research/fixtures/business-process-object/data-exchange.bsl"),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/data-exchange.platform.txt"
        ),
        None,
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            true,
            false,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/data-exchange-phase.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/data-exchange-phase.platform.txt"
        ),
        Some(include_str!(
            "../../../docs/research/fixtures/business-process-object/data-exchange-phase-object.bsl"
        )),
    );
}

#[test]
fn native_business_process_pending_reference_and_factories_match() {
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            true,
        ),
        include_str!("../../../docs/research/fixtures/business-process-object/new-reference.bsl"),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/new-reference.platform.txt"
        ),
        None,
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            true,
            true,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/new-reference-module.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/new-reference-module.platform.txt"
        ),
        Some(include_str!(
            "../../../docs/research/fixtures/business-process-object/new-reference-module-object.bsl"
        )),
    );
    replay_native_control_with_module(
        projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            false,
            true,
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/new-reference-invalid.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/business-process-object/new-reference-invalid.platform.txt"
        ),
        None,
    );
}

#[test]
fn acquired_task_type_presentation_replays_original_native_receipts() {
    for (source, expected, module) in [
        (
            include_str!("../../../docs/research/fixtures/business-process-object/head-task.bsl"),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/head-task.platform.txt"
            ),
            None,
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/head-task-reference.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/head-task-reference.platform.txt"
            ),
            None,
        ),
        (
            include_str!(
                "../../../docs/research/fixtures/business-process-object/head-task-module.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/business-process-object/head-task-module.platform.txt"
            ),
            Some(include_str!(
                "../../../docs/research/fixtures/business-process-object/head-task-module-object.bsl"
            )),
        ),
    ] {
        let original = projected_with_task(
            BP,
            MetadataKind::BusinessProcess,
            include_bytes!(
                "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
            ),
            false,
            module.is_some(),
            true,
        );
        let objects = original
            .objects()
            .iter()
            .cloned()
            .map(|object| {
                if object.id() == TASK {
                    object.with_reference_type_presentation("Замер задачи")
                } else {
                    object
                }
            })
            .collect();
        let catalog = MetadataCatalog::new("native-099", objects)
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
        replay_native_control_with_module(catalog, source, expected, module);
    }
}

#[test]
fn sdk_task_synonyms_replay_native_primary_language_controls() {
    for (language, english_synonym, source, expected) in [
        (
            "ru",
            true,
            include_str!("../../../docs/research/fixtures/task-reference-presentation/russian.bsl"),
            include_str!(
                "../../../docs/research/fixtures/task-reference-presentation/russian.platform.txt"
            ),
        ),
        (
            "en",
            true,
            include_str!("../../../docs/research/fixtures/task-reference-presentation/english.bsl"),
            include_str!(
                "../../../docs/research/fixtures/task-reference-presentation/english.platform.txt"
            ),
        ),
        (
            "en",
            false,
            include_str!(
                "../../../docs/research/fixtures/task-reference-presentation/missing-english.bsl"
            ),
            include_str!(
                "../../../docs/research/fixtures/task-reference-presentation/missing-english.platform.txt"
            ),
        ),
        (
            "en",
            true,
            include_str!("../../../docs/research/fixtures/task-reference-presentation/aliases.bsl"),
            include_str!(
                "../../../docs/research/fixtures/task-reference-presentation/aliases.platform.txt"
            ),
        ),
    ] {
        let mut descriptors = parse_config_descriptors(TASK, include_bytes!("../../../docs/research/fixtures/demo-business-task-reference-types/2adee501-a0c9-4849-9fd0-a3d10679d4f7.native.deflate")).unwrap();
        let task = descriptors
            .iter_mut()
            .find(|d| d.object_guid.as_str() == TASK)
            .unwrap();
        // Состав синонимов совпадает с XML-входами независимых нативных контролей.
        task.synonyms = vec![open_sdbl::metadata::Synonym {
            language: "ru".into(),
            text: "Русская задача".into(),
        }];
        if english_synonym {
            task.synonyms.push(open_sdbl::metadata::Synonym {
                language: "en".into(),
                text: "English task".into(),
            });
        }
        let metadata = resolve_metadata(
            DbNames::from_entries(vec![]),
            descriptors,
            SchemaStorage {
                tables: vec![],
                anomalies: vec![],
            },
            vec![],
        )
        .snapshot;
        let objects = vec![
            bsl_orm::MetadataObject::new(TASK, "ЗамерЗадачи", MetadataKind::Task),
            bsl_orm::MetadataObject::new("language", "Язык", MetadataKind::Language)
                .with_string_property("КодЯзыка", language),
        ];
        let objects =
            crate::project_task_reference_presentations(objects, Some(&metadata), language);
        let catalog = MetadataCatalog::new("native-099", objects)
            .unwrap()
            .with_configuration(
                "Проба",
                "1.0",
                bsl_orm::ScriptVariant::Russian,
                bsl_orm::CompatibilityMode::DontUse,
                Vec::<String>::new(),
            )
            .unwrap()
            .with_primary_language("language")
            .unwrap();
        replay_native_control(catalog, source, expected);
    }
}

#[test]
fn unavailable_primary_language_does_not_fabricate_metadata() {
    let original = projected_with_task(
        BP,
        MetadataKind::BusinessProcess,
        include_bytes!(
            "../../../docs/research/fixtures/business-process-object/fill-enabled.native.deflate"
        ),
        false,
        false,
        true,
    );
    let catalog = MetadataCatalog::new("native-099", original.objects().to_vec())
        .unwrap()
        .with_configuration(
            "Проба",
            "1.0",
            bsl_orm::ScriptVariant::Russian,
            bsl_orm::CompatibilityMode::DontUse,
            Vec::<String>::new(),
        )
        .unwrap();
    replay_native_control(
        catalog,
        "ОтказЯзыка = 0; Попытка ЗначениеЯзыка = Метаданные.ОсновнойЯзык; Исключение ОтказЯзыка = 1; КонецПопытки; Сообщить(ОтказЯзыка);",
        "1\n",
    );
}

#[test]
fn native_processor_collections_keep_order_and_verified_empty_sections() {
    let id = "ff92317d-71e7-4664-a13a-18bd31fd3701";
    let catalog = projected(
        id,
        MetadataKind::DataProcessor,
        include_bytes!("../tests/fixtures/data-processor-0911/Замеры.deflate"),
        false,
    );
    let owner = catalog.object_by_id(id).unwrap();
    assert_eq!(
        owner
            .data_processor_attributes()
            .unwrap()
            .iter()
            .map(|field| field.name())
            .collect::<Vec<_>>(),
        ["ФлагПробы", "ТекстПробы"]
    );
    let sections = owner.data_processor_tabular_sections().unwrap();
    assert_eq!(
        sections
            .iter()
            .map(|section| section.name())
            .collect::<Vec<_>>(),
        ["СтрокиПробы", "ДругаяЧасть", "ПустаяЧасть"]
    );
    let fields = sections[0].attributes().unwrap();
    assert_eq!(
        fields
            .iter()
            .map(DocumentAttribute::name)
            .collect::<Vec<_>>(),
        ["ТекстСтроки", "ФлагСтроки"]
    );
    assert_eq!(fields[0].types()[0].id, TypeRef::Native(TypeId::String));
    assert_eq!(fields[0].types()[0].quals, ["12", "1"]);
    assert_eq!(fields[1].types()[0].id, TypeRef::Native(TypeId::Boolean));
    assert_eq!(
        owner.data_processor_attributes().unwrap()[1].types()[0].quals,
        ["12", "1"]
    );
    assert!(sections[2].attributes().unwrap().is_empty());
    let empty_id = "4722aab2-abc1-40ce-80b4-6f540dfa1891";
    let empty = projected(
        empty_id,
        MetadataKind::DataProcessor,
        include_bytes!("../tests/fixtures/data-processor-0911/ПустаяОбработка.deflate"),
        false,
    );
    let owner = empty.object_by_id(empty_id).unwrap();
    assert!(owner.data_processor_attributes().unwrap().is_empty());
    assert!(owner.data_processor_tabular_sections().unwrap().is_empty());
}

#[test]
fn processor_projection_builder_preserves_unknown_and_rejects_wrong_owner() {
    let catalog = || {
        MetadataCatalog::new(
            "processor-projection",
            vec![
                bsl_orm::MetadataObject::new("processor", "Обработка", MetadataKind::DataProcessor),
                bsl_orm::MetadataObject::new("catalog", "Справочник", MetadataKind::Catalog),
            ],
        )
        .unwrap()
    };
    for owner in ["missing", "catalog"] {
        assert!(matches!(
            catalog().with_data_processor_metadata(owner, Some(vec![]), Some(vec![])),
            Err(bsl_orm::CatalogError::UnknownDataProcessorOwner(_))
        ));
    }
    let known = catalog()
        .with_data_processor_metadata("processor", Some(vec![]), Some(vec![]))
        .unwrap();
    assert!(
        known
            .object_by_id("processor")
            .unwrap()
            .data_processor_attributes()
            .unwrap()
            .is_empty()
    );
    let unknown = known
        .with_data_processor_metadata("processor", None, None)
        .unwrap();
    let owner = unknown.object_by_id("processor").unwrap();
    assert!(owner.data_processor_attributes().is_none());
    assert!(owner.data_processor_tabular_sections().is_none());
}

#[test]
fn native_processor_section_fields_keep_the_acquired_nonalphabetic_order() {
    let id = "ff92317d-71e7-4664-a13a-18bd31fd3701";
    let catalog = projected(
        id,
        MetadataKind::DataProcessor,
        include_bytes!("../tests/fixtures/data-processor-0911/order.deflate"),
        false,
    );
    let fields = catalog
        .object_by_id(id)
        .unwrap()
        .data_processor_tabular_sections()
        .unwrap()[1]
        .attributes()
        .unwrap();
    assert_eq!(
        fields
            .iter()
            .map(DocumentAttribute::name)
            .collect::<Vec<_>>(),
        ["ДругойФлагСтроки", "ДругойТекстСтроки"]
    );
    assert_eq!(fields[0].types()[0].id, TypeRef::Native(TypeId::Boolean));
    assert_eq!(fields[1].types()[0].id, TypeRef::Native(TypeId::String));
}

#[test]
fn acquired_processor_object_guid_projects_without_a_reference_decoder() {
    let id = "ff92317d-71e7-4664-a13a-18bd31fd3701";
    let mut descriptors = parse_config_descriptors(
        id,
        include_bytes!("../tests/fixtures/data-processor-0911/Замеры.deflate"),
    )
    .unwrap();
    let guid = descriptors
        .iter()
        .find(|d| d.object_guid.as_str() == id)
        .unwrap()
        .data_processor_object_type
        .clone()
        .unwrap();
    assert_eq!(guid.as_str(), "7992a8c3-af89-4b9f-ba1a-836068499055");
    // GUID приобретён из нативного артефакта; объявление реквизита — вход адаптера.
    let field = descriptors
        .iter_mut()
        .find(|d| d.name == "ФлагПробы")
        .unwrap();
    field.declared_types = Some(vec![open_sdbl::metadata::ConfigDeclaredType {
        tag: "#".into(),
        reference: Some(guid.clone()),
        qualifiers: vec![],
    }]);
    field.filling_value = Some(open_sdbl::metadata::ConfigFillingValue::Reference {
        type_id: guid,
        value_id: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
    });
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let catalog = MetadataCatalog::new(
        "processor-type-projection",
        vec![bsl_orm::MetadataObject::new(
            id,
            "Замеры",
            MetadataKind::DataProcessor,
        )],
    )
    .unwrap();
    let expected = Arc::clone(catalog.data_processor_object_types().next().unwrap());
    let projected = project(catalog, &metadata).unwrap();
    let field = &projected
        .object_by_id(id)
        .unwrap()
        .data_processor_attributes()
        .unwrap()[0];
    assert_eq!(field.types().len(), 1);
    assert_eq!(field.types()[0].id, TypeRef::Value(expected));
    assert!(field.fill_value().is_none());
    assert_eq!(Arc::new(projected).transferable_value_types().count(), 0);
}

#[test]
fn unknown_processor_object_guid_remains_an_unsupported_declared_type() {
    let ty = open_sdbl::metadata::ConfigDeclaredType {
        tag: "#".into(),
        reference: Some("7992a8c3-af89-4b9f-ba1a-836068499055".parse().unwrap()),
        qualifiers: vec![],
    };
    assert!(column(&ty, &References::new()).is_none());
}

#[test]
fn acquired_report_composition_reaches_runtime_catalog() {
    let id = "8bac013d-e4dc-43c2-980d-6190972b6678";
    let catalog = projected(
        id,
        MetadataKind::Report,
        include_bytes!("../tests/fixtures/report-object-sdk/ЗамерОтчета.deflate"),
        false,
    );
    let owner = catalog.object_by_id(id).unwrap();
    assert_eq!(owner.report_attributes().unwrap().len(), 2);
    assert_eq!(owner.report_tabular_sections().unwrap().len(), 1);
    assert_eq!(
        owner.report_tabular_sections().unwrap()[0]
            .attributes()
            .unwrap()
            .len(),
        2
    );
}
