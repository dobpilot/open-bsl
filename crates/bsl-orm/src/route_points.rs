//! Приобретённые точки маршрута без исполнения бизнес-процесса.

use super::*;

/// Собственная точка маршрута; идентичность включает владельца.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessProcessRoutePoint {
    /// Идентификатор владельца — бизнес-процесса.
    pub owner_id: String,
    /// Идентификатор точки внутри владельца.
    pub point_id: String,
    /// Имя для доступа из BSL.
    pub name: String,
    /// Приобретённые пары кода языка и представления.
    pub synonyms: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub(super) struct Composition {
    points: Arc<[BusinessProcessRoutePoint]>,
    ty: Arc<ValueTypeDescriptor>,
}

impl PartialEq for Composition {
    fn eq(&self, other: &Self) -> bool {
        self.points == other.points
            && self.ty.identity() == other.ty.identity()
            && self.ty.names() == other.ty.names()
    }
}

impl MetadataCatalog {
    /// Присоединяет полный приобретённый состав точек и GUID типа ссылки.
    /// `None` сохраняет неизвестный состав; пустой список означает известную пустоту.
    /// Синонимы владельца задают представление типа на основном языке конфигурации.
    /// При неизвестном типе ссылки коллекция остаётся недоступной.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного владельца, другого вида объекта,
    /// пустого типа, неверной принадлежности или повторяющихся имён и GUID точек.
    pub fn with_business_process_route_points(
        mut self,
        owner: &str,
        reference_type: Option<&str>,
        points: Option<Vec<BusinessProcessRoutePoint>>,
        owner_synonyms: &[(String, String)],
    ) -> Result<Self, CatalogError> {
        let invalid = || CatalogError::InvalidRoutePoints(owner.into());
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::BusinessProcess
            })
            .ok_or_else(invalid)?;
        let composition = match (reference_type, points) {
            (_, None) | (None, _) => None,
            (Some(reference_type), Some(points)) => {
                if reference_type.is_empty()
                    || points.iter().enumerate().any(|(index, point)| {
                        point.owner_id != owner
                            || point.point_id.is_empty()
                            || point.name.is_empty()
                            || points[..index].iter().any(|known| {
                                known.point_id == point.point_id
                                    || folded_eq(&known.name, &point.name)
                            })
                    })
                {
                    return Err(invalid());
                }
                let name = self.objects[index].name();
                let presentation = primary_language_code(&self)
                    .and_then(|language| owner_synonyms.iter().find(|(code, _)| code == language))
                    .map_or(name, |(_, text)| text.as_str());
                Some(Composition {
                    points: points.into(),
                    ty: Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{}/route-point/{owner}/{reference_type}", self.identity),
                        format!("{presentation}(route point)"),
                        [
                            format!("ТочкаМаршрутаБизнесПроцессаСсылка.{name}"),
                            format!("BusinessProcessRoutePointRef.{name}"),
                        ],
                    )),
                })
            }
        };
        Arc::make_mut(&mut self.objects)[index].route_points = composition;
        Ok(self)
    }

    /// Возвращает полный состав точек владельца; `None` — неизвестный состав.
    pub fn business_process_route_points(
        &self,
        owner: &str,
    ) -> Option<&[BusinessProcessRoutePoint]> {
        self.object_by_id(owner)?
            .route_points
            .as_ref()
            .map(|composition| composition.points.as_ref())
    }

    /// Типы ссылок приобретённых точек для регистрации через фасад.
    pub fn route_point_reference_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.objects.iter().filter_map(|object| {
            object
                .route_points
                .as_ref()
                .map(|composition| &composition.ty)
        })
    }
}

#[derive(Debug)]
struct Collection {
    catalog: Arc<MetadataCatalog>,
    composition: Composition,
}

impl Collection {
    fn reference(&self, index: usize) -> Value {
        Value::new_object(Reference {
            catalog: Arc::clone(&self.catalog),
            composition: self.composition.clone(),
            index,
        })
    }
    fn index(&self, name: &str) -> RtResult<usize> {
        self.composition
            .points
            .iter()
            .position(|point| folded_eq(&point.name, name))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }
}

impl ObjectProtocol for Collection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COLLECTION_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.composition.points.len())
    }
    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        Ok(self.reference(self.index(name)?))
    }
    fn set_property(&self, name: &str, _: Value, _: &mut CallContext<'_>) -> RtResult<()> {
        // Платформа принимает запись известного члена, сохраняя прежнюю ссылку.
        self.index(name).map(|_| ())
    }
    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Number(number) = index else {
            return Err(RtError::BadIndex);
        };
        let index = number
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        if index >= self.composition.points.len() {
            return Err(RtError::IndexOutOfBounds {
                index: index as i64,
                len: self.composition.points.len(),
            });
        }
        Ok(self.reference(index))
    }
}

#[derive(Debug)]
struct Reference {
    catalog: Arc<MetadataCatalog>,
    composition: Composition,
    index: usize,
}

impl ObjectProtocol for Reference {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &REFERENCE_TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.composition.ty))
    }
    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
    fn value_eq(&self, other: &open_bsl::ObjectRef) -> Option<bool> {
        Some(other.downcast_ref::<Self>().is_some_and(|other| {
            self.composition.ty.identity() == other.composition.ty.identity()
                && self.composition.points[self.index].point_id
                    == other.composition.points[other.index].point_id
        }))
    }
    fn value_hash_key(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.composition.ty.identity().hash(&mut hash);
        self.composition.points[self.index].point_id.hash(&mut hash);
        Some(hash.finish())
    }
    fn presentation(&self) -> RtResult<String> {
        Ok(self.display())
    }
    fn display(&self) -> String {
        let point = &self.composition.points[self.index];
        let language = primary_language_code(&self.catalog);
        language
            .and_then(|language| point.synonyms.iter().find(|(code, _)| code == language))
            .map_or_else(|| point.name.clone(), |(_, text)| text.clone())
    }
}

pub(super) fn manager_property(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or_else(|| RtError::DynamicError("ожидался менеджер бизнес-процесса".into()))?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::UnknownProperty("ТочкиМаршрута".into()))?;
    if object.kind() != MetadataKind::BusinessProcess {
        return Err(RtError::UnknownProperty("ТочкиМаршрута".into()));
    }
    let composition = object
        .route_points
        .clone()
        .ok_or_else(|| RtError::DynamicError("состав точек маршрута не приобретён".into()))?;
    Ok(Value::new_object(Collection {
        catalog: Arc::clone(&manager.catalog),
        composition,
    }))
}

pub(super) static COLLECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ТочкиМаршрутаБизнесПроцесса",
    type_display: "Точки маршрута бизнес-процесса",
    type_names: &["BusinessProcessRoutePoints"],
};
pub(super) static REFERENCE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ТочкаМаршрутаБизнесПроцессаСсылка",
    type_display: "Business process route point reference",
    type_names: &["BusinessProcessRoutePointRef"],
};
pub(super) static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Количество", "Count"],
    Arity::exact(0),
    metadata_collection_count,
)];

fn primary_language_code(catalog: &MetadataCatalog) -> Option<&str> {
    catalog
        .primary_language_id()
        .and_then(|id| catalog.object_by_id(id))
        .and_then(|object| {
            object
                .string_properties
                .iter()
                .find(|(name, _)| name.as_ref() == "КодЯзыка")
        })
        .map(|(_, code)| code.as_ref())
}
