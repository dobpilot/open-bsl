use super::*;
use std::sync::Arc;

const JOB: &str = "9ef2eacd-8de2-4a5d-9f0f-fa06f16b469f";
const MODULE: &str = "430b88f0-d9a9-475b-a409-a6fbad144999";
const JOB_BYTES: &[u8] = include_bytes!(
    "../../../docs/research/fixtures/scheduled-job-method-name-1015/9ef2eacd-8de2-4a5d-9f0f-fa06f16b469f.deflate"
);
const MODULE_BYTES: &[u8] = include_bytes!(
    "../../../docs/research/fixtures/scheduled-job-method-name-1015/430b88f0-d9a9-475b-a409-a6fbad144999.deflate"
);

fn resource(name: &str, compressed: &[u8]) -> ConfigResource {
    ConfigResource {
        file_name: name.into(),
        compressed: Arc::from(compressed),
    }
}

fn job() -> MetadataObject {
    MetadataObject::new(
        JOB,
        "РегламентнаяОбработкаОчередиЗадач",
        MetadataKind::ScheduledJob,
    )
}

#[test]
fn native_scheduled_job_projects_both_runtime_aliases() {
    let source = [resource(JOB, JOB_BYTES), resource(MODULE, MODULE_BYTES)];
    let projected = project(job(), &Resources::new(&source).unwrap()).unwrap();
    let method = "ОчередьЗаданийСлужебный.РегламентнаяОбработкаОчередиЗаданий";
    assert_eq!(
        projected,
        job()
            .with_string_property("ИмяМетода", method)
            .with_string_property("MethodName", method)
    );
}

#[test]
fn an_unknown_job_shape_does_not_invent_a_method_name() {
    let source = [resource(JOB, MODULE_BYTES)];
    assert_eq!(
        project(job(), &Resources::new(&source).unwrap()).unwrap(),
        job()
    );
}

#[test]
fn a_missing_explicit_common_module_fails_projection() {
    let source = [resource(JOB, JOB_BYTES)];
    assert!(matches!(
        project(job(), &Resources::new(&source).unwrap()),
        Err(LoadError::MissingResource(id)) if id == MODULE
    ));
}

#[test]
fn an_explicit_target_of_another_class_fails_projection() {
    let source = [resource(JOB, JOB_BYTES), resource(MODULE, JOB_BYTES)];
    assert!(project(job(), &Resources::new(&source).unwrap()).is_err());
}

#[test]
fn projected_method_names_are_readable_through_the_facade() {
    let source = [resource(JOB, JOB_BYTES), resource(MODULE, MODULE_BYTES)];
    let projected = project(job(), &Resources::new(&source).unwrap()).unwrap();
    let catalog = Arc::new(MetadataCatalog::new("native-job", vec![projected]).unwrap());
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
        state.exec(
            "Задание = Метаданные.РегламентныеЗадания[0];
            Возврат Задание.ИмяМетода = Задание.mEtHoDnAmE
                И Задание.ИмяМетода = \"ОчередьЗаданийСлужебный.РегламентнаяОбработкаОчередиЗаданий\";"
        ).unwrap(),
        open_bsl::Value::Boolean(true)
    );
    assert!(
        state
            .exec("Метаданные.РегламентныеЗадания[0].ИмяМетода = \"Другая.Процедура\";")
            .is_err()
    );
}
