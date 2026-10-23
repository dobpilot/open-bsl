//! `СистемнаяИнформация`.

use super::*;

#[derive(Debug)]
struct SystemInformationObject {
    platform_version: Arc<str>,
}

pub(crate) static SYSTEM_INFORMATION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-session", "СистемнаяИнформация");

impl ObjectProtocol for SystemInformationObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SYSTEM_INFORMATION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SYSTEM_INFORMATION_PROPERTIES
    }
}

pub(crate) fn construct_system_information(
    ctx: &mut CallContext<'_>,
    _: &[Value],
) -> RtResult<Value> {
    let platform_version = runtime(ctx)?.platform_version.clone();
    Ok(Value::new_object(SystemInformationObject {
        platform_version,
    }))
}

fn system_information_app_version(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let information = receiver
        .downcast_ref::<SystemInformationObject>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(information.platform_version.as_ref().into()))
}

pub(crate) static SYSTEM_INFORMATION_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ВерсияПриложения", "AppVersion"],
    get: system_information_app_version,
    set: None,
}];
