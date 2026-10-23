use bsl_orm::{MetadataCatalog, Principal, SessionData, library};
use open_bsl::{BslObject, Engine, Value};
use std::{rc::Rc, sync::Arc};

#[test]
fn dot_access_matches_the_complete_native_control() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::new(MetadataCatalog::new("compatibility", vec![]).unwrap()),
            Principal::new("user", "Тест", vec![]),
        )))
        .build();
    let source =
        include_str!("../../../../docs/research/fixtures/compatibility-property-dot/measure.bsl")
            .replace("Сообщить(", "РезультатПробы.Добавить(");
    let Value::Object(object) = state.exec(&format!(
        "Перем РезультатПробы; РезультатПробы = Новый Массив;\n{source}\nВозврат РезультатПробы;"
    )).unwrap() else { panic!("ожидался массив результатов"); };
    let BslObject::Array(array) = object.as_ref() else {
        panic!("ожидался массив результатов");
    };
    let actual = array
        .borrow()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(
        actual,
        include_str!(
            "../../../../docs/research/fixtures/compatibility-property-dot/measure.platform.txt"
        )
    );
    assert_eq!(
        state
            .exec("Возврат Строка(Метаданные.ObjectProperties.CompatibilityMode.dOnTuSe);")
            .unwrap()
            .to_string(),
        "НеИспользовать"
    );
    let error = state
        .exec("Возврат Метаданные.СвойстваОбъектов.РежимСовместимости.НетТакогоЧлена;")
        .unwrap_err();
    assert!(error.to_string().contains("НетТакогоЧлена"));
}
