//! Холодное связывание имён корневого модуля с явным сеансом.

use bsl_bytecode::Program;
use bsl_rt::RuntimeShapes;

pub(super) fn bind(program: &mut Program, runtime: &mut RuntimeShapes) -> Result<(), crate::Error> {
    bsl_vm::bind_program_shapes(program, runtime)
        .map(|_| ())
        .map_err(|error| match error {
            bsl_rt::RtError::DynamicError(message) => crate::Error::Configuration(message),
            other => crate::Error::Runtime(other),
        })
}

#[cfg(test)]
#[path = "instance_names_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "instance_large_names_tests.rs"]
mod large_names_tests;
