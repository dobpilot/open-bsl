//! Компиляция фрагмента `Выполнить`/`Вычислить`.
//!
//! Реализация той стороны [`bsl_bytecode::DynamicCompiler`], которая
//! действительно умеет компилировать: сам контракт — нейтральный и лежит в
//! `bsl-bytecode`, потому что его знают обе стороны границы, а фронтенд
//! знает только хост. Кэш скомпилированных фрагментов не здесь и не в VM:
//! им владеет тот, кто знает, сколько живёт сессия.

use bsl_bytecode::{DynamicKind, DynamicRequest, DynamicUnit, LibraryRequirement};

/// Лексика, разбор, резолвинг и кодоген фрагмента — весь фронтенд одним
/// вызовом.
///
/// Это ровно тот путь, которым компилируется статический код: один движок,
/// одна семантика, второго интерпретатора динамики не существует.
///
/// `symbols` — символы условной компиляции окружения. Платформа гасит их
/// все у динамического кода; здесь фрагмент видит тот же набор, что и
/// модуль вокруг него — сознательное отступление, см.
/// `docs/reference/language/preprocessor.md`.
///
/// `scope` — номер, под которым хост будет знать область этого фрагмента
/// (см. [`DynamicUnit::scope`]); здесь он только переносится в результат.
/// Придумывать идентичность прогона компилятору нечем и незачем.
///
/// # Errors
///
/// Текст первой ошибки любой из фаз, а также сообщение о конфликте версий
/// компонентов между программой и фрагментом.
pub fn compile_dynamic_snippet(
    request: &DynamicRequest<'_>,
    registry: Option<&bsl_rt::RuntimeRegistry>,
    symbols: &bsl_syntax::PreprocSymbols,
    scope: std::num::NonZeroU64,
) -> Result<DynamicUnit, String> {
    // `Вычислить` заворачивается в `Возврат (...)`, чтобы значение
    // выражения получалось тем же путём, что и у обычного `Возврат`.
    let source = match request.kind {
        DynamicKind::Eval => format!("Возврат ({});", request.source),
        DynamicKind::Execute => request.source.to_string(),
    };

    let parsed = bsl_syntax::parse_with_symbols(&source, symbols).map_err(|e| format!("{e}"))?;
    let mut stmts = Vec::with_capacity(parsed.items.len());
    for item in parsed.items {
        match item {
            bsl_syntax::Item::Stmt(s) => stmts.push(s),
            bsl_syntax::Item::VarDecl(vd) => stmts.push(bsl_syntax::Stmt::VarDecl(vd)),
            // НЕ ИЗМЕРЕНО(EXEC.PROC_DECLARATION): может ли фрагмент вообще
            // объявлять процедуры и функции. Взято «нет» — объявленную
            // процедуру было бы некуда деть: таблица чанков программы уже
            // скомпилирована.
            _ => {
                return Err(
                    "Выполнить/Вычислить не поддерживают объявление процедур/функций".to_string(),
                );
            }
        }
    }

    let signatures: Vec<bsl_sema::SnippetSignature> = request
        .functions
        .iter()
        .map(|f| bsl_sema::SnippetSignature {
            name: f.name.to_string(),
            arity: f.arity,
            is_procedure: f.is_procedure,
            has_default: f.param_has_default.to_vec(),
        })
        .collect();
    let (all_locals, body, fragment_requirements) = match registry {
        Some(registry) => {
            let resolved = if request.caller_is_async {
                bsl_sema::resolve_async_snippet_stmts_with_registry(
                    request.locals,
                    request.module_vars,
                    &stmts,
                    &signatures,
                    registry,
                )
            } else {
                bsl_sema::resolve_snippet_stmts_with_registry(
                    request.locals,
                    request.module_vars,
                    &stmts,
                    &signatures,
                    registry,
                )
            };
            resolved.map_err(|e| format!("{e}"))?
        }
        None => {
            let resolved = if request.caller_is_async {
                bsl_sema::resolve_async_snippet_stmts(
                    request.locals,
                    request.module_vars,
                    &stmts,
                    &signatures,
                )
            } else {
                bsl_sema::resolve_snippet_stmts(
                    request.locals,
                    request.module_vars,
                    &stmts,
                    &signatures,
                )
            };
            let (locals, body) = resolved.map_err(|e| format!("{e}"))?;
            (locals, body, vec![LibraryRequirement::bsl_rt()])
        }
    };
    let requirements = merge_requirements(request.requirements, &fragment_requirements)?;
    let callee_params: Vec<Vec<bool>> = request
        .functions
        .iter()
        .map(|f| f.param_by_val.to_vec())
        .collect();
    let crate::SnippetUnit {
        mut chunk,
        names,
        shapes,
    } = crate::compile_snippet_with_requirements(
        &all_locals,
        &body,
        request.names,
        &callee_params,
        &requirements,
    )
    .map_err(|e| format!("{e}"))?;
    chunk.is_async = request.caller_is_async;

    Ok(DynamicUnit {
        scope,
        chunk,
        names,
        shapes,
        requirements,
    })
}

/// Требования программы плюс требования фрагмента. Нулевая позиция —
/// всегда `bsl-rt`, остальные упорядочены по имени пакета, потому что от
/// этого порядка зависят номера библиотек в инструкциях.
fn merge_requirements(
    base: &[LibraryRequirement],
    extra: &[LibraryRequirement],
) -> Result<Vec<LibraryRequirement>, String> {
    let mut merged = base.to_vec();
    for requirement in extra {
        match merged
            .iter()
            .find(|existing| existing.package == requirement.package)
        {
            Some(existing) if existing.version != requirement.version => {
                return Err(format!(
                    "для {} одновременно требуются версии {} и {}",
                    requirement.package, existing.version, requirement.version
                ));
            }
            Some(_) => {}
            None => merged.push(requirement.clone()),
        }
    }
    merged[1..].sort_by(|left, right| left.package.cmp(&right.package));
    Ok(merged)
}
