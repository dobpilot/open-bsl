use super::{FunctionCode, FunctionKind, LibraryDescriptor, RegistryError};

pub(super) fn build_globals(
    libraries: &[LibraryDescriptor],
) -> Result<Vec<(&'static str, u8, FunctionCode)>, RegistryError> {
    let mut globals: Vec<(&'static str, u8, FunctionCode)> = Vec::new();
    for (index, library) in libraries.iter().enumerate() {
        for global in library.globals() {
            if global.names.is_empty() || global.names.iter().any(|name| name.is_empty()) {
                return Err(RegistryError::EmptyNames {
                    package: library.package().into(),
                    code: global.function.get(),
                });
            }
            if !library.functions().iter().any(|function| {
                function.code == global.function
                    && function.kind != FunctionKind::Procedure
                    && function.arity.accepts(0)
            }) {
                return Err(RegistryError::InvalidGlobalFunction {
                    package: library.package().into(),
                    code: global.function,
                });
            }
            for name in global.names {
                if matches!(
                    crate::BuiltinFn::lookup(name),
                    Some(crate::BuiltinFn::CommandLineArguments | crate::BuiltinFn::ThisObject)
                ) || crate::lookup_enum(name).is_some()
                    || [
                        "Символы",
                        "Chars",
                        "ФабрикаXDTO",
                        "XDTOFactory",
                        "ФайловыеПотоки",
                        "FileStreams",
                        "ФоновыеЗадания",
                        "BackgroundJobs",
                    ]
                    .iter()
                    .any(|reserved| crate::folded_eq(reserved, name))
                {
                    return Err(RegistryError::ReservedGlobalName((*name).into()));
                }
                if globals
                    .iter()
                    .any(|(previous, _, _)| crate::folded_eq(previous, name))
                {
                    return Err(RegistryError::DuplicateGlobalName((*name).into()));
                }
                globals.push((name, index as u8, global.function));
            }
        }
    }
    Ok(globals)
}
