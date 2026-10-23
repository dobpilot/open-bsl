use super::*;

const JOB: &str = "9ef2eacd-8de2-4a5d-9f0f-fa06f16b469f";
const MODULE: &str = "430b88f0-d9a9-475b-a409-a6fbad144999";

fn scheduled_reader() -> Reader {
    let mut source = reader();
    let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
        .replace(
            ",1,{061d872a-5787-460e-95ac-ed74ea3a3e84,1,571e1bd9-0f64-4a05-a276-dba4fe242ad0}",
            &format!(",2,{{{COMMON},1,{MODULE}}},{{{SCHEDULED_JOB},1,{JOB}}}"),
        );
    source.bootstrap.descriptor = text(DESCRIPTOR, &root);
    for (name, bytes) in [
        (JOB, include_bytes!("../../../docs/research/fixtures/scheduled-job-method-name-1015/9ef2eacd-8de2-4a5d-9f0f-fa06f16b469f.deflate").as_slice()),
        (MODULE, include_bytes!("../../../docs/research/fixtures/scheduled-job-method-name-1015/430b88f0-d9a9-475b-a409-a6fbad144999.deflate").as_slice()),
    ] {
        source.resources.insert(name.into(), ConfigResource {
            file_name: name.into(),
            compressed: Arc::from(bytes),
        });
    }
    source.resources.insert(
        format!("{MODULE}.0"),
        text(
            &format!("{MODULE}.0"),
            "ВызватьИсключение \"тело не требуется для метаданных\";",
        ),
    );
    source
}

#[test]
fn scheduled_job_dependency_reads_only_the_referenced_module_header() {
    let reader = scheduled_reader();
    let reads = reader.reads.clone();
    let source = Arc::new(
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap(),
    );
    let catalog = Arc::new(source.metadata_catalog().unwrap());
    reads.lock().unwrap().clear();
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(std::rc::Rc::new(bsl_orm::SessionData::new(
            catalog,
            Principal::new("user", "", vec![]),
        )))
        .build();
    assert_eq!(
        state
            .exec("Возврат Метаданные.РегламентныеЗадания[0].ИмяМетода;")
            .unwrap(),
        open_bsl::Value::Str("ОчередьЗаданийСлужебный.РегламентнаяОбработкаОчередиЗаданий".into())
    );
    assert_eq!(*reads.lock().unwrap(), [JOB, MODULE]);
}

#[test]
fn scheduled_job_dependency_failure_keeps_the_job_unpublished_and_retryable() {
    let reader = scheduled_reader();
    let reads = reader.reads.clone();
    let source = Arc::new(
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap(),
    );
    let _ = source.metadata_catalog().unwrap();
    let mut unavailable = scheduled_reader();
    unavailable.resources.remove(MODULE);
    unavailable.reads = reads.clone();
    *source.connection.lock().unwrap() = Box::new(unavailable);
    assert!(bsl_orm::MetadataObjectSource::load_object(source.as_ref(), JOB).is_err());
    assert!(source.metadata_snapshot(&[JOB]).is_err());
    let mut restored = scheduled_reader();
    restored.reads = reads.clone();
    *source.connection.lock().unwrap() = Box::new(restored);
    reads.lock().unwrap().clear();
    assert!(bsl_orm::MetadataObjectSource::load_object(source.as_ref(), JOB).is_ok());
    assert_eq!(*reads.lock().unwrap(), [JOB, MODULE]);
}
