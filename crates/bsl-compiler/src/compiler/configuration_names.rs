//! Узкие индексы методов в общей таблице большого каталога.

use super::*;
use std::collections::HashSet;

pub(super) fn is_capacity_error(error: &CompileError) -> bool {
    match error {
        CompileError::TooManyNames => true,
        CompileError::InModule(context) | CompileError::InMethod(context) => {
            is_capacity_error(&context.1)
        }
        _ => false,
    }
}

pub(super) fn check_links(
    resolved: &ResolvedProgram,
    names: &NameInterner,
) -> Result<(), CompileError> {
    for link in &resolved.links {
        if let bsl_sema::ResolvedLink::ObjectMethod {
            type_name, method, ..
        } = link
        {
            for name in [*type_name, method.as_str()] {
                if names
                    .lookup(name)
                    .is_some_and(|id| u16::try_from(id.index()).is_err())
                {
                    return Err(CompileError::TooManyNames);
                }
            }
        }
    }
    Ok(())
}

pub(super) fn collect(
    modules: &[(String, &ResolvedProgram)],
    entry: Option<&ResolvedProgram>,
    opts: BuildOptions,
) -> Result<Vec<String>, CompileError> {
    let mut canonical = NameInterner::new();
    let mut narrow = HashSet::new();
    for (name, resolved) in modules {
        collect_module(resolved, true, opts, &mut canonical, &mut narrow)
            .map_err(|source| CompileError::InModule(Box::new((name.clone(), source))))?;
    }
    if let Some(resolved) = entry {
        collect_module(resolved, false, opts, &mut canonical, &mut narrow)?;
    }
    Ok(canonical
        .into_names()
        .into_iter()
        .enumerate()
        .filter_map(|(index, name)| {
            narrow
                .contains(&bsl_rt::NameId::from_index(index as u32))
                .then_some(name)
        })
        .collect())
}

fn collect_module(
    resolved: &ResolvedProgram,
    configuration: bool,
    opts: BuildOptions,
    canonical: &mut NameInterner,
    narrow: &mut HashSet<bsl_rt::NameId>,
) -> Result<(), CompileError> {
    let mut names = NameInterner::new();
    let mut shapes = ShapeTable::new();
    // Повторяем существующий кодоген по одному модулю, не собирая связи
    // и не вводя второй обход семантических выражений. Крупные кадры
    // конфигурации сохраняют свой разрешённый путь уплотнения.
    let (chunks, _) = compile_module_chunks(
        resolved,
        &mut names,
        &mut shapes,
        opts,
        configuration,
        configuration,
    )?;
    for name in names.as_names() {
        canonical.intern(name);
    }
    let mut require = |name: &str| {
        narrow.insert(canonical.intern(name));
        if narrow.len() > usize::from(u16::MAX) + 1 {
            return Err(CompileError::TooManyNames);
        }
        Ok(())
    };
    for chunk in chunks {
        for instruction in chunk.instrs {
            let index = match instruction {
                Instr::CallObjectMethod { method, .. }
                | Instr::CallObjectProcedure { method, .. } => method,
                Instr::GetObjectProp { name, .. } | Instr::SetObjectProp { name, .. } => name,
                _ => continue,
            };
            require(&names.as_names()[usize::from(index)])?;
        }
    }
    // Таблица связей плоская; тип и метод в ней имеют отдельные `u16`.
    for link in &resolved.links {
        if let bsl_sema::ResolvedLink::ObjectMethod {
            type_name, method, ..
        } = link
        {
            require(type_name)?;
            require(method)?;
        }
    }
    Ok(())
}
