use super::*;
use open_bsl::{
    BoundConfigurationSource, ConfigurationArtifacts, ConfigurationSource, XmlConfigurationSource,
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn selected_local_bsl_runs_in_the_existing_factory_and_sessions_keep_separate_modules() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = Directory(std::env::temp_dir().join(format!(
        "open-bsl-file-session-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    let root = &directory.0;
    std::fs::create_dir_all(root.join("CommonModules/Вычисляемый/Ext")).unwrap();
    std::fs::create_dir_all(root.join("Ext")).unwrap();
    let ns = "http://v8.1c.ru/8.3/MDClasses";
    std::fs::write(root.join("Configuration.xml"), format!(r#"<MetaDataObject xmlns="{ns}" version="2.20"><Configuration uuid="11111111-1111-1111-1111-111111111111"><Properties><Name>ФайловаяПроба</Name></Properties><ChildObjects><CommonModule>Вычисляемый</CommonModule></ChildObjects></Configuration></MetaDataObject>"#)).unwrap();
    std::fs::write(root.join("CommonModules/Вычисляемый.xml"), format!(r#"<MetaDataObject xmlns="{ns}" version="2.20"><CommonModule uuid="22222222-2222-2222-2222-222222222222"><Properties><Name>Вычисляемый</Name><Server>true</Server></Properties></CommonModule></MetaDataObject>"#)).unwrap();
    std::fs::write(root.join("CommonModules/Вычисляемый/Ext/Module.bsl"), "Перем Счетчик; Функция Следующее() Экспорт Если Счетчик = Неопределено Тогда Счетчик = 0; КонецЕсли; Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции").unwrap();
    std::fs::write(root.join("Ext/SessionModule.bsl"), "Процедура УстановкаПараметровСеанса(Требуемые) Имя = \"Вычисляемый\"; Цель = Вычислить(Имя); ПараметрыСеанса.Результат = Цель.Следующее() + Цель.Следующее(); КонецПроцедуры").unwrap();
    let files = Arc::new(XmlConfigurationSource::open(root).unwrap());
    let source: Arc<dyn ConfigurationSource> = Arc::new(
        BoundConfigurationSource::new(files.clone(), files.root_identity(), "database-namespace")
            .unwrap(),
    );
    let catalog = Arc::new(
        MetadataCatalog::new(
            "database-namespace",
            vec![bsl_orm::MetadataObject::new(
                "result",
                "Результат",
                MetadataKind::SessionParameter,
            )],
        )
        .unwrap()
        .with_configuration_object_id(files.root_identity())
        .unwrap()
        .with_file_source(source.clone())
        .unwrap(),
    );
    let factory = SessionFactory::new_with_source(
        source,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
        std::iter::empty::<String>(),
    )
    .unwrap();
    for number in [1, 2] {
        let session = factory
            .open(
                Principal::new("user", "Пользователь", vec![]),
                SessionResources::new(Box::new(()), number),
            )
            .unwrap();
        assert_eq!(
            session.parameters().peek("Результат").unwrap(),
            Some(Value::number_from_i64(3))
        );
    }
}

#[test]
fn native_common_and_owner_templates_reach_the_existing_bsl_methods() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/file-configuration-native-1022/native-export");
    let files = Arc::new(XmlConfigurationSource::open(root).unwrap());
    let objects = files
        .descriptions()
        .iter()
        .filter(|d| d.kind.as_ref() == "DataProcessor")
        .map(|d| {
            bsl_orm::MetadataObject::new(
                d.id.to_string(),
                d.name.to_string(),
                MetadataKind::DataProcessor,
            )
            .with_data_processor_attributes(vec![])
        })
        .collect();
    let source: Arc<dyn ConfigurationSource> = Arc::new(
        BoundConfigurationSource::new(
            files.clone(),
            files.root_identity(),
            "file-template-session",
        )
        .unwrap(),
    );
    let catalog = Arc::new(
        MetadataCatalog::new("file-template-session", objects)
            .unwrap()
            .with_configuration_object_id(files.root_identity())
            .unwrap()
            .with_file_source(source.clone())
            .unwrap(),
    );
    let factory = SessionFactory::new_with_source(
        source,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
        std::iter::empty::<String>(),
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Пользователь", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт ТекстМакета = ПолучитьОбщийМакет(\"ТекстФайла\"); МакетОбъекта = Обработки.Замеры.ПолучитьМакет(\"МакетОбъекта\"); Возврат ТекстМакета.ПолучитьТекст() = \"Контроль & <тег>\" + Символы.ПС + \"Вторая строка\" И МакетОбъекта.ПолучитьТекст() = \"Макет владельца\"; КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
