use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn native_edt_metadata_form_and_source_are_indexed_without_an_ide() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "open-bsl-edt-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let original = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/file-configuration-native-1022/edt-reference/GitConverter/src");
    let result = {
        std::fs::create_dir_all(root.join("Configuration")).unwrap();
        std::fs::write(root.join("Configuration/Configuration.mdo"), r#"<m:Configuration xmlns:m="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="a8dd2e89-c043-4f61-ab58-dafb034cd4c1"><name>Контроль</name><commonModules>CommonModule.ОбщегоНазначения</commonModules><catalogs>Catalog.ВерсииEDT</catalogs></m:Configuration>"#).unwrap();
        for path in [
            "CommonModules/ОбщегоНазначения/ОбщегоНазначения.mdo",
            "Catalogs/ВерсииEDT/ВерсииEDT.mdo",
            "Catalogs/ВерсииEDT/Forms/ФормаЭлемента/Form.form",
        ] {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(original.join(path), target).unwrap();
        }
        let text = "Функция ИзФайла() Экспорт\nВозврат 123;\nКонецФункции\n";
        std::fs::write(root.join("CommonModules/ОбщегоНазначения/Module.bsl"), text).unwrap();
        let source = EdtConfigurationSource::open(&root).unwrap();
        let common = source
            .index()
            .modules()
            .iter()
            .find(|m| m.name() == "ОбщегоНазначения")
            .unwrap();
        assert!(common.execution().server());
        assert!(common.execution().external_connection());
        assert_eq!(source.module_source(common.id()).unwrap().as_ref(), text);
        let artifacts = source.artifacts().unwrap();
        let form = artifacts
            .descriptions()
            .iter()
            .find(|d| d.kind.as_ref() == "Form")
            .unwrap();
        let artifact = artifacts.artifact(&form.id).unwrap().unwrap();
        assert!(
            std::str::from_utf8(&artifact.bytes)
                .unwrap()
                .contains("CatalogObject.ВерсииEDT")
        );
        assert_eq!(
            form.owner.as_deref(),
            Some("e7ae28d9-60d7-4498-b7f4-5d977663282a")
        );
        Ok::<_, String>(())
    };
    std::fs::remove_dir_all(root).unwrap();
    result.unwrap();
}
