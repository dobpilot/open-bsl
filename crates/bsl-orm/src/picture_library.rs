//! Глобальное свойство `БиблиотекаКартинок` над общими картинками конфигурации.

use super::*;

#[derive(Debug)]
struct PictureLibrary {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct PictureValue {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
}

pub(crate) static PICTURE_LIBRARY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "БиблиотекаКартинок",
    type_display: "БиблиотекаКартинок",
    type_names: &["PictureLib"],
};

impl ObjectProtocol for PictureLibrary {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PICTURE_LIBRARY_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let object = self
            .catalog
            .object(MetadataKind::CommonPicture, name)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        Ok(Value::new_object(PictureValue {
            catalog: Arc::clone(&self.catalog),
            object_id: Arc::clone(&object.id),
        }))
    }
}

impl ObjectProtocol for PictureValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PICTURE_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        let index = self
            .catalog
            .objects
            .iter()
            .position(|object| object.id == self.object_id)?;
        Some((Arc::as_ptr(&self.catalog) as usize, index))
    }

    fn display(&self) -> String {
        "Picture".into()
    }
}

pub(crate) fn picture_library(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(PictureLibrary {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}
