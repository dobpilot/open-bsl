use super::*;

pub(super) const METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["НайтиПоТипу", "FindByType"],
    Arity::exact(1),
    find_by_type,
)];

fn find_by_type(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    let [Value::Type(ty)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Тип",
            op: "Метаданные.НайтиПоТипу",
        });
    };
    // Связь с владельцем уже задана при построении каталога. Представление
    // типа не является идентификатором и не участвует в обратном поиске.
    let owner = match ty {
        TypeRef::Value(ty) => root
            .catalog
            .reference_types
            .iter()
            .chain(&root.catalog.catalog_object_types)
            .chain(&root.catalog.catalog_selection_types)
            .find(|(_, candidate)| {
                candidate.package() == ty.package() && candidate.identity() == ty.identity()
            }),
        TypeRef::Object(descriptor) if descriptor.package == "bsl-orm" => None,
        _ => return Ok(Value::Undefined),
    };
    if let Some((id, _)) = owner {
        let object = root.catalog.materialized_object_by_id(id)?.ok_or_else(|| {
            RtError::DynamicError("владелец типа отсутствует в каталоге метаданных".into())
        })?;
        return Ok(metadata_object_value(&root.catalog, object));
    }
    Err(RtError::DynamicError(
        "поиск метаданных для этого типа конфигурации не поддерживается".into(),
    ))
}
