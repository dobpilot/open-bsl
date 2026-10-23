use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

const IMAGE: &[u8] =
    include_bytes!("../../../tests/conformance/measure/1c/native-image-module-initializer.image");

#[test]
fn source_control_preserves_private_storage_of_three_objects_in_one_session() {
    let source = include_str!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer-source.bsl"
    );
    assert_instances_match_native(source);
}

#[test]
fn recovered_initializer_preserves_private_storage_of_three_objects_in_one_session() {
    let source = super::decompile(IMAGE).unwrap();
    assert_instances_match_native(&source);
}

#[test]
fn initializer_boundaries_cannot_overlap_methods_or_hide_missing_termination() {
    for entry in [0, 7, 12, 14, 19, u32::MAX - 1] {
        let mut image = super::parse(IMAGE).unwrap();
        image.entry = entry;
        assert!(super::recover_module(&image).is_err(), "entry={entry}");
    }
    let mut image = super::parse(IMAGE).unwrap();
    image.instructions.last_mut().unwrap().operand = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn a_corrupt_initializer_rejects_the_whole_module_after_valid_method_bodies() {
    for instruction in [
        super::Instruction {
            code: 9999,
            operand: 0,
        },
        super::Instruction {
            code: 2,
            operand: 1,
        },
        super::Instruction {
            code: 3,
            operand: 0,
        },
        super::Instruction {
            code: 4,
            operand: 1,
        },
    ] {
        let mut image = super::parse(IMAGE).unwrap();
        image.instructions[14] = instruction;
        let error = super::recover_module(&image).unwrap_err();
        assert!(error.contains("image/инициализация"), "{error}");
        assert!(error.contains("Cmd[14]"), "{error}");
    }
}

#[test]
fn an_explicit_native_global_reference_cannot_silently_become_a_local_reference() {
    let mut image = super::parse(IMAGE).unwrap();
    image.procedures[1].variables.as_mut().unwrap()[0].name = "ЗначениеПробы".into();
    let error = super::recover_module(&image).unwrap_err();
    assert!(error.contains("глобальное имя перекрыто"), "{error}");
}

fn assert_instances_match_native(source: &str) {
    let name = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "object",
            name,
            source,
            ModuleKind::Object,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("catalog"),
    ];
    let object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription)
        .with_object_module(name);
    let factory = SessionFactory::new(
        ConfigurationCodeSnapshot::new("native-initializer", modules).unwrap(),
        Arc::new(MetadataCatalog::new("native-initializer", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let driver = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-catalog-module-initializer.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-catalog-module-initializer-source.platform.txt"
    )
    .replace("module_initializer_source.", "module_initializer.");
    for _ in 0..2 {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{driver}\nВозврат РезультатыПробы; КонецФункции"
            ))
            .unwrap();
        let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
        else {
            panic!("массив результатов");
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов");
        };
        let actual = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(actual, expected.trim());
    }
}

#[test]
fn separate_initializer_and_module_termination_preserve_native_instance_storage() {
    let image =
        include_bytes!("../../../docs/research/fixtures/native-initializer-double-end/image");
    let receipt =
        include_str!("../../../docs/research/fixtures/native-initializer-double-end/platform.txt");
    let original = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-module-initializer.platform.txt"
    );
    assert_eq!(receipt.trim(), original.trim());
    let source = super::decompile(image).unwrap();
    assert_instances_match_native(&source);
}

#[test]
fn separate_initializer_termination_keeps_corrupt_operands_and_body_unsupported() {
    let bytes =
        include_bytes!("../../../docs/research/fixtures/native-initializer-double-end/image");
    for index in [18, 19] {
        let mut image = super::parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err());
    }
    let mut image = super::parse(bytes).unwrap();
    image.instructions[15].code = 9999;
    assert!(super::recover_module(&image).is_err());
}
