use super::*;

/// Метаданные из типовой конфигурации лежат в неотслеживаемом каталоге
/// `private/`: без них тесты, которые их читают, пропускаются.
fn private_fixture(relative: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../private/fixtures")
        .join(relative);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            eprintln!("пропуск: нет приватной фикстуры {}", path.display());
            None
        }
    }
}

#[test]
fn legacy_defined_type_dependency_is_acquired_before_projecting_the_native_buh_field() {
    let Some(buh_owner) = private_fixture(
        "file-configuration-native-1022/native-buh-defined-reference/owner.deflate",
    ) else {
        return;
    };
    let Some(buh_definition) = private_fixture(
        "file-configuration-native-1022/native-buh-defined-reference/definition.deflate",
    ) else {
        return;
    };
    let Some(buh_target) =
        private_fixture("file-configuration-native-1022/native-buh-catalog-field/target.deflate")
    else {
        return;
    };
    let owner = "bffeceba-fe82-4593-9d34-edc03d99fa44";
    let definition = "28a25822-2306-409a-a7c8-6f7e42bf25e8";
    let target = "d5ada3af-5fe4-470a-a52d-888d88c9234f";
    let unused = COMMON_ID;
    let original = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt");
    let needle = "1,{061d872a-5787-460e-95ac-ed74ea3a3e84,1,571e1bd9-0f64-4a05-a276-dba4fe242ad0}";
    let collections = format!(
        "3,{{cf4abea6-37b2-11d4-940f-008048da11f9,2,{owner},{target}}},{{c045099e-13b9-4fb6-9d50-fca00202971e,1,{definition}}},{{{COMMON},1,{unused}}}"
    );
    assert_eq!(original.matches(needle).count(), 1);
    let mut reader = reader();
    reader.bootstrap.descriptor = text(DESCRIPTOR, &original.replace(needle, &collections));
    for (id, bytes) in [
        (owner, buh_owner.as_slice()),
        (definition, buh_definition.as_slice()),
        (target, buh_target.as_slice()),
    ] {
        reader.resources.insert(
            id.into(),
            ConfigResource {
                file_name: id.into(),
                compressed: bytes.into(),
            },
        );
    }
    let reads = Arc::clone(&reader.reads);
    let source =
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap();
    let object = bsl_orm::MetadataObjectSource::load_object(&source, owner).unwrap();
    let field = object
        .catalog_attributes()
        .unwrap()
        .iter()
        .find(|a| a.name() == "Подразделение")
        .unwrap();
    assert_eq!(field.types().len(), 1);
    assert_eq!(field.types()[0].id.name(), "ПодразделенияОрганизаций");
    let acquired = reads.lock().unwrap();
    for id in [owner, definition, target] {
        assert_eq!(
            acquired.iter().filter(|name| name.as_str() == id).count(),
            2
        );
    }
    assert_eq!(
        acquired
            .iter()
            .filter(|name| name.as_str() == unused)
            .count(),
        1
    );
}

#[test]
fn native_defined_type_dependencies_reach_the_selected_document_without_unrelated_details() {
    let event = "571e1bd9-0f64-4a05-a276-dba4fe242ad0";
    let basis = "51c403a3-e062-4f0d-b39c-11989c0ffe2b";
    let unused = "0169b236-52cf-4e6a-966e-2e180783b83b";
    let document_class = "061d872a-5787-460e-95ac-ed74ea3a3e84";
    let catalog_class = "cf4abea6-37b2-11d4-940f-008048da11f9";
    let defined_class = "c045099e-13b9-4fb6-9d50-fca00202971e";
    let rows = include_str!("../../../docs/research/unf-event-basis-reference-identities.tsv")
        .lines()
        .skip(1)
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut documents = vec![event];
    documents.extend(
        rows.iter()
            .filter(|row| row[0] == "Document" && row[1] != event)
            .map(|row| row[1]),
    );
    let mut catalogs = vec![unused];
    catalogs.extend(
        rows.iter()
            .filter(|row| row[0] == "Catalog")
            .map(|row| row[1]),
    );
    // Корень сокращён для счётного провайдера. Тела описателей и эталонные
    // идентичности принадлежат независимым нативным Config/XML-артефактам.
    let collections = format!(
        "3,{{{document_class},{},{}}},{{{catalog_class},{},{}}},{{{defined_class},1,{basis}}}",
        documents.len(),
        documents.join(","),
        catalogs.len(),
        catalogs.join(",")
    );
    let original = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt");
    let needle = format!("1,{{{document_class},1,{event}}}");
    assert_eq!(original.matches(&needle).count(), 1);
    let mut reader = reader();
    reader.bootstrap.descriptor = text(DESCRIPTOR, &original.replace(&needle, &collections));
    reader.resources.remove(COMMON_ID);
    for (id, bytes) in [
        (
            event,
            include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate").as_slice(),
        ),
        (
            basis,
            include_bytes!("../tests/fixtures/document-owned-attributes/basis.deflate").as_slice(),
        ),
        (
            unused,
            include_bytes!("../tests/fixtures/catalog-owners/tax-registration.deflate").as_slice(),
        ),
    ] {
        reader.resources.insert(
            id.into(),
            ConfigResource {
                file_name: id.into(),
                compressed: bytes.into(),
            },
        );
    }
    let resources = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/document-reference-owner-acquisition-1021/resources");
    for row in &rows {
        reader.resources.insert(
            row[1].into(),
            ConfigResource {
                file_name: row[1].into(),
                compressed: std::fs::read(resources.join(format!("{}.deflate", row[1])))
                    .unwrap()
                    .into(),
            },
        );
    }
    let reads = Arc::clone(&reader.reads);
    let source =
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap();
    let document = bsl_orm::MetadataObjectSource::load_object(&source, event).unwrap();
    let attribute = document
        .document_attributes()
        .unwrap()
        .iter()
        .find(|attribute| attribute.id() == "e350fdab-d2d1-45e8-bd75-a2ee410245f1")
        .unwrap();
    assert_eq!(attribute.types().len(), 56);
    for (column, row) in attribute.types().iter().zip(rows) {
        let open_bsl::TypeRef::Value(ty) = &column.id else {
            panic!("ожидался приобретённый ссылочный тип");
        };
        let prefix = if row[0] == "Document" {
            "DocumentRef"
        } else {
            "CatalogRef"
        };
        assert!(ty.names().contains(&format!("{prefix}.{}", row[3])));
    }
    let snapshot = source.metadata_snapshot(&[event]).unwrap();
    assert!(
        snapshot
            .descriptors()
            .iter()
            .any(|descriptor| descriptor.object_guid.as_str() == basis)
    );
    assert!(
        !snapshot
            .descriptors()
            .iter()
            .any(|descriptor| descriptor.object_guid.as_str() == unused)
    );
    assert_eq!(
        reads
            .lock()
            .unwrap()
            .iter()
            .filter(|id| id.as_str() == unused)
            .count(),
        1
    );
}
