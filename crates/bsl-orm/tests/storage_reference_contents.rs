use bsl_orm::{
    MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData, catalog_reference_value,
    library, value_storage_from_internal_with_resolver,
};
use open_bsl::{
    Arity, BslObject, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind,
    LibraryDescriptor, RtResult, Value, uuid,
};
use std::{rc::Rc, sync::Arc};

fn load(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let catalog = Arc::clone(
        context
            .host_data()?
            .downcast_ref::<SessionData>()
            .unwrap()
            .catalog(),
    );
    let mut resolve = |kind: &str, payload: &str| match kind {
        "fc01b5df-97fe-449b-83d4-218a090e681e" => Ok(Some(Value::Object(Rc::new(
            BslObject::Uuid(uuid::parse(payload)?),
        )))),
        "f4f1e787-e5b3-59af-ae48-c43f0c92e2c3" => {
            assert_eq!(payload, "53:abcd0123456789ab4321123412345678");
            catalog_reference_value(
                Arc::clone(&catalog),
                "catalog",
                uuid::parse("12345678-1234-4321-abcd-0123456789ab")?,
            )
            .map(Some)
        }
        _ => Ok(None),
    };
    value_storage_from_internal_with_resolver(
        r##"{"#",4238019d-7e49-4fc9-91db-b6b951d5cf8e,{2,{{"S","Ref"},{"#",f4f1e787-e5b3-59af-ae48-c43f0c92e2c3,53:abcd0123456789ab4321123412345678}},{{"S","Id"},{"#",fc01b5df-97fe-449b-83d4-218a090e681e,12345678-1234-4321-abcd-0123456789ab}}}}"##,
        context,
        &mut resolve,
    )
}

static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ПрочитатьСнимокСсылки"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: Some(load),
}];

#[test]
fn storage_materializes_registered_references_and_uuid_inside_detached_structure() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "storage-reference-control",
            vec![MetadataObject::new(
                "catalog",
                "ЗамерВременныхТаблиц",
                MetadataKind::Catalog,
            )],
        )
        .unwrap(),
    );
    let mut builder = Engine::builder()
        .register_library(library())
        .register_library(
            LibraryDescriptor::new("storage-reference-control", "1").with_functions(FUNCTIONS),
        );
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("control", "Контроль", vec![]),
        )))
        .build();
    let value = state
        .exec(
            r#"
        Хранилище = ПрочитатьСнимокСсылки();
        Первое = Хранилище.Получить();
        Ссылка = Первое.Ref;
        Идентификатор = Первое.Id;
        Первое.Ref = Неопределено;
        Второе = Хранилище.Get();
        Возврат Второе.Ref = Ссылка И Второе.Id = Идентификатор
            И Ссылка.УникальныйИдентификатор() = Идентификатор
            И Строка(Идентификатор) = "12345678-1234-4321-abcd-0123456789ab"
            И ТипЗнч(Ссылка) = Тип("СправочникСсылка.ЗамерВременныхТаблиц");
    "#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
}
