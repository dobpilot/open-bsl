//! Минимальная модель метаданных для среды без конфигурации 1С.

use crate::{
    Arity, BslValue, CallContext, MethodDescriptor, ObjectProtocol, PropertyDescriptor, RtError,
    RtResult, TypeDescriptor,
};

static METADATA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "Метаданные",
    type_display: "Метаданные",
    type_names: &["Metadata"],
};

static COMMON_MODULES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияОбщихМодулей",
    type_display: "Коллекция общих модулей",
    type_names: &["CommonModuleCollection"],
};

#[derive(Debug)]
struct MetadataObject;

impl ObjectProtocol for MetadataObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METADATA_PROPERTIES
    }
}

#[derive(Debug)]
struct CommonModulesObject;

impl ObjectProtocol for CommonModulesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COMMON_MODULES_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        COMMON_MODULE_METHODS
    }
}

fn common_modules(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver
        .downcast_ref::<MetadataObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(BslValue::new_object(CommonModulesObject))
}

fn find_common_module(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver
        .downcast_ref::<CommonModulesObject>()
        .ok_or(RtError::NotAnObject)?;
    // СОЗНАТЕЛЬНОЕ ОТКЛОНЕНИЕ: open-bsl исполняет отдельный модуль без
    // конфигурации 1С, поэтому коллекция общих модулей всегда пуста.
    Ok(BslValue::Undefined)
}

static METADATA_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ОбщиеМодули", "CommonModules"],
    get: common_modules,
    set: None,
}];

static COMMON_MODULE_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Найти", "Find"],
    Arity::exact(1),
    find_common_module,
)];

#[must_use]
pub fn new_metadata() -> BslValue {
    BslValue::new_object(MetadataObject)
}
