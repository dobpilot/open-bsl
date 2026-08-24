use std::collections::{HashMap, HashSet};

use bsl_number::BslNumber;
use bsl_syntax::{Expr as AExpr, Item, LValue, Stmt as AStmt};

use crate::core_receivers::{self, CoreReceiver};
use crate::resolved::{
    RExpr, RStmt, Resolved, ResolvedArg, ResolvedFunction, ResolvedParam, ResolvedProgram,
};

/// Позиция, в которой разрешается вызов.
///
/// Различие несёт СЕМАНТИКУ, а не удобство: глобальная процедура законна
/// только как оператор (`Сообщить(1);`), а в выражении — ошибка, потому
/// что значения у неё нет.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallPosition {
    Expression,
    Statement,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemaError {
    /// Идентификатор читается раньше первого присваивания/объявления.
    UndefinedVariable(String),
    /// Вызов имени, для которого нет объявленной `Процедура`/`Функция` в
    /// этом модуле (методов объектов и встроенных функций пока нет).
    UndefinedFunction(String),
    DuplicateFunction(String),
    ArgumentCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// `Ф(1, , 3)` — позиция пропущена (`,,`), но у соответствующего
    /// параметра `Ф` нет значения по умолчанию: пропустить нечем.
    MissingRequiredArgument {
        name: String,
        /// Номер параметра ОТ НУЛЯ — как он лежит в списке. В тексте
        /// ошибки печатается человеческая позиция, то есть на единицу
        /// больше: пользователь считает аргументы с первого.
        index: usize,
    },
    /// Функция встроенного языка (`Строка`, `СтрДлина`, `Вычислить`, …)
    /// вызвана отдельным оператором. Платформа отказывается компилировать
    /// такой модуль — «Встроенная функция может быть использована только в
    /// выражении», — но правило касается лишь функций ЯЗЫКА: обычную
    /// функцию глобального контекста (`СтрНайти`, `ПрочитатьJSON`, …)
    /// оператором звать можно. Деление измерено полным перебором — см.
    /// [`bsl_rt::BuiltinFn::is_intrinsic`].
    BuiltinFunctionAsStatement(String),
    /// Глобальная процедура (`Сообщить`, `ЗаписатьJSON`,
    /// `ЗаполнитьЗначенияСвойств`) в позиции выражения: платформа отвечает
    /// «Обращение к процедуре как к функции».
    ProcedureAsFunction(String),
    /// `'20240230'` — литерал прошёл лексер (цифры, верная длина), но
    /// такой календарной даты не существует.
    BadDateLiteral(String),
    /// `0.333…` со scale больше `bsl_number::MAX_SCALE`: литерал прошёл
    /// лексер, но `BslNumber` его не представляет. Позиции пока нет — узел
    /// числа span не хранит; она придёт вместе с позицией у всей
    /// `SemaError` (остаток плана abi-refactor-f).
    BadNumericLiteral(String),
    /// Конструкция языка, для которой ещё нет резолвинга (коллекции,
    /// `Выполнить`/`Вычислить`, значения по умолчанию/пропуски аргументов,
    /// ... — приходят в последующих milestone'ах).
    Unsupported(&'static str),
}

/// Разрешённый динамический фрагмент и полное замыкание его компонентов.
impl std::fmt::Display for SemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemaError::UndefinedVariable(name) => {
                write!(f, "переменная «{name}» читается до присваивания")
            }
            SemaError::UndefinedFunction(name) => {
                write!(f, "нет процедуры или функции с именем «{name}»")
            }
            SemaError::DuplicateFunction(name) => {
                write!(f, "процедура или функция «{name}» объявлена дважды")
            }
            SemaError::ArgumentCountMismatch {
                name,
                expected,
                found,
            } => write!(
                f,
                "«{name}» принимает аргументов: {expected}, передано: {found}"
            ),
            SemaError::MissingRequiredArgument { name, index } => write!(
                f,
                "у «{name}» пропущен обязательный аргумент на позиции {}",
                // Насыщение, а не `+ 1`: тип публичный, и встраивающая
                // программа вправе собрать его с любым `index`. Резолвер
                // такого не породит (индексов больше, чем параметров у
                // функции, не бывает), но печать публичной ошибки обязана
                // работать при любом содержимом, а не только при
                // достижимом.
                index.saturating_add(1)
            ),
            SemaError::BuiltinFunctionAsStatement(name) => write!(
                f,
                "функция встроенного языка «{name}» не может стоять оператором"
            ),
            SemaError::ProcedureAsFunction(name) => {
                write!(f, "процедура «{name}» не возвращает значения")
            }
            SemaError::BadDateLiteral(text) => write!(f, "некорректный литерал даты «{text}»"),
            SemaError::BadNumericLiteral(text) => {
                // Литерал может быть в сотню тысяч цифр — печатаем начало,
                // а не заваливаем диагностику всем его телом.
                let preview: String = text.chars().take(24).collect();
                let tail = if text.chars().nth(24).is_some() {
                    "…"
                } else {
                    ""
                };
                write!(f, "некорректный числовой литерал «{preview}{tail}»")
            }
            SemaError::Unsupported(what) => write!(f, "не поддержано: {what}"),
        }
    }
}

impl std::error::Error for SemaError {}

pub type ResolvedSnippetWithRequirements =
    (Vec<String>, Vec<RStmt>, Vec<bsl_rt::LibraryRequirement>);

/// Сигнатура функции/процедуры, собранная до резолвинга тел — нужна, чтобы
/// вызовы разрешались независимо от порядка объявления (в том числе
/// рекурсия и взаимные вызовы).
struct FuncSig {
    index: u32,
    /// Есть ли у параметра в этой позиции значение по умолчанию — длина
    /// заодно и есть арность (по-настоящему нужен только режим передачи
    /// каждого параметра, а он читается из `ResolvedFunction::params` на
    /// этапе кодогена, не отсюда). Пропущенный аргумент (`Ф(1, , 3)`)
    /// допустим только там, где здесь `true` — иначе это ошибка резолвинга
    /// вызывающего кода, а не рантайма вызываемой функции.
    has_default: Vec<bool>,
    /// Объявлена ли `Процедура`. Вызов процедуры в позиции выражения
    /// платформа отвергает на компиляции — то же правило, что и для
    /// процедур глобального контекста (`CALL.EXPR.PROCEDURE`), но ветка
    /// разрешения другая: имя ищется в таблице объявлений модуля, а не в
    /// реестре компонентов.
    is_procedure: bool,
}

/// Сигнатура функции модуля глазами ФРАГМЕНТА `Выполнить`/`Вычислить`:
/// всё, что фрагменту нужно знать о вызываемой функции, не имея её
/// резолвнутого дерева. Вид объявления здесь не украшение: процедура в
/// позиции выражения отвергается и во фрагменте тоже (измерено, строка
/// «своя процедура выражением» в `measure-stmtcall.platform.txt`), а
/// после компиляции отличить её от функции больше не по чему — обе
/// возвращают `Неопределено`, если не сказано иначе.
pub struct SnippetSignature {
    pub name: String,
    pub arity: usize,
    pub is_procedure: bool,
    /// Есть ли у каждого параметра умолчание — в порядке параметров. Без
    /// него фрагмент подставлял заглушку «умолчаний нет» и отвергал
    /// `Вычислить("Хвост(1, )")` при том, что тот же текст в модуле
    /// компилировался (воспроизведение 4).
    pub has_default: Vec<bool>,
}

/// Резолвит весь модуль: собирает сигнатуры всех `Процедура`/`Функция` за
/// один проход (чтобы вызовы работали независимо от порядка объявления и
/// поддерживали рекурсию), затем резолвит каждое тело и операторы верхнего
/// уровня.
/// `Перем` на уровне модуля образует ОБЛАСТЬ МОДУЛЯ: процедуры и функции
/// видят такие переменные и пишут в них, и запись видна снаружи —
/// ИЗМЕРЕНО на 8.3.27. Хранилище — первые слоты кадра верхнего уровня: он
/// стоит в самом низу стека значений и живёт всё исполнение, поэтому
/// доступ из любого кадра — это прямая индексация (`Instr::GetModuleVar`).
///
/// # Errors
///
/// Возвращает [`SemaError`] при повторном объявлении функции, неизвестном имени, неверном
/// числе аргументов, недопустимом литерале даты или ещё не поддерживаемой конструкции.
pub fn resolve_program(items: &[Item]) -> Result<ResolvedProgram, SemaError> {
    resolve_program_impl(items, None)
}

/// Разрешает модуль с каталогом функций и конструкторов собранного
/// runtime. Старый [`resolve_program`] на время миграции сохраняет закрытые
/// встроенные таблицы, а этот вход открывает компонентные функции.
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же правилам, что и [`resolve_program`].
pub fn resolve_program_with_registry(
    items: &[Item],
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<ResolvedProgram, SemaError> {
    resolve_program_impl(items, Some(registry))
}

fn resolve_program_impl(
    items: &[Item],
    registry: Option<&bsl_rt::RuntimeRegistry>,
) -> Result<ResolvedProgram, SemaError> {
    let mut sigs: HashMap<String, FuncSig> = HashMap::new();
    let mut func_items: Vec<&Item> = Vec::new();
    let mut top_stmts: Vec<AStmt> = Vec::new();
    // `Перем` на уровне модуля — это ОБЛАСТЬ МОДУЛЯ, а не просто первые
    // локальные тела: измерено, что процедуры их видят.
    let mut module_vars: Vec<String> = Vec::new();

    for item in items {
        match item {
            Item::Function(f) => {
                declare_sig(
                    &mut sigs,
                    &f.name,
                    f.params.iter().map(|p| p.default.is_some()).collect(),
                    false,
                )?;
                func_items.push(item);
            }
            Item::Procedure(p) => {
                declare_sig(
                    &mut sigs,
                    &p.name,
                    p.params.iter().map(|p| p.default.is_some()).collect(),
                    true,
                )?;
                func_items.push(item);
            }
            Item::VarDecl(vd) => {
                for name in &vd.names {
                    // Свёртка через `folded_eq`, а не `eq_ignore_ascii_case`:
                    // поиск слота модульной переменной идёт по `to_uppercase`
                    // (см. `module_index` ниже), и `eq_ignore_ascii_case` на
                    // кириллице с ним расходится — разнорегистровый дубль
                    // `Перем` порождал бы второй слот, в который запись из
                    // тела и чтение из процедуры уходят порознь.
                    if !module_vars.iter().any(|n| bsl_rt::folded_eq(n, name)) {
                        module_vars.push(name.clone());
                    }
                }
                top_stmts.push(AStmt::VarDecl(vd.clone()));
            }
            Item::Stmt(s) => top_stmts.push(s.clone()),
        }
    }

    // Номера слотов модульных переменных: они же — первые слоты кадра
    // верхнего уровня (см. затравку резолвера тела ниже).
    let module_index: HashMap<String, u32> = module_vars
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    let empty_module_index: HashMap<String, u32> = HashMap::new();

    // Статически доказанные ядровые приёмники — пре-проход по AST до
    // резолвинга тел: его вердикты решают, остаётся ли вызов метода
    // открытым (см. `core_receivers` и сайт `RExpr::CallMethod` ниже).
    let by_val_modes: HashMap<String, Vec<bool>> = func_items
        .iter()
        .map(|item| match item {
            Item::Function(f) => (&f.name, &f.params),
            Item::Procedure(p) => (&p.name, &p.params),
            _ => unreachable!(),
        })
        .map(|(name, params)| {
            (
                name.to_uppercase(),
                params.iter().map(|p| p.by_val).collect(),
            )
        })
        .collect();
    let function_bodies: Vec<(&[bsl_syntax::Param], &[AStmt])> = func_items
        .iter()
        .map(|item| match item {
            Item::Function(f) => (f.params.as_slice(), f.body.as_slice()),
            Item::Procedure(p) => (p.params.as_slice(), p.body.as_slice()),
            _ => unreachable!(),
        })
        .collect();
    let core_maps = core_receivers::analyze(
        &top_stmts,
        &function_bodies,
        &module_vars,
        &by_val_modes,
        registry,
    );

    let mut functions = Vec::with_capacity(func_items.len());
    let mut used_libraries = HashSet::new();
    for (func_index, item) in func_items.iter().enumerate() {
        let (name, params, body, is_procedure) = match item {
            Item::Function(f) => (&f.name, &f.params, &f.body, false),
            Item::Procedure(p) => (&p.name, &p.params, &p.body, true),
            _ => unreachable!(),
        };
        let mut r = Resolver {
            locals: Vec::new(),
            index: HashMap::new(),
            funcs: &sigs,
            module_index: &module_index,
            registry,
            used_libraries: HashSet::new(),
            strict_stmt_calls: true,
            core_locals: Some(&core_maps.functions[func_index]),
            core_module: Some(&core_maps.module_slots),
        };
        for p in params {
            r.declare(&p.name);
        }
        let resolved_body = r.resolve_block(body)?;
        // Значения по умолчанию резолвятся ПОСЛЕ тела, но той же `r` — её
        // `locals`/`index` к этому моменту уже содержат слоты параметров
        // (объявлены выше, до `resolve_block`), так что дефолт вида
        // `Ф(б = а + 1)`, ссылающийся на предыдущий параметр, резолвится
        // корректно.
        let mut resolved_params = Vec::with_capacity(params.len());
        for p in params {
            let default = match &p.default {
                Some(e) => Some(r.resolve_expr(e)?),
                None => None,
            };
            resolved_params.push(ResolvedParam {
                by_val: p.by_val,
                default,
            });
        }
        used_libraries.extend(r.used_libraries.iter().cloned());
        functions.push(ResolvedFunction {
            name: name.clone(),
            is_procedure,
            // По телу И по умолчаниям параметров: последние компилируются в
            // тот же чанк прологом, и `Ф(а = Вычислить("..."))` обязан
            // материализовать `local_names`, даже когда тело статично.
            uses_dynamic: crate::resolved::block_uses_dynamic(&resolved_body)
                || crate::resolved::params_use_dynamic(&resolved_params),
            params: resolved_params,
            locals: r.locals,
            body: resolved_body,
        });
    }

    // Тело модуля видит те же переменные как ОБЫЧНЫЕ локальные: его кадр и
    // есть их хранилище, и номер слота совпадает с номером в `module_index`
    // — на этом совпадении держится доступ из функций.
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &sigs,
        module_index: &empty_module_index,
        registry,
        used_libraries: HashSet::new(),
        strict_stmt_calls: true,
        core_locals: Some(&core_maps.top),
        core_module: None,
    };
    for name in &module_vars {
        r.declare(name);
    }
    let top_body = r.resolve_block(&top_stmts)?;
    used_libraries.extend(r.used_libraries.iter().cloned());
    let top_level = Resolved {
        uses_dynamic: crate::resolved::block_uses_dynamic(&top_body),
        locals: r.locals,
        body: top_body,
    };

    Ok(ResolvedProgram {
        requirements: match registry {
            Some(registry) => registry.requirements_for(used_libraries),
            None => vec![bsl_rt::LibraryRequirement::bsl_rt()],
        },
        functions,
        top_level,
        module_vars,
    })
}

fn declare_sig(
    sigs: &mut HashMap<String, FuncSig>,
    name: &str,
    has_default: Vec<bool>,
    is_procedure: bool,
) -> Result<(), SemaError> {
    let key = name.to_uppercase();
    if sigs.contains_key(&key) {
        return Err(SemaError::DuplicateFunction(name.to_string()));
    }
    let index = sigs.len() as u32;
    sigs.insert(
        key,
        FuncSig {
            index,
            has_default,
            is_procedure,
        },
    );
    Ok(())
}

/// Разрешает имена в плоском скрипте верхнего уровня без объявлений функций —
/// удобно для тестов. Функции разрешаются только через [`resolve_program`].
///
/// # Errors
///
/// Возвращает [`SemaError`] при неизвестном имени, неверном числе аргументов, недопустимом
/// литерале даты или ещё не поддерживаемой конструкции.
pub fn resolve_script(stmts: &[AStmt]) -> Result<Resolved, SemaError> {
    let empty_funcs = HashMap::new();
    let empty_module = HashMap::new();
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &empty_funcs,
        module_index: &empty_module,
        registry: None,
        used_libraries: HashSet::new(),
        strict_stmt_calls: true,
        core_locals: None,
        core_module: None,
    };
    let body = r.resolve_block(stmts)?;
    Ok(Resolved {
        uses_dynamic: crate::resolved::block_uses_dynamic(&body),
        locals: r.locals,
        body,
    })
}

/// Разрешает имена во фрагменте кода для `Выполнить`/`Вычислить`: `existing_locals` —
/// уже объявленные переменные окружающего скрипта, которые первыми добавляются в таблицу имён
/// первыми, поэтому ссылки на них в фрагменте попадают на ТЕ ЖЕ слоты, а
/// не заводят копию. Новые имена, объявленные внутри фрагмента, получают
/// слоты ПОСЛЕ существующих — полный список возвращается вызывающему,
/// который сам решает, сохранять ли их (VM для `Выполнить`/`Вычислить`
/// внутри уже скомпилированного кода их не сохраняет: она не может расширять
/// статически размеченный кадр; REPL сохраняет, поскольку его
/// кадр и так растёт от строки к строке).
///
/// ИЗМЕРЕНО на 8.3.27: фрагмент ВИДИТ процедуры и функции модуля —
/// `Вычислить("Удвоить(21)")` возвращает 42. Поэтому `signatures` тянется
/// сюда из уже скомпилированной программы — [`SnippetSignature`] на
/// каждую, в том же порядке, в каком функции лежат в
/// `Program::chunks[1..]`. Пустой список означает «функций нет» (так зовёт
/// REPL до первого объявления), а не «вызывать нельзя».
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что и [`resolve_script`], а также если фрагмент
/// вызывает функцию, которой нет в `signatures`.
pub fn resolve_snippet_stmts(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[SnippetSignature],
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    resolve_snippet_stmts_mode(existing_locals, module_vars, stmts, signatures, true)
}

/// То же, что [`resolve_snippet_stmts`], но для строки REPL: голый вызов
/// функции встроенного языка (`СтрДлина("аб")`) не отвергается — REPL
/// печатает его значение. Платформенное правило про оператор относится к
/// компиляции модуля, а строка REPL модулем не является; фрагменты
/// `Выполнить` остаются строгими, как на платформе (ИЗМЕРЕНО, якорь
/// `CALL.STMT.INTRINSIC`).
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что и
/// [`resolve_snippet_stmts`], кроме [`SemaError::BuiltinFunctionAsStatement`].
pub fn resolve_repl_stmts(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[SnippetSignature],
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    resolve_snippet_stmts_mode(existing_locals, module_vars, stmts, signatures, false)
}

/// Разрешает динамический фрагмент с тем же каталогом компонентов, что и
/// основную программу, и возвращает его собственное замыкание требований.
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что
/// [`resolve_snippet_stmts`].
pub fn resolve_snippet_stmts_with_registry(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[SnippetSignature],
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<ResolvedSnippetWithRequirements, SemaError> {
    resolve_snippet_stmts_mode_registry(
        existing_locals,
        module_vars,
        stmts,
        signatures,
        true,
        Some(registry),
    )
}

/// То же, что [`resolve_snippet_stmts_with_registry`], но по правилам
/// REPL: голый вызов функции встроенного языка не отвергается.
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что и
/// [`resolve_repl_stmts`].
pub fn resolve_repl_stmts_with_registry(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[SnippetSignature],
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<ResolvedSnippetWithRequirements, SemaError> {
    resolve_snippet_stmts_mode_registry(
        existing_locals,
        module_vars,
        stmts,
        signatures,
        false,
        Some(registry),
    )
}

fn resolve_snippet_stmts_mode(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[SnippetSignature],
    strict_stmt_calls: bool,
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    let (locals, body, _) = resolve_snippet_stmts_mode_registry(
        existing_locals,
        module_vars,
        stmts,
        signatures,
        strict_stmt_calls,
        None,
    )?;
    Ok((locals, body))
}

fn resolve_snippet_stmts_mode_registry(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[SnippetSignature],
    strict_stmt_calls: bool,
    registry: Option<&bsl_rt::RuntimeRegistry>,
) -> Result<ResolvedSnippetWithRequirements, SemaError> {
    let empty_funcs: HashMap<String, FuncSig> = signatures
        .iter()
        .enumerate()
        .map(|(index, sig)| {
            (
                sig.name.to_uppercase(),
                FuncSig {
                    index: index as u32,
                    // Реальные умолчания вызываемой функции модуля, а не
                    // заглушка: фрагмент отличает опущенный хвостовой
                    // необязательный аргумент от пропущенного обязательного
                    // так же, как статический резолвер (воспроизведение 4).
                    has_default: sig.has_default.clone(),
                    is_procedure: sig.is_procedure,
                },
            )
        })
        .collect();
    let index = existing_locals
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    // Фрагмент ВИДИТ переменные уровня модуля и пишет в них — ИЗМЕРЕНО на
    // 8.3.27. Свой стек этому не мешает: модульные значения едут во
    // фрагмент отдельным блоком, а `Program::module_base` говорит VM, с
    // какого места он там лежит (см. `run_dynamic_snippet`).
    //
    // Имена из `existing_locals` по-прежнему выигрывают: `index` заполнен
    // до `module_index`, а резолвер смотрит сначала в него. На верхнем
    // уровне это важно — там модульные переменные И ЕСТЬ локальные кадра,
    // и разрешать их как модульные значило бы писать в блок-копию.
    let module_index: HashMap<String, u32> = module_vars
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    let mut r = Resolver {
        locals: existing_locals.to_vec(),
        index,
        funcs: &empty_funcs,
        module_index: &module_index,
        registry,
        used_libraries: HashSet::new(),
        strict_stmt_calls,
        core_locals: None,
        core_module: None,
    };
    let body = r.resolve_block(stmts)?;
    let requirements = match registry {
        Some(registry) => registry.requirements_for(r.used_libraries.iter().cloned()),
        None => vec![bsl_rt::LibraryRequirement::bsl_rt()],
    };
    Ok((r.locals, body, requirements))
}

/// Типы БАЗОВОГО рантайма, которые умеет строить `Новый`, — в каноническом
/// написании, оба языка. Список нужен снаружи (автодополнение REPL
/// добавляет к нему конструкторы реестра компонентов), а `resolve_new`
/// разбирает каждый по-своему: у них разная арность и разный смысл
/// аргументов, одной таблицей не обойтись. Типы вынесенных компонентов
/// сюда не входят — их имена объявляют `ConstructorDescriptor` компонентов.
/// Что список не разъедется с `match`, проверяет
/// `every_new_type_is_recognised_by_resolve_new`.
pub const NEW_TYPES: &[&str] = &[
    "Массив",
    "Array",
    "Структура",
    "Structure",
    "Соответствие",
    "Map",
    "ТаблицаЗначений",
    "ValueTable",
    "ОписаниеТипов",
    "TypeDescription",
    "СравнениеЗначений",
    "ValueComparison",
    "ЗаписьТекста",
    "TextWriter",
    // Английское написание проверено пробой `BIN.NEW.EN` на платформе:
    // `Новый BinaryData(Путь)` принимается.
    "ДвоичныеДанные",
    "BinaryData",
    "УникальныйИдентификатор",
    "UUID",
];

struct Resolver<'a> {
    locals: Vec<String>,
    /// Ключ — имя в верхнем регистре: доступ к переменным регистронезависим.
    index: HashMap<String, u32>,
    funcs: &'a HashMap<String, FuncSig>,
    /// Переменные уровня модуля: имя -> номер слота в кадре верхнего
    /// уровня. У резолвера ТЕЛА модуля пуста (там те же переменные уже
    /// локальные), у резолвера каждой функции — заполнена.
    ///
    /// Локальное имя ЗАТЕНЯЕТ модульное — ИЗМЕРЕНО на 8.3.27: функция с
    /// явной `Перем` того же имени работает со своей копией, а модульная
    /// после вызова цела.
    module_index: &'a HashMap<String, u32>,
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    used_libraries: HashSet<bsl_rt::LibraryKey>,
    /// Отвергать ли функции встроенного языка в позиции оператора. В
    /// модулях и фрагментах `Выполнить` — да, как на платформе; в REPL —
    /// нет: голый вызов `СтрДлина("аб")` там печатает значение, и лишать
    /// REPL этого ради правила о компиляции модулей незачем.
    ///
    /// Остаётся булевым сознательно: у него ровно два очевидных состояния,
    /// одно место чтения и объяснение прямо здесь. Имя-перечисление на
    /// месте этого поля не убрало бы ни ветвления, ни комментария —
    /// значит, добавило бы только тип.
    strict_stmt_calls: bool,
    /// Статически доказанные ядровые приёмники этой области (см.
    /// `core_receivers`): локальные — по имени в верхнем регистре. `None`
    /// у REPL и фрагментов `Выполнить` — их слоты живут дольше одного
    /// разрешения, и доказательств там нет.
    core_locals: Option<&'a HashMap<String, CoreReceiver>>,
    /// Модульные переменные — по номеру слота (для `RExpr::ModuleVar` из
    /// тел функций).
    core_module: Option<&'a HashMap<u32, CoreReceiver>>,
}

impl<'a> Resolver<'a> {
    fn declare(&mut self, name: &str) -> u32 {
        let key = name.to_uppercase();
        if let Some(&slot) = self.index.get(&key) {
            return slot;
        }
        let slot = self.locals.len() as u32;
        self.locals.push(name.to_string());
        self.index.insert(key, slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<u32> {
        self.index.get(&name.to_uppercase()).copied()
    }

    /// Статически доказанный ядровый тип приёмника: переменная с
    /// вердиктом пре-прохода (см. `core_receivers`) или сам ядровый
    /// конструктор на месте вызова.
    fn core_receiver_of(&self, obj: &RExpr) -> Option<CoreReceiver> {
        match obj {
            RExpr::Local(slot) => {
                let name = self.locals.get(*slot as usize)?;
                self.core_locals?.get(&name.to_uppercase()).copied()
            }
            RExpr::ModuleVar(slot) => self.core_module?.get(slot).copied(),
            RExpr::NewTextWriter { .. } => Some(CoreReceiver::TextWriter),
            RExpr::NewArray { .. }
            | RExpr::NewStructure { .. }
            | RExpr::NewTable
            | RExpr::NewMap
            | RExpr::NewTypeDescription(_)
            | RExpr::NewValueComparison
            | RExpr::NewBinaryData { .. }
            | RExpr::NewUuid { .. } => Some(CoreReceiver::Other),
            _ => None,
        }
    }

    fn resolve_block(&mut self, stmts: &[AStmt]) -> Result<Vec<RStmt>, SemaError> {
        let mut out = Vec::new();
        for s in stmts {
            if let Some(rs) = self.resolve_stmt(s)? {
                out.push(rs);
            }
        }
        Ok(out)
    }

    fn resolve_stmt(&mut self, s: &AStmt) -> Result<Option<RStmt>, SemaError> {
        match s {
            AStmt::Assign { target, value } => match target {
                LValue::Name(name) => {
                    // Уже известное локальное имя — обычная запись. Иначе
                    // ищем модульное: присваивание модульной переменной
                    // ОБЯЗАНО писать в неё, а не заводить локальную копию,
                    // иначе результат не увидит ни тело модуля, ни соседняя
                    // функция.
                    if let Some(slot) = self.lookup(name) {
                        let value = self.resolve_expr(value)?;
                        return Ok(Some(RStmt::AssignLocal { slot, value }));
                    }
                    if let Some(&slot) = self.module_index.get(&name.to_uppercase()) {
                        let value = self.resolve_expr(value)?;
                        return Ok(Some(RStmt::AssignModuleVar { slot, value }));
                    }
                    let slot = self.declare(name);
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignLocal { slot, value }))
                }
                LValue::Index { obj, index } => {
                    let obj = self.resolve_expr(obj)?;
                    let index = self.resolve_expr(index)?;
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignIndex { obj, index, value }))
                }
                LValue::Field { obj, name } => {
                    let obj = self.resolve_expr(obj)?;
                    let value = self.resolve_expr(value)?;
                    // Запись свойства всегда компилируется в закрытый
                    // `SetProp` — обоснование у чтения поля в
                    // `resolve_expr`.
                    Ok(Some(RStmt::AssignField {
                        obj,
                        name: name.clone(),
                        value,
                    }))
                }
            },
            AStmt::ExprStmt(e) => {
                // Позиция оператора: единственное место, где легальна
                // глобальная процедура, — и запретное для функций
                // встроенного языка. Проверка второго — по РЕЗУЛЬТАТУ
                // разрешения, а не по имени: пользовательская функция,
                // затеняющая встроенное имя, разрешится в `RExpr::Call` и
                // под правило не попадёт.
                let r = self.resolve_expr_at(e, CallPosition::Statement)?;
                let forbidden = match &r {
                    RExpr::CallBuiltinFn { builtin, .. } => builtin.is_intrinsic(),
                    RExpr::CallComponent { kind, .. } => *kind == bsl_rt::FunctionKind::Intrinsic,
                    RExpr::DynEval(_) => true,
                    _ => false,
                };
                if self.strict_stmt_calls && forbidden {
                    // Обе запретные формы — вызовы по имени-идентификатору;
                    // другие выражения сюда не приводят.
                    let shown = match e {
                        AExpr::Call { callee, .. } => match callee.as_ref() {
                            AExpr::Ident(n) => n.clone(),
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    return Err(SemaError::BuiltinFunctionAsStatement(shown));
                }
                Ok(Some(RStmt::ExprStmt(r)))
            }
            AStmt::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let cond = self.resolve_expr(cond)?;
                let then_branch = self.resolve_block(then_branch)?;
                let mut elsifs = Vec::new();
                for (c, b) in elsif_branches {
                    elsifs.push((self.resolve_expr(c)?, self.resolve_block(b)?));
                }
                let else_branch = match else_branch {
                    Some(b) => Some(self.resolve_block(b)?),
                    None => None,
                };
                Ok(Some(RStmt::If {
                    cond,
                    then_branch,
                    elsif_branches: elsifs,
                    else_branch,
                }))
            }
            AStmt::While { cond, body } => {
                let cond = self.resolve_expr(cond)?;
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::While { cond, body }))
            }
            AStmt::ForNumeric {
                var,
                from,
                to,
                body,
            } => {
                let from = self.resolve_expr(from)?;
                let to = self.resolve_expr(to)?;
                // Переменная цикла объявляется до тела: тело может ссылаться
                // на неё, а сама она остаётся живой и после `КонецЦикла`.
                let slot = self.declare(var);
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::ForNumeric {
                    slot,
                    from,
                    to,
                    body,
                }))
            }
            AStmt::ForEach { var, iter, body } => {
                let iter = self.resolve_expr(iter)?;
                // Как и в ForNumeric: переменная объявляется до тела, живёт
                // и после КонецЦикла.
                let slot = self.declare(var);
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::ForEach { slot, iter, body }))
            }
            AStmt::Break => Ok(Some(RStmt::Break)),
            AStmt::Continue => Ok(Some(RStmt::Continue)),
            AStmt::Return(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Return(r)))
            }
            AStmt::Try { body, except_body } => {
                let body = self.resolve_block(body)?;
                let except_body = self.resolve_block(except_body)?;
                Ok(Some(RStmt::Try { body, except_body }))
            }
            AStmt::Raise(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Raise(r)))
            }
            AStmt::VarDecl(vd) => {
                // Только регистрирует слоты; значение по умолчанию —
                // Неопределено, для этого не нужна ни одна инструкция VM
                // (регистры кадра и так инициализируются Неопределено).
                for name in &vd.names {
                    self.declare(name);
                }
                Ok(None)
            }
            AStmt::Execute(e) => Ok(Some(RStmt::Execute(self.resolve_expr(e)?))),
        }
    }

    /// Разрешает выражение в позиции ВЫРАЖЕНИЯ.
    ///
    /// Вложенные части — аргументы, индексы, объекты цепочек — разрешаются
    /// только этим входом, поэтому процедура в аргументе
    /// (`Сообщить(Сообщить(1))`) не проскочит: позиция оператора туда
    /// физически не передаётся. Раньше её нёс изменяемый флаг, который
    /// ветка `ExprStmt` взводила, а вход сюда гасил `mem::take`; правило
    /// держалось на том, что гашение не забудут.
    fn resolve_expr(&mut self, e: &AExpr) -> Result<RExpr, SemaError> {
        self.resolve_expr_at(e, CallPosition::Expression)
    }

    /// То же с явной позицией. Отдельный вход, а не параметр у
    /// `resolve_expr`: позиция значима ровно для ОДНОГО узла — верхнего
    /// вызова оператора, — и её незачем протаскивать через полсотни
    /// вызовов, которым она всегда `Expression`.
    fn resolve_expr_at(&mut self, e: &AExpr, position: CallPosition) -> Result<RExpr, SemaError> {
        match e {
            AExpr::Number(text) => {
                // Лексер пропускает и `0.` со scale больше
                // `bsl_number::MAX_SCALE` (сотня тысяч дробных цифр) — такой
                // литерал `BslNumber` не представляет. `Engine::compile`
                // объявлена возвращающей `Result` и обязана им быть, а не
                // ронять процесс.
                let n = BslNumber::parse_canonical(text)
                    .map_err(|_| SemaError::BadNumericLiteral(text.clone()))?;
                Ok(RExpr::Number(n))
            }
            AExpr::Bool(b) => Ok(RExpr::Bool(*b)),
            AExpr::Undefined => Ok(RExpr::Undefined),
            AExpr::Null => Ok(RExpr::Null),
            AExpr::Ident(name) => match self.lookup(name) {
                Some(slot) => Ok(RExpr::Local(slot)),
                None => match self.module_index.get(&name.to_uppercase()) {
                    Some(&slot) => Ok(RExpr::ModuleVar(slot)),
                    // `АргументыКоманднойСтроки` пишется и без скобок — как
                    // свойство глобального контекста в OneScript, откуда
                    // это расширение и взято. Переменная с тем же именем
                    // побеждает: проверки выше.
                    None if bsl_rt::BuiltinFn::lookup(name)
                        == Some(bsl_rt::BuiltinFn::CommandLineArguments) =>
                    {
                        Ok(RExpr::CallBuiltinFn {
                            builtin: bsl_rt::BuiltinFn::CommandLineArguments,
                            args: Vec::new(),
                        })
                    }
                    // `ФабрикаXDTO` у платформы — свойство глобального
                    // контекста (фабрика КОНФИГУРАЦИИ), поэтому пишется без
                    // скобок и разрешается тем же приёмом, что и
                    // `АргументыКоманднойСтроки`: голое имя становится
                    // вызовом функции компонента с нулём аргументов
                    // (`bsl_xml::configuration_factory`, всегда ловимая
                    // ошибка — метаданных конфигурации здесь нет).
                    // Переменная с тем же именем побеждает: проверки выше.
                    None if matches!(
                        name.to_uppercase().as_str(),
                        "ФАБРИКАXDTO" | "XDTOFACTORY"
                    ) =>
                    {
                        let Some((library_index, function)) = self
                            .registry
                            .and_then(|registry| registry.lookup_function(name))
                        else {
                            return Err(SemaError::Unsupported(
                                "ФабрикаXDTO требует зарегистрированный компонент bsl-xml",
                            ));
                        };
                        let registry = self.registry.expect("lookup выше нашёл функцию");
                        let descriptor = registry
                            .function(library_index, function)
                            .expect("индекс получен из таблицы имён этого реестра");
                        let package = registry
                            .library(library_index)
                            .expect("индекс получен из таблицы имён этого реестра")
                            .package();
                        let library = bsl_rt::LibraryKey::new(package);
                        self.used_libraries.insert(library.clone());
                        Ok(RExpr::CallComponent {
                            library,
                            function,
                            kind: descriptor.kind,
                            args: Vec::new(),
                        })
                    }
                    // Голое имя СИСТЕМНОГО перечисления (без `.Член`) — тоже
                    // валидное выражение, а не обращение к неопределённой
                    // переменной. ИЗМЕРЕНО на `ВариантЗаписиДатыJSON`
                    // (`JSON.DATE_VARIANT_EN_NAMES`, «Т+»:
                    // `Вычислить("JSONDateWritingVariant")` платформа
                    // вычисляет, а не отвергает). Переменная/модульная с тем
                    // же именем побеждает — проверки выше. Что именно
                    // возвращают `Строка()`/`ТипЗнч()` такого значения, не
                    // измерено (см. `bsl_rt::BslValue::EnumType`,
                    // `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`).
                    None if bsl_rt::lookup_enum(name).is_some() => Ok(RExpr::EnumTypeRef(
                        bsl_rt::lookup_enum(name).expect("проверено guard'ом выше"),
                    )),
                    // Голое имя менеджера `ФайловыеПотоки` разрешается так
                    // же, как голое имя перечисления, — но НЕ в константу:
                    // измерено, что `ФайловыеПотоки = ФайловыеПотоки` —
                    // «Нет», значит каждое обращение строит новый объект.
                    None if matches!(
                        name.to_uppercase().as_str(),
                        "ФАЙЛОВЫЕПОТОКИ" | "FILESTREAMS"
                    ) =>
                    {
                        let Some((library_index, constructor)) = self
                            .registry
                            .and_then(|registry| registry.lookup_constructor(name))
                        else {
                            return Err(SemaError::Unsupported(
                                "ФайловыеПотоки требует зарегистрированный компонент bsl-stream",
                            ));
                        };
                        let registry = self.registry.expect("lookup выше нашёл конструктор");
                        let package = registry
                            .library(library_index)
                            .expect("индекс получен из таблицы имён этого реестра")
                            .package();
                        let library = bsl_rt::LibraryKey::new(package);
                        self.used_libraries.insert(library.clone());
                        Ok(RExpr::CreateObject {
                            library,
                            constructor,
                            args: Vec::new(),
                        })
                    }
                    None => Err(SemaError::UndefinedVariable(name.clone())),
                },
            },
            AExpr::Unary { op, expr } => Ok(RExpr::Unary {
                op: *op,
                expr: Box::new(self.resolve_expr(expr)?),
            }),
            AExpr::Binary { op, lhs, rhs } => Ok(RExpr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_expr(lhs)?),
                rhs: Box::new(self.resolve_expr(rhs)?),
            }),
            AExpr::Call { callee, args } => self.resolve_call(callee, args, position),
            AExpr::Str(s) => Ok(RExpr::Str(s.clone())),
            // Литерал `'ГГГГММДД'`/`'ГГГГММДДЧЧММСС'`. Лексер уже проверил,
            // что внутри только цифры и их 8 или 14, — но НЕ проверил, что
            // получившаяся дата существует (`'20240230'` пройдёт лексер),
            // поэтому разбор может провалиться и здесь, и это ошибка
            // резолвинга, а не рантайма: литерал известен на этапе
            // компиляции, падать на нём во время исполнения незачем.
            AExpr::Date(digits) => bsl_rt::BslDate::parse_digits(digits)
                .map(RExpr::Date)
                .ok_or(SemaError::BadDateLiteral(digits.clone())),
            AExpr::Index { obj, index } => Ok(RExpr::Index {
                obj: Box::new(self.resolve_expr(obj)?),
                index: Box::new(self.resolve_expr(index)?),
            }),
            AExpr::Field { obj, name } => {
                // `ТипЗначенияJSON.ИмяСвойства` — не чтение поля объекта, а
                // КОНСТАНТА: у платформы перечисление тоже не объект, и
                // несуществующий член она ловит на компиляции, а не в
                // рантайме (проверено `Вычислить`). Проверка идёт до
                // резолвинга левой части — иначе имя перечисления сначала
                // не нашлось бы среди переменных.
                if let AExpr::Ident(base) = obj.as_ref()
                    && let Some(kind) = bsl_rt::lookup_enum(base)
                {
                    let member = bsl_rt::lookup_member(kind, name)
                        .ok_or_else(|| SemaError::UndefinedVariable(format!("{base}.{name}")))?;
                    return Ok(RExpr::EnumMember(member));
                }
                // Доступ к свойству всегда компилируется в закрытый
                // `GetProp`: открытый двойник не несёт информации сверх
                // закрытого (то же имя в той же таблице `.names`, только
                // усечённое до u16), а тела совпадают на всех трёх
                // исполнителях — в интерпретаторе, в JIT-шимах и в
                // эффектах бандлов. Для компонентного получателя оба
                // варианта идут одним строковым `get_property`, а
                // нативный в закрытом варианте остаётся в горячем цикле
                // диспетчера — с реестром это измеренные десятки
                // процентов на сценариях с плотным доступом к полям
                // (`csv_write`). Открытые `GetObjectProp`/`SetObjectProp`
                // остаются в формате байт-кода ради уже сериализованных
                // программ.
                Ok(RExpr::Field {
                    obj: Box::new(self.resolve_expr(obj)?),
                    name: name.clone(),
                })
            }
            AExpr::New { type_name, args } => self.resolve_new(type_name, args),
            AExpr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => Ok(RExpr::Ternary {
                cond: Box::new(self.resolve_expr(cond)?),
                then_expr: Box::new(self.resolve_expr(then_expr)?),
                else_expr: Box::new(self.resolve_expr(else_expr)?),
            }),
        }
    }

    /// Разбирает известные платформенные типы из [`NEW_TYPES`]. Общие
    /// пользовательские типы пока не поддержаны.
    fn resolve_new(&mut self, type_name: &str, args: &[AExpr]) -> Result<RExpr, SemaError> {
        if let Some(registry) = self.registry
            && let Some((library_index, constructor)) = registry.lookup_constructor(type_name)
        {
            let descriptor = registry
                .constructor(library_index, constructor)
                .expect("индекс получен из таблицы имён этого реестра");
            let found: u8 =
                args.len()
                    .try_into()
                    .map_err(|_| SemaError::ArgumentCountMismatch {
                        name: format!("Новый {type_name}"),
                        expected: descriptor.arity.max() as usize,
                        found: args.len(),
                    })?;
            if !descriptor.arity.accepts(found) {
                return Err(SemaError::ArgumentCountMismatch {
                    name: format!("Новый {type_name}"),
                    expected: descriptor.arity.max() as usize,
                    found: args.len(),
                });
            }
            let mut resolved_args = Vec::with_capacity(args.len());
            for argument in args {
                resolved_args.push(self.resolve_expr(argument)?);
            }
            let package = registry
                .library(library_index)
                .expect("индекс получен из таблицы имён этого реестра")
                .package();
            let library = bsl_rt::LibraryKey::new(package);
            self.used_libraries.insert(library.clone());
            return Ok(RExpr::CreateObject {
                library,
                constructor,
                args: resolved_args,
            });
        }
        // Имена типов вынесенных компонентов. Конструирует их только реестр
        // (ветка выше); здесь остаётся внятный отказ — и когда компонент не
        // зарегистрирован, и когда конвейер собран вовсе без реестра.
        let upper = type_name.to_uppercase();
        if matches!(upper.as_str(), "ТЕКСТОВЫЙДОКУМЕНТ" | "TEXTDOCUMENT") {
            return Err(SemaError::Unsupported(
                "ТекстовыйДокумент требует зарегистрированный компонент bsl-textdoc",
            ));
        }
        if matches!(upper.as_str(), "БУФЕРДВОИЧНЫХДАННЫХ" | "BINARYDATABUFFER") {
            return Err(SemaError::Unsupported(
                "БуферДвоичныхДанных требует зарегистрированный компонент bsl-binbuf",
            ));
        }
        if matches!(
            upper.as_str(),
            "ПОТОКВПАМЯТИ"
                | "MEMORYSTREAM"
                | "ФАЙЛОВЫЙПОТОК"
                | "FILESTREAM"
                | "ЧТЕНИЕДАННЫХ"
                | "DATAREADER"
                | "ЗАПИСЬДАННЫХ"
                | "DATAWRITER"
        ) {
            return Err(SemaError::Unsupported(
                "потоковый тип требует зарегистрированный компонент bsl-stream",
            ));
        }
        if matches!(upper.as_str(), "ТАБЛИЧНЫЙДОКУМЕНТ" | "SPREADSHEETDOCUMENT") {
            return Err(SemaError::Unsupported(
                "ТабличныйДокумент требует зарегистрированный компонент bsl-spreadsheet",
            ));
        }
        if matches!(
            upper.as_str(),
            "ЧТЕНИЕXML"
                | "XMLREADER"
                | "ЗАПИСЬXML"
                | "XMLWRITER"
                | "ПАРАМЕТРЫЗАПИСИXML"
                | "XMLWRITERSETTINGS"
        ) {
            return Err(SemaError::Unsupported(
                "тип XML требует зарегистрированный компонент bsl-xml",
            ));
        }
        if matches!(
            upper.as_str(),
            "ПОСТРОИТЕЛЬDOM"
                | "DOMBUILDER"
                | "ДОКУМЕНТDOM"
                | "DOMDOCUMENT"
                | "ЗАПИСЬDOM"
                | "DOMWRITER"
                | "РАЗЫМЕНОВАТЕЛЬПРОСТРАНСТВИМЕНDOM"
                | "DOMNAMESPACERESOLVER"
        ) {
            return Err(SemaError::Unsupported(
                "тип DOM требует зарегистрированный компонент bsl-xml",
            ));
        }
        if matches!(
            upper.as_str(),
            "ПОСТРОИТЕЛЬСХЕМXML"
                | "XMLSCHEMABUILDER"
                | "СХЕМАXML"
                | "XMLSCHEMA"
                | "НАБОРСХЕМXML"
                | "XMLSCHEMASET"
                | "РАСШИРЕННОЕИМЯXML"
        ) {
            return Err(SemaError::Unsupported(
                "тип модели схемы требует зарегистрированный компонент bsl-xml",
            ));
        }
        if matches!(
            upper.as_str(),
            "ФАБРИКАXDTO" | "XDTOFACTORY" | "СЕРИАЛИЗАТОРXDTO" | "XDTOSERIALIZER"
        ) {
            return Err(SemaError::Unsupported(
                "тип XDTO требует зарегистрированный компонент bsl-xml",
            ));
        }
        if matches!(
            upper.as_str(),
            "ДОКУМЕНТPDF" | "PDFDOCUMENT" | "КОЛЛЕКЦИЯВЛОЖЕНИЙPDF" | "PDFATTACHMENTCOLLECTION"
        ) {
            return Err(SemaError::Unsupported(
                "тип PDF требует зарегистрированный компонент bsl-pdf",
            ));
        }
        if matches!(
            upper.as_str(),
            "ЧТЕНИЕZIPФАЙЛА"
                | "ZIPFILEREADER"
                | "ЧТЕНИЕФАЙЛААРХИВА"
                | "ARCHIVEFILEREADER"
                | "ЗАПИСЬZIPФАЙЛА"
                | "ZIPFILEWRITER"
                | "ЗАПИСЬФАЙЛААРХИВА"
                | "ARCHIVEFILEWRITER"
        ) {
            return Err(SemaError::Unsupported(
                "тип архива требует зарегистрированный компонент bsl-zip",
            ));
        }
        if matches!(
            upper.as_str(),
            "ЧТЕНИЕJSON"
                | "JSONREADER"
                | "ЗАПИСЬJSON"
                | "JSONWRITER"
                | "ПАРАМЕТРЫЗАПИСИJSON"
                | "JSONWRITERSETTINGS"
                | "НАСТРОЙКИСЕРИАЛИЗАЦИИJSON"
                | "JSONSERIALIZERSETTINGS"
        ) {
            return Err(SemaError::Unsupported(
                "JSON-тип требует зарегистрированный компонент bsl-json",
            ));
        }
        match type_name.to_uppercase().as_str() {
            "МАССИВ" | "ARRAY" => {
                let mut dims = Vec::with_capacity(args.len());
                for a in args {
                    dims.push(self.resolve_expr(a)?);
                }
                Ok(RExpr::NewArray { dims })
            }
            "СТРУКТУРА" | "STRUCTURE" => {
                if args.is_empty() {
                    return Ok(RExpr::NewStructure {
                        keys: Vec::new(),
                        values: Vec::new(),
                    });
                }
                // ВНИМАНИЕ: список ключей обязан быть строковым ЛИТЕРАЛОМ —
                // именно это делает число форм структур конечным и
                // известным на этапе компиляции (`ShapeTable::intern`
                // заводит их с `depth = 0`, то есть с полным запасом
                // переходов). Когда сюда добавят вычисляемую строку
                // (`Новый Структура(КлючиИзПеременной)`), интернировать её
                // ключи в таблицу форм НЕЛЬЗЯ: цикл с разными ключами
                // заводил бы бессмертную форму на каждой итерации, а
                // деградация по `MAX_SHAPE_TRANSITIONS` тут не спасает —
                // каждая такая форма создаётся с нулевой глубиной. Такой
                // конструктор должен сразу строить структуру в словарном
                // режиме (`bsl_rt::StructureStorage::Dictionary`), минуя
                // интернирование.
                let key_text = match &args[0] {
                    AExpr::Str(s) => s,
                    _ => {
                        return Err(SemaError::Unsupported(
                            "Новый Структура(...) со списком полей не строковым литералом появится позже",
                        ));
                    }
                };
                let keys: Vec<String> = key_text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let rest = &args[1..];
                let values = if rest.is_empty() {
                    keys.iter().map(|_| RExpr::Undefined).collect()
                } else {
                    if rest.len() != keys.len() {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: "Новый Структура".to_string(),
                            expected: keys.len(),
                            found: rest.len(),
                        });
                    }
                    let mut vs = Vec::with_capacity(rest.len());
                    for a in rest {
                        vs.push(self.resolve_expr(a)?);
                    }
                    vs
                };
                Ok(RExpr::NewStructure { keys, values })
            }
            "ТАБЛИЦАЗНАЧЕНИЙ" | "VALUETABLE" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ТаблицаЗначений".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTable)
            }
            "ОПИСАНИЕТИПОВ" | "TYPEDESCRIPTION" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ОписаниеТипов".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTypeDescription(Box::new(
                    self.resolve_expr(&args[0])?,
                )))
            }
            "СРАВНЕНИЕЗНАЧЕНИЙ" | "VALUECOMPARISON" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый СравнениеЗначений".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewValueComparison)
            }
            "СООТВЕТСТВИЕ" | "MAP" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый Соответствие".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewMap)
            }
            "ЗАПИСЬТЕКСТА" | "TEXTWRITER" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЗаписьТекста".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTextWriter {
                    path: Box::new(self.resolve_expr(&args[0])?),
                })
            }
            // Ровно один аргумент — имя файла. Ни пустой конструктор, ни
            // два аргумента платформа не принимает (пробы `BIN.NEW.NOARG`,
            // `BIN.NEW.TWOARGS`), поэтому арность проверяется здесь, а не
            // в рантайме.
            "ДВОИЧНЫЕДАННЫЕ" | "BINARYDATA" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ДвоичныеДанные".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewBinaryData {
                    path: Box::new(self.resolve_expr(&args[0])?),
                })
            }
            // Размер обязателен, порядок байтов необязателен, третьего
            "УНИКАЛЬНЫЙИДЕНТИФИКАТОР" | "UUID" => {
                if args.len() > 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый УникальныйИдентификатор".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let arg = match args.first() {
                    Some(a) => self.resolve_expr(a)?,
                    None => RExpr::Undefined,
                };
                Ok(RExpr::NewUuid { arg: Box::new(arg) })
            }
            _ => Err(SemaError::Unsupported(
                "этот тип в выражении Новый пока не поддержан",
            )),
        }
    }

    fn resolve_call(
        &mut self,
        callee: &AExpr,
        args: &[Option<AExpr>],
        position: CallPosition,
    ) -> Result<RExpr, SemaError> {
        match callee {
            AExpr::Ident(name) => {
                if let Some((index, has_default, is_procedure)) = self
                    .funcs
                    .get(&name.to_uppercase())
                    .map(|s| (s.index, s.has_default.clone(), s.is_procedure))
                {
                    // Своя процедура подчиняется тому же правилу, что и
                    // процедура глобального контекста, — измерено на
                    // 8.3.27 (строка «своя процедура выражением» в
                    // `measure-stmtcall.platform.txt`: отказ, при том что
                    // строкой выше та же процедура ОПЕРАТОРОМ принята).
                    // Проверка стоит до арности, как и в ветке компонента:
                    // речь о форме вызова, а не о его аргументах.
                    if is_procedure && position != CallPosition::Statement {
                        return Err(SemaError::ProcedureAsFunction(name.clone()));
                    }
                    let arity = has_default.len();
                    // ИЗМЕРЕНО(CALL.OMITTED_TRAILING): 8.3.27 принимает `Ф(1)`
                    // при `Ф(а, б = 100)` и возвращает `1/100` — опущенные
                    // ХВОСТОВЫЕ позиции берут умолчание. Правило ПОЗИЦИОННОЕ, а
                    // не по числу обязательных: платформа принимает и
                    // обязательный параметр ПОСЛЕ необязательного (фикстура
                    // `ПоСсылкеРядомСПропуском`), поэтому счётное правило
                    // «допустить [required, arity]» пропустило бы `Ф(1, 2)` при
                    // `Ф(а, б = 2, в)` с непереданным обязательным `в`. Больше
                    // переданных позиций, чем параметров, — по-прежнему ошибка.
                    if args.len() > arity {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: arity,
                            found: args.len(),
                        });
                    }
                    // Каждая позиция `[0, arity)` разрешается сама: явный
                    // аргумент — `Value`; пропуск (`Ф(1, , 3)`) или опущенный
                    // хвост (`i >= args.len()`) допустимы лишь при умолчании у
                    // ЭТОГО параметра — тогда `ResolvedArg::Default`, — иначе
                    // обязательный параметр не передан.
                    let mut rargs = Vec::with_capacity(arity);
                    for (i, has_default_i) in has_default.iter().enumerate() {
                        match args.get(i) {
                            Some(Some(e)) => rargs.push(ResolvedArg::Value(self.resolve_expr(e)?)),
                            Some(None) | None if *has_default_i => {
                                rargs.push(ResolvedArg::Default);
                            }
                            Some(None) | None => {
                                return Err(SemaError::MissingRequiredArgument {
                                    name: name.clone(),
                                    index: i,
                                });
                            }
                        }
                    }
                    return Ok(RExpr::Call {
                        func: index,
                        args: rargs,
                    });
                }
                if let Some(registry) = self.registry
                    && let Some((library_index, function)) = registry.lookup_function(name)
                {
                    let descriptor = registry
                        .function(library_index, function)
                        .expect("индекс получен из таблицы имён этого реестра");
                    if descriptor.kind == bsl_rt::FunctionKind::Procedure
                        && position != CallPosition::Statement
                    {
                        return Err(SemaError::ProcedureAsFunction(name.clone()));
                    }
                    let found: u8 =
                        args.len()
                            .try_into()
                            .map_err(|_| SemaError::ArgumentCountMismatch {
                                name: name.clone(),
                                expected: descriptor.arity.max() as usize,
                                found: args.len(),
                            })?;
                    if !descriptor.arity.accepts(found) {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: descriptor.arity.max() as usize,
                            found: args.len(),
                        });
                    }
                    // Глобальные функции компонентов имеют ту же
                    // BSL-семантику пропущенных позиций, что и builtin:
                    // `ПрочитатьJSON(Ч, , Имена)` передаёт `Неопределено`.
                    let rargs = self.resolve_builtin_args(args)?;
                    let package = registry
                        .library(library_index)
                        .expect("индекс получен из таблицы имён этого реестра")
                        .package();
                    let library = bsl_rt::LibraryKey::new(package);
                    self.used_libraries.insert(library.clone());
                    return Ok(RExpr::CallComponent {
                        library,
                        function,
                        kind: descriptor.kind,
                        args: rargs,
                    });
                }
                // `Окр(x[, ЧислоРазрядов[, Режим]])` — единственный
                // builtin с необязательными аргументами, до генерального
                // механизма умолчаний builtin'ов (которого нет — см.
                // `bsl_rt::BuiltinFn::arity`, всегда фиксированная арность).
                // Подставляем недостающие `0` литералами здесь же, а не
                // заводим вариативную арность ради одной функции:
                // `BuiltinFn::Round` в рантайме всегда видит ровно 3
                // аргумента. `0` для режима означает "умолчание" (см.
                // `BslValue::round`).
                if bsl_rt::folded_eq(name, "Окр") || bsl_rt::folded_eq(name, "Round") {
                    const ROUND_ARITY: usize = 3;
                    if args.is_empty() || args.len() > ROUND_ARITY {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: ROUND_ARITY,
                            found: args.len(),
                        });
                    }
                    let mut rargs = self.resolve_builtin_args(args)?;
                    while rargs.len() < ROUND_ARITY {
                        // ИМЕННО `Неопределено`, а не `0`: измерено, что
                        // умолчание платформы совпадает с режимом 1, а не с
                        // режимом 0, — подстановка нуля меняла бы семантику.
                        rargs.push(RExpr::Undefined);
                    }
                    return Ok(RExpr::CallBuiltinFn {
                        builtin: bsl_rt::BuiltinFn::Round,
                        args: rargs,
                    });
                }
                if let Some(builtin) = bsl_rt::BuiltinFn::lookup(name) {
                    // Глобальная процедура легальна только оператором:
                    // `Х = Сообщить(1)` платформа отвергает («Обращение к
                    // процедуре как к функции») — ИЗМЕРЕНО, якорь
                    // `CALL.EXPR.PROCEDURE`.
                    if builtin.is_procedure() && position != CallPosition::Statement {
                        return Err(SemaError::ProcedureAsFunction(name.clone()));
                    }
                    let (min, max) = builtin.arity_range();
                    if args.len() < min || args.len() > max {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: max,
                            found: args.len(),
                        });
                    }
                    // Недостающие необязательные позиции добиваются
                    // `Неопределено` ЗДЕСЬ, а не вариативной арностью в
                    // рантайме: `call_builtin_fn` тогда всегда индексирует
                    // фиксированный набор аргументов, а «аргумент опущен»
                    // становится обычным значением, которое сама функция и
                    // трактует (`Сред` — до конца строки, `КодСимвола` —
                    // позиция 1, `СтрШаблон` — пустая подстановка).
                    let mut rargs = self.resolve_builtin_args(args)?;
                    while rargs.len() < max {
                        rargs.push(RExpr::Undefined);
                    }
                    return Ok(RExpr::CallBuiltinFn {
                        builtin,
                        args: rargs,
                    });
                }
                if bsl_rt::folded_eq(name, "Вычислить") || bsl_rt::folded_eq(name, "Eval")
                {
                    if args.len() != 1 {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            found: args.len(),
                        });
                    }
                    let mut rargs = self.resolve_required_args(args)?;
                    return Ok(RExpr::DynEval(Box::new(rargs.remove(0))));
                }
                Err(SemaError::UndefinedFunction(name.clone()))
            }
            AExpr::Field { obj, name } => {
                let method = bsl_rt::BuiltinMethod::lookup(name);
                // Фиксированные арности методов вынесены в
                // `BuiltinMethod::static_arity` (bsl-rt), чтобы их проверял и
                // резолвер здесь, и связывание VM на крафтнутом байт-коде.
                // `None` означает, что арность полиморфна по типу получателя
                // (`Добавить` — 0 у таблицы, 1 у массива; тип в динамическом
                // BSL здесь ещё не известен), и её решает рантайм.
                let expected: Option<usize> = method.and_then(bsl_rt::BuiltinMethod::static_arity);
                if let Some(expected) = expected
                    && args.len() != expected
                {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: name.clone(),
                        expected,
                        found: args.len(),
                    });
                }
                let rargs = self.resolve_required_args(args)?;
                let obj = self.resolve_expr(obj)?;
                // Открытый вызов обязателен, когда получатель может
                // оказаться компонентным объектом. Если же приёмник
                // статически доказан ядровым (все его перепривязки — один
                // `Новый T` из `NEW_TYPES`, см. `core_receivers`),
                // закрытый путь семантически совпадает с открытым и
                // остаётся измеренным горячим. Имена вне ядровой таблицы
                // компилятор и при `open=false` выпускает открытым
                // `CallObjectMethod`.
                let closed_is_safe = self.core_receiver_of(&obj).is_some();
                Ok(RExpr::CallMethod {
                    obj: Box::new(obj),
                    method: name.clone(),
                    open: self.registry.is_some() && !closed_is_safe,
                    args: rargs,
                })
            }
            _ => Err(SemaError::Unsupported(
                "вызов не по простому имени/методу появится позже",
            )),
        }
    }

    /// Аргументы ВСТРОЕННОЙ функции: пропущенная позиция (`Ф(1, , 3)`) —
    /// то же `Неопределено`, которым добиваются недостающие хвостовые
    /// позиции (см. вызывающие ветки). Без этого
    /// `ЗаполнитьЗначенияСвойств(П, И, , "Б")` — обычная для этой функции
    /// форма записи, когда нужен только последний параметр, — не
    /// компилировалась бы вовсе.
    ///
    /// У ПОЛЬЗОВАТЕЛЬСКИХ функций правило другое (`ResolvedArg::Default` и
    /// пролог умолчаний, см. `resolve_call`): там пропуск значит «взять
    /// объявленное значение по умолчанию», а объявить его встроенной
    /// функции негде. Пропуск ОБЯЗАТЕЛЬНОЙ позиции здесь не ловится: как и
    /// в 1С, это ошибка времени исполнения у той функции, которой достался
    /// `Неопределено`, а не ошибка компиляции.
    fn resolve_builtin_args(&mut self, args: &[Option<AExpr>]) -> Result<Vec<RExpr>, SemaError> {
        let mut rargs = Vec::with_capacity(args.len());
        for a in args {
            rargs.push(match a {
                Some(e) => self.resolve_expr(e)?,
                None => RExpr::Undefined,
            });
        }
        Ok(rargs)
    }

    /// Аргументы там, где пропуск позиции подставить нечем: методы
    /// объектов и `Вычислить`. У метода нет ни объявленных умолчаний (он
    /// не пользовательский), ни фиксированной арности, по которой можно
    /// было бы отличить пропущенный необязательный аргумент от
    /// пропущенного обязательного, — тип получателя в BSL известен только
    /// в рантайме. Поэтому здесь `Ф(1, , 3)` остаётся ошибкой резолвинга,
    /// в отличие от [`resolve_builtin_args`](Self::resolve_builtin_args).
    fn resolve_required_args(&mut self, args: &[Option<AExpr>]) -> Result<Vec<RExpr>, SemaError> {
        let mut rargs = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Some(e) => rargs.push(self.resolve_expr(e)?),
                None => {
                    return Err(SemaError::Unsupported(
                        "пропущенные аргументы Ф(1, , 3) появятся вместе со значениями по умолчанию",
                    ));
                }
            }
        }
        Ok(rargs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_syntax::parse;

    fn resolve_src(src: &str) -> Resolved {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        let stmts = items_to_stmts(prog.items);
        resolve_script(&stmts).unwrap_or_else(|e| panic!("sema error: {e:?}"))
    }

    /// В тестовых скриптах верхнего уровня допускаем только `Перем` и
    /// обычные операторы — объявления процедур/функций сюда не проверяем
    /// (для них нужны кадры, это M4).
    fn items_to_stmts(items: Vec<bsl_syntax::Item>) -> Vec<AStmt> {
        items
            .into_iter()
            .map(|item| match item {
                bsl_syntax::Item::Stmt(s) => s,
                bsl_syntax::Item::VarDecl(vd) => AStmt::VarDecl(vd),
                other => panic!("expected only statements/Перем in test script, got {other:?}"),
            })
            .collect()
    }

    fn resolve_program_src(src: &str) -> ResolvedProgram {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        resolve_program(&prog.items).unwrap_or_else(|e| panic!("sema error: {e:?}"))
    }

    /// `NEW_TYPES` — список для автодополнения, а разбор `Новый` живёт в
    /// `match` по имени типа. Разъехаться им нельзя: имя из списка,
    /// которого `resolve_new` не знает, REPL предлагал бы вхолостую.
    #[test]
    fn every_new_type_is_recognised_by_resolve_new() {
        for type_name in NEW_TYPES {
            let prog = parse(&format!("x = Новый {type_name}();")).unwrap();
            let stmts = items_to_stmts(prog.items);
            // Арность у типов разная (`ЗаписьТекста` требует путь), поэтому
            // ошибка числа аргументов здесь допустима — недопустимо
            // «такой тип не поддержан».
            match resolve_script(&stmts) {
                Ok(_) | Err(SemaError::ArgumentCountMismatch { .. }) => {}
                Err(other) => panic!("Новый {type_name}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_type_outside_the_list_is_not_constructible() {
        let prog = parse("x = Новый СписокЗначений();").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts),
            Err(SemaError::Unsupported(_))
        ));
    }

    #[test]
    fn implicit_declaration_on_first_assignment() {
        let r = resolve_src("PI = 3.14;");
        assert_eq!(r.locals, vec!["PI".to_string()]);
        assert_eq!(
            r.body,
            vec![RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Number(BslNumber::parse_canonical("3.14").unwrap()),
            }]
        );
    }

    #[test]
    fn reading_undefined_variable_is_an_error() {
        let prog = parse("y = x;").unwrap();
        let stmts = items_to_stmts(prog.items);
        let err = resolve_script(&stmts).unwrap_err();
        assert_eq!(err, SemaError::UndefinedVariable("x".to_string()));
    }

    #[test]
    fn case_insensitive_identifier_is_the_same_variable() {
        let r = resolve_src("x = 1;\nX = x + 1;");
        assert_eq!(r.locals, vec!["x".to_string()]);
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Binary {
                    op: bsl_syntax::BinaryOp::Add,
                    lhs: Box::new(RExpr::Local(0)),
                    rhs: Box::new(RExpr::Number(BslNumber::from_i64(1))),
                },
            }
        );
    }

    #[test]
    fn for_loop_variable_is_declared_and_alive_in_body() {
        let r = resolve_src("Для i = 0 По 10 Цикл\ny = i;\nКонецЦикла");
        assert_eq!(r.locals, vec!["i".to_string(), "y".to_string()]);
    }

    #[test]
    fn var_decl_registers_slot_without_runtime_effect() {
        let r = resolve_src("Перем a, b;\na = 1;");
        assert_eq!(r.locals, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(r.body.len(), 1);
    }

    #[test]
    fn calling_undeclared_function_is_an_error() {
        let prog = parse("Ф();").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::UndefinedFunction("Ф".to_string())
        );
    }

    #[test]
    fn builtin_function_call_resolves_without_user_declaration() {
        let r = resolve_src("x = sqrt(4);");
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::CallBuiltinFn {
                    builtin: bsl_rt::BuiltinFn::Sqrt,
                    args: vec![RExpr::Number(BslNumber::from_i64(4))],
                },
            }
        );
    }

    /// Синтетический реестр для проверки generic-механизмов: измеренные
    /// границы настоящих компонентов закреплены их собственными тестами
    /// (например, писатели архива в `bsl-zip`).
    fn synthetic_registry() -> bsl_rt::RuntimeRegistry {
        fn construct(
            _context: &mut bsl_rt::CallContext<'_>,
            _arguments: &[bsl_rt::BslValue],
        ) -> bsl_rt::RtResult<bsl_rt::BslValue> {
            Ok(bsl_rt::BslValue::Undefined)
        }
        fn call(
            _context: &mut bsl_rt::CallContext<'_>,
            _arguments: &[bsl_rt::BslValue],
        ) -> bsl_rt::RtResult<bsl_rt::BslValue> {
            Ok(bsl_rt::BslValue::Undefined)
        }
        const CONSTRUCTORS: &[bsl_rt::ConstructorDescriptor] = &[bsl_rt::ConstructorDescriptor {
            code: bsl_rt::ConstructorCode::new(1),
            names: &["Тестовый"],
            arity: bsl_rt::Arity::range(0, 2),
            call: construct,
        }];
        const FUNCTIONS: &[bsl_rt::FunctionDescriptor] = &[bsl_rt::FunctionDescriptor {
            code: bsl_rt::FunctionCode::new(1),
            names: &["ТестоваяПроцедура"],
            arity: bsl_rt::Arity::exact(1),
            kind: bsl_rt::FunctionKind::Procedure,
            call,
        }];
        const LIBRARY: bsl_rt::LibraryDescriptor =
            bsl_rt::LibraryDescriptor::new("test-lib", "0.0.0", bsl_rt::ObjectContextNeed::Reduced)
                .with_functions(FUNCTIONS)
                .with_constructors(CONSTRUCTORS);
        let mut builder = bsl_rt::RuntimeBuilder::new();
        builder.register(bsl_rt::core_library()).register(LIBRARY);
        builder.build().unwrap()
    }

    /// Арность конструктора реестра проверяется в `resolve_new` по
    /// дескриптору.
    #[test]
    fn a_registry_constructor_arity_mismatch_is_reported() {
        let registry = synthetic_registry();

        let resolve = |src: &str| {
            let prog = parse(src).unwrap();
            resolve_program_with_registry(&prog.items, &registry)
        };
        assert!(resolve("x = Новый Тестовый(1, 2);").is_ok());
        assert_eq!(
            resolve("x = Новый Тестовый(1, 2, 3);").unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Новый Тестовый".to_string(),
                expected: 2,
                found: 3,
            }
        );
    }

    /// Процедура компонента подчиняется тому же измеренному правилу, что и
    /// встроенная (`CALL.EXPR.PROCEDURE`): оператором — можно, в позиции
    /// выражения — «Обращение к процедуре как к функции».
    #[test]
    fn a_registry_procedure_is_a_statement_but_not_an_expression() {
        let registry = synthetic_registry();
        let resolve = |src: &str| {
            let prog = parse(src).unwrap();
            resolve_program_with_registry(&prog.items, &registry)
        };
        assert!(resolve("ТестоваяПроцедура(1);").is_ok());
        assert_eq!(
            resolve("x = ТестоваяПроцедура(1);").unwrap_err(),
            SemaError::ProcedureAsFunction("ТестоваяПроцедура".to_string())
        );
    }

    #[test]
    fn builtin_function_arity_mismatch_is_an_error() {
        let prog = parse("x = Pow(2);").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert_eq!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Pow".to_string(),
                expected: 2,
                found: 1,
            }
        );
    }

    #[test]
    fn count_method_call_resolves_on_array() {
        let r = resolve_src("a = Новый Массив(3);\nn = a.Count();");
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::CallMethod {
                    obj: Box::new(RExpr::Local(0)),
                    method: "Count".to_string(),
                    open: false,
                    args: vec![],
                },
            }
        );
    }

    #[test]
    fn an_unknown_method_is_left_for_the_runtime_receiver() {
        let prog = parse("a = Новый Массив(3);\nn = a.НетТакогоМетода();").unwrap();
        let stmts = items_to_stmts(prog.items);
        let resolved = resolve_script(&stmts).unwrap();
        assert!(matches!(
            &resolved.body[1],
            RStmt::AssignLocal {
                value: RExpr::CallMethod { method, .. },
                ..
            } if method == "НетТакогоМетода"
        ));
    }

    #[test]
    fn functions_can_be_called_before_their_declaration() {
        // Main вызывает Helper, объявленную ниже по тексту — должно работать,
        // сигнатуры собираются за отдельный проход до резолвинга тел.
        let rp = resolve_program_src(
            "Функция Main()\nВозврат Helper();\nКонецФункции\n\nФункция Helper()\nВозврат 1;\nКонецФункции",
        );
        assert_eq!(rp.functions.len(), 2);
        assert_eq!(rp.functions[0].name, "Main");
        assert_eq!(
            rp.functions[0].body,
            vec![RStmt::Return(Some(RExpr::Call {
                func: 1,
                args: vec![],
            }))]
        );
    }

    #[test]
    fn params_occupy_the_first_slots_by_val_flag_recorded() {
        let rp = resolve_program_src("Процедура П(Знач а, б)\nКонецПроцедуры");
        let f = &rp.functions[0];
        assert_eq!(f.locals[0], "а");
        assert_eq!(f.locals[1], "б");
        assert!(f.params[0].by_val);
        assert!(!f.params[1].by_val);
    }

    #[test]
    fn argument_count_mismatch_is_an_error() {
        let prog = parse("Функция Ф(а)\nВозврат а;\nКонецФункции\nx = Ф(1, 2);").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Ф".to_string(),
                expected: 1,
                found: 2,
            }
        );
    }

    #[test]
    fn skipping_a_parameter_without_a_default_is_an_error() {
        let prog = parse("Функция Ф(а, б)\nВозврат а;\nКонецФункции\nx = Ф(1, );").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                index: 1,
            }
        );
        // В тексте — человеческая позиция: пропущен ВТОРОЙ аргумент.
        assert_eq!(
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                index: 1,
            }
            .to_string(),
            "у «Ф» пропущен обязательный аргумент на позиции 2"
        );
        // Тип публичный, поэтому его печать обязана быть тотальной:
        // резолвер предельного индекса не породит, но встраивающая
        // программа вправе собрать вариант руками, а `index + 1` на
        // пределе — паника в debug и ноль в release.
        assert_eq!(
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                index: usize::MAX,
            }
            .to_string(),
            format!(
                "у «Ф» пропущен обязательный аргумент на позиции {}",
                usize::MAX
            )
        );
    }

    #[test]
    fn skipping_a_defaulted_parameter_resolves_to_skipped_marker() {
        let prog = parse("Функция Ф(а, б = 100)\nВозврат а;\nКонецФункции\nx = Ф(1, );").unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        match &resolved.top_level.body[0] {
            RStmt::AssignLocal {
                value: RExpr::Call { args, .. },
                ..
            } => {
                assert_eq!(args[1], ResolvedArg::Default);
            }
            other => panic!("expected AssignLocal(Call), got {other:?}"),
        }
    }

    /// ИЗМЕРЕНО(CALL.OMITTED_TRAILING): 8.3.27 принимает `Ф(1)` при
    /// `Ф(а, б = 100)` и возвращает `1/100`. Опущенная ХВОСТОВАЯ позиция
    /// берёт умолчание (`ResolvedArg::Default`), а не отвергается по числу
    /// аргументов — правило арности вызова позиционное.
    #[test]
    fn an_omitted_trailing_optional_argument_takes_its_default() {
        let prog = parse("Функция Ф(а, б = 100)\nВозврат а;\nКонецФункции\nx = Ф(1);").unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        match &resolved.top_level.body[0] {
            RStmt::AssignLocal {
                value: RExpr::Call { args, .. },
                ..
            } => {
                assert_eq!(args.len(), 2, "хвостовая позиция добита до арности");
                assert!(matches!(args[0], ResolvedArg::Value(_)));
                assert_eq!(args[1], ResolvedArg::Default);
            }
            other => panic!("ожидался AssignLocal(Call), получено {other:?}"),
        }
    }

    /// ИЗМЕРЕНО: платформа принимает обязательный параметр ПОСЛЕ
    /// необязательного (фикстура `ПоСсылкеРядомСПропуском`), поэтому правило
    /// не «число позиций без умолчания». При `Ф(а, б = 100, в)` вызов
    /// `Ф(1, 2)` — не «мало аргументов», а пропущенный обязательный `в` на
    /// третьей позиции: счётное правило приняло бы его ошибочно.
    #[test]
    fn a_required_parameter_after_an_optional_one_must_be_supplied() {
        let prog =
            parse("Функция Ф(а, б = 100, в)\nВозврат а;\nКонецФункции\nx = Ф(1, 2);").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                index: 2,
            }
        );
        assert_eq!(
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                index: 2,
            }
            .to_string(),
            "у «Ф» пропущен обязательный аргумент на позиции 3"
        );
    }

    /// Литерал со scale больше `bsl_number::MAX_SCALE` проходит лексер, но
    /// `BslNumber` его не представляет. `Engine::compile` обязана вернуть
    /// `Result`, а не паниковать, — и в ЛЮБОМ профиле (прежде это была
    /// паника, а debug и release расходятся ровно на переполнении).
    #[test]
    fn a_numeric_literal_beyond_max_scale_is_an_error_not_a_panic() {
        let src = format!("х = 0.{};", "3".repeat(100_001));
        let prog = parse(&src).expect("лексер принимает длинную дробь");
        assert!(matches!(
            resolve_program(&prog.items),
            Err(SemaError::BadNumericLiteral(_))
        ));
    }

    /// У ВСТРОЕННОЙ функции объявленных умолчаний нет, поэтому пропуск —
    /// не `ResolvedArg::Default`, а `Неопределено`: ровно то же, чем добиваются
    /// недостающие хвостовые позиции. Ради этой формы записи
    /// (`ЗаполнитьЗначенияСвойств(П, И, , "Б")`) правило и заведено.
    #[test]
    fn skipping_a_builtin_argument_resolves_to_undefined() {
        let resolved =
            resolve_src("П = Новый Структура(\"А\", 1);\nЗаполнитьЗначенияСвойств(П, П, , \"Б\");");
        let RStmt::ExprStmt(RExpr::CallBuiltinFn { builtin, args }) = &resolved.body[1] else {
            panic!(
                "ожидался вызов встроенной функции, получено {:?}",
                resolved.body[1]
            );
        };
        assert_eq!(*builtin, bsl_rt::BuiltinFn::FillPropertyValues);
        assert_eq!(args.len(), 4);
        assert_eq!(args[2], RExpr::Undefined, "пропущенная позиция");
        assert!(
            matches!(args[3], RExpr::Str(_)),
            "последняя позиция на месте"
        );
    }

    /// Правило платформы о позиции вызова трёхчастное — снято полным
    /// перебором таблицы имён, деление хранит `BuiltinFn::is_intrinsic`.
    /// Функция встроенного языка оператором — отказ компиляции.
    #[test]
    fn an_intrinsic_call_as_a_statement_is_rejected() {
        for src in [
            "Строка(1);",
            "СтрЗаменить(\"а\", \"а\", \"б\");",
            "ТекущаяДата();",
            "Вычислить(\"1\");",
            "StrLen(\"аб\");",
        ] {
            let prog = parse(src).unwrap();
            let err = resolve_script(&items_to_stmts(prog.items)).unwrap_err();
            assert!(
                matches!(err, SemaError::BuiltinFunctionAsStatement(_)),
                "{src}: {err:?}"
            );
        }
    }

    /// Функция глобального контекста оператором законна — платформа зовёт
    /// её и отбрасывает результат, а нуль-арная просто выполняется.
    /// Процедуры оператором — тем более.
    #[test]
    fn a_context_function_call_as_a_statement_is_allowed() {
        resolve_src("СтрНайти(\"а\", \"б\");");
        resolve_src("ТекущаяУниверсальнаяДатаВМиллисекундах();");
        resolve_src("Сообщить(1);");
    }

    /// Глобальная процедура в позиции выражения — «Обращение к процедуре
    /// как к функции», в том числе аргументом другого вызова.
    #[test]
    fn a_procedure_in_an_expression_is_rejected() {
        // Позиция оператора относится ровно к ВЕРХНЕМУ узлу; глубина
        // вложения роли не играет. Пока её нёс изменяемый флаг, это
        // держалось на том, что вход в разрешение выражения его погасит, —
        // теперь позиция просто не передаётся вглубь (см. `CallPosition`).
        for src in [
            "Х = Сообщить(1);",
            "Сообщить(Сообщить(1));",
            "Х = ЗаполнитьЗначенияСвойств(1, 2);",
            "Х = 1 + Сообщить(1);",
            "Х = ?(Истина, Сообщить(1), 2);",
            "м = Новый Массив(3);\nм[0] = Сообщить(1);",
            "м = Новый Массив(3);\nСообщить(м[Сообщить(0)]);",
            "с = Новый Структура(\"а\", 1);\nс.а = Сообщить(1);",
        ] {
            let prog = parse(src).unwrap();
            let err = resolve_script(&items_to_stmts(prog.items)).unwrap_err();
            assert!(
                matches!(err, SemaError::ProcedureAsFunction(_)),
                "{src}: {err:?}"
            );
        }
    }

    /// Своя процедура в позиции выражения — то же «Обращение к процедуре
    /// как к функции», что и у процедуры глобального контекста, но ветка
    /// разрешения другая: имя ищется в таблице объявлений модуля.
    /// ИЗМЕРЕНО на 8.3.27 — строка «своя процедура выражением» в
    /// `measure-stmtcall.platform.txt`: отказ, при том что строкой выше
    /// та же процедура ОПЕРАТОРОМ принята.
    ///
    /// Проверяется через `resolve_program`, а не `resolve_script`:
    /// последний разрешает плоский скрипт БЕЗ объявлений, то есть
    /// объявить в нём процедуру попросту негде.
    #[test]
    fn a_user_procedure_in_an_expression_is_rejected() {
        let decls = concat!(
            "Процедура П()\n",
            "КонецПроцедуры\n",
            "Функция Ф(а)\n",
            "\tВозврат а;\n",
            "КонецФункции\n",
        );
        let resolve = |tail: &str| {
            let prog = parse(&format!("{decls}{tail}")).unwrap();
            resolve_program(&prog.items)
        };

        for tail in [
            "Х = П();",
            "Х = 1 + П();",
            "Ф(П());",
            "Х = ?(Истина, П(), 2);",
            "м = Новый Массив(3);\nм[0] = П();",
            "м = Новый Массив(3);\nФ(м[П()]);",
            "с = Новый Структура(\"а\", 1);\nс.а = П();",
            "Если П() Тогда\nКонецЕсли;",
            "Возврат П();",
        ] {
            let err = resolve(tail).unwrap_err();
            assert!(
                matches!(err, SemaError::ProcedureAsFunction(_)),
                "{tail}: {err:?}"
            );
        }

        // Контроль: та же процедура ОПЕРАТОРОМ законна, и своя ФУНКЦИЯ
        // законна в обеих позициях — запрет держится на виде объявления,
        // а не на том, что имя пользовательское.
        for tail in ["П();", "Х = Ф(1);", "Ф(1);", "Х = Ф(Ф(1));"] {
            assert!(resolve(tail).is_ok(), "должно разрешаться: {tail}");
        }
    }

    /// Тот же запрет во ФРАГМЕНТЕ `Выполнить`/`Вычислить`: там резолвинг
    /// идёт в рантайме по [`SnippetSignature`], и вид объявления должен
    /// доехать до него через `Chunk::is_procedure`. Именно этой формой
    /// правило и измерено — ошибку компиляции модуля на платформе иначе
    /// не увидеть.
    #[test]
    fn a_user_procedure_in_a_dynamic_expression_is_rejected() {
        let sigs = [
            SnippetSignature {
                name: "П".to_string(),
                arity: 0,
                is_procedure: true,
                has_default: vec![],
            },
            SnippetSignature {
                name: "Ф".to_string(),
                arity: 0,
                is_procedure: false,
                has_default: vec![],
            },
        ];
        let snippet = |src: &str| {
            let prog = parse(src).unwrap();
            resolve_snippet_stmts(&[], &[], &items_to_stmts(prog.items), &sigs)
        };
        assert!(matches!(
            snippet("Х = П();").unwrap_err(),
            SemaError::ProcedureAsFunction(_)
        ));
        assert!(snippet("П();").is_ok());
        assert!(snippet("Х = Ф();").is_ok());
    }

    /// REPL мягче модуля: голый вызов функции языка там печатает значение,
    /// а строгий фрагмент `Выполнить` идёт через `resolve_snippet_stmts` и
    /// отвергает его, как платформа.
    #[test]
    fn the_repl_keeps_bare_intrinsic_calls() {
        let prog = parse("СтрДлина(\"аб\");").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(resolve_repl_stmts(&[], &[], &stmts, &[]).is_ok());
        assert!(matches!(
            resolve_snippet_stmts(&[], &[], &stmts, &[]),
            Err(SemaError::BuiltinFunctionAsStatement(_))
        ));
    }

    #[test]
    fn duplicate_function_name_is_an_error() {
        let prog = parse("Функция Ф()\nКонецФункции\nПроцедура ф()\nКонецПроцедуры").unwrap();
        assert!(matches!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::DuplicateFunction(_)
        ));
    }

    #[test]
    fn top_level_can_call_functions_declared_anywhere_in_module() {
        let rp = resolve_program_src("Процедура П(x)\nКонецПроцедуры\nП(1);");
        assert_eq!(
            rp.top_level.body,
            vec![RStmt::ExprStmt(RExpr::Call {
                func: 0,
                args: vec![ResolvedArg::Value(RExpr::Number(BslNumber::from_i64(1)))],
            })]
        );
    }

    #[test]
    fn new_array_resolves_dimensions() {
        let r = resolve_src("a = Новый Массив(3, 4);");
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewArray {
                    dims: vec![
                        RExpr::Number(BslNumber::from_i64(3)),
                        RExpr::Number(BslNumber::from_i64(4)),
                    ],
                },
            }
        );
    }

    #[test]
    fn new_structure_with_literal_keys_and_values() {
        let r = resolve_src(r#"s = Новый Структура("x,y", 1, 2);"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewStructure {
                    keys: vec!["x".to_string(), "y".to_string()],
                    values: vec![
                        RExpr::Number(BslNumber::from_i64(1)),
                        RExpr::Number(BslNumber::from_i64(2)),
                    ],
                },
            }
        );
    }

    #[test]
    fn new_structure_keys_only_defaults_values_to_undefined() {
        let r = resolve_src(r#"s = Новый Структура("x,y");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewStructure {
                    keys: vec!["x".to_string(), "y".to_string()],
                    values: vec![RExpr::Undefined, RExpr::Undefined],
                },
            }
        );
    }

    #[test]
    fn new_structure_with_dynamic_key_list_is_unsupported() {
        // Проверка формы аргумента (строковый литерал?) идёт раньше
        // резолвинга самого идентификатора, поэтому это Unsupported, а не
        // UndefinedVariable("k"), даже когда k нигде не объявлена.
        let prog = parse("s = Новый Структура(k);").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::Unsupported(_)
        ));
    }

    #[test]
    fn index_and_field_assignment_targets() {
        let r =
            resolve_src("a = Новый Массив(1);\ns = Новый Структура(\"x\");\na[0] = 1;\ns.x = 2;");
        assert!(matches!(r.body[2], RStmt::AssignIndex { .. }));
        assert!(matches!(r.body[3], RStmt::AssignField { .. }));
    }

    #[test]
    fn for_each_declares_loop_variable() {
        let r = resolve_src("a = Новый Массив();\nДля Каждого x Из a Цикл\ny = x;\nКонецЦикла");
        assert_eq!(r.locals[0], "a".to_string());
        assert!(matches!(r.body[1], RStmt::ForEach { .. }));
    }

    #[test]
    fn try_except_resolves_both_bodies() {
        let r = resolve_src("Попытка\nx = 1;\nИсключение\ny = 2;\nКонецПопытки");
        assert!(matches!(r.body[0], RStmt::Try { .. }));
    }

    #[test]
    fn raise_with_and_without_expression() {
        let r = resolve_src(
            "Попытка\nВызватьИсключение \"ошибка\";\nИсключение\nВызватьИсключение;\nКонецПопытки",
        );
        match &r.body[0] {
            RStmt::Try { body, except_body } => {
                assert_eq!(
                    body[0],
                    RStmt::Raise(Some(RExpr::Str("ошибка".to_string())))
                );
                assert_eq!(except_body[0], RStmt::Raise(None));
            }
            other => panic!("expected Try, got {other:?}"),
        }
    }

    #[test]
    fn execute_resolves_to_rstmt_execute() {
        let r = resolve_src(r#"Выполнить("x = 1");"#);
        assert_eq!(r.body[0], RStmt::Execute(RExpr::Str("x = 1".to_string())));
    }

    #[test]
    fn vychislit_resolves_to_dyn_eval() {
        let r = resolve_src(r#"y = Вычислить("2+2");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::DynEval(Box::new(RExpr::Str("2+2".to_string()))),
            }
        );
    }

    /// Строчное написание встроенного имени свёрнуто через `folded_eq`, а не
    /// `eq_ignore_ascii_case`: на кириллице последнее — ложь, отчего `окр`
    /// не доходил до спецразбора `Окр` и отвергался, хотя `Окр(1.5)`
    /// принимался.
    #[test]
    fn a_lowercase_okr_resolves_like_its_canonical_form() {
        let r = resolve_src("х = окр(1.5);");
        match &r.body[0] {
            RStmt::AssignLocal {
                value: RExpr::CallBuiltinFn { builtin, .. },
                ..
            } => assert_eq!(*builtin, bsl_rt::BuiltinFn::Round),
            other => panic!("ожидался вызов Round, получено {other:?}"),
        }
    }

    /// Тот же изъян для `Вычислить`: строчное `вычислить` не распознавалось
    /// ни резолвером, ни пре-проходом `core_receivers` — и оба свёрнуты вместе.
    #[test]
    fn a_lowercase_vychislit_resolves_to_dyn_eval() {
        let r = resolve_src(r#"y = вычислить("2+2");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::DynEval(Box::new(RExpr::Str("2+2".to_string()))),
            }
        );
    }

    /// Разнорегистровый дубль `Перем` — один слот, а не два. Дедуп
    /// объявлений сворачивается тем же `folded_eq`, что и поиск слота
    /// (`module_index` по `to_uppercase`): иначе `Счётчик` и `счётчик` дают
    /// два слота, поиск последним-побеждает уводит их в один, а первый
    /// остаётся фантомным — запись из процедуры и чтение сверху расходятся.
    #[test]
    fn a_mixed_case_repeated_module_var_collapses_to_one_slot() {
        let resolved = resolve_program_src(
            "Перем Счётчик;\n\
             Перем счётчик;\n\
             Процедура П()\n\
                 Счётчик = 99;\n\
             КонецПроцедуры\n\
             П();\n\
             Сообщить(Счётчик);",
        );
        assert_eq!(resolved.module_vars.len(), 1, "разнорегистровый дубль слит");
    }

    #[test]
    fn resolve_snippet_stmts_seeds_existing_locals_and_extends() {
        let existing = vec!["x".to_string()];
        let prog = parse("x = x + 1;\ny = 2;").unwrap();
        let stmts = items_to_stmts(prog.items);
        let (locals, body) = resolve_snippet_stmts(&existing, &[], &stmts, &[]).unwrap();
        assert_eq!(locals, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(
            body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Binary {
                    op: bsl_syntax::BinaryOp::Add,
                    lhs: Box::new(RExpr::Local(0)),
                    rhs: Box::new(RExpr::Number(BslNumber::from_i64(1))),
                },
            }
        );
        assert_eq!(
            body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::Number(BslNumber::from_i64(2))
            }
        );
    }

    #[test]
    fn command_line_arguments_reads_bare_and_a_local_shadows_it() {
        // Голое имя без объявления — чтение встроенной функции, а не
        // UndefinedVariable; регистр и английский синоним не важны.
        for name in [
            "АргументыКоманднойСтроки",
            "аргументыкоманднойстроки",
            "CommandLineArguments",
        ] {
            let r = resolve_src(&format!("А = {name};"));
            assert_eq!(
                r.body[0],
                RStmt::AssignLocal {
                    slot: 0,
                    value: RExpr::CallBuiltinFn {
                        builtin: bsl_rt::BuiltinFn::CommandLineArguments,
                        args: Vec::new(),
                    },
                },
                "{name}"
            );
        }

        // Присваивание объявляет обычную локальную переменную, и дальше
        // имя читается из неё, а не из встроенной функции.
        let r = resolve_src("АргументыКоманднойСтроки = 1;\nБ = АргументыКоманднойСтроки;");
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::Local(0),
            }
        );
    }

    /// Все пары «имя метода, open» из разрешённой программы — в порядке
    /// обхода. Только для проверок ниже: ходит ровно по тем узлам, где
    /// вызов метода может встретиться в этих тестах.
    fn call_opens(program: &ResolvedProgram) -> Vec<(String, bool)> {
        fn from_expr(expr: &RExpr, into: &mut Vec<(String, bool)>) {
            match expr {
                RExpr::CallMethod {
                    obj,
                    method,
                    open,
                    args,
                } => {
                    from_expr(obj, into);
                    into.push((method.clone(), *open));
                    for arg in args {
                        from_expr(arg, into);
                    }
                }
                RExpr::Unary { expr, .. } => from_expr(expr, into),
                RExpr::Binary { lhs, rhs, .. } => {
                    from_expr(lhs, into);
                    from_expr(rhs, into);
                }
                RExpr::Call { args, .. } => {
                    for arg in args {
                        if let ResolvedArg::Value(arg) = arg {
                            from_expr(arg, into);
                        }
                    }
                }
                RExpr::CallBuiltinFn { args, .. }
                | RExpr::CallComponent { args, .. }
                | RExpr::CreateObject { args, .. } => {
                    for arg in args {
                        from_expr(arg, into);
                    }
                }
                RExpr::Index { obj, index } => {
                    from_expr(obj, into);
                    from_expr(index, into);
                }
                RExpr::Field { obj, .. } => from_expr(obj, into),
                _ => {}
            }
        }
        fn from_block(body: &[RStmt], into: &mut Vec<(String, bool)>) {
            for stmt in body {
                match stmt {
                    RStmt::AssignLocal { value, .. }
                    | RStmt::AssignModuleVar { value, .. }
                    | RStmt::ExprStmt(value)
                    | RStmt::Execute(value) => from_expr(value, into),
                    RStmt::ForNumeric { from, to, body, .. } => {
                        from_expr(from, into);
                        from_expr(to, into);
                        from_block(body, into);
                    }
                    RStmt::ForEach { iter, body, .. } => {
                        from_expr(iter, into);
                        from_block(body, into);
                    }
                    RStmt::While { cond, body } => {
                        from_expr(cond, into);
                        from_block(body, into);
                    }
                    RStmt::If {
                        cond,
                        then_branch,
                        elsif_branches,
                        else_branch,
                    } => {
                        from_expr(cond, into);
                        from_block(then_branch, into);
                        for (c, b) in elsif_branches {
                            from_expr(c, into);
                            from_block(b, into);
                        }
                        if let Some(b) = else_branch {
                            from_block(b, into);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut collected = Vec::new();
        from_block(&program.top_level.body, &mut collected);
        for function in &program.functions {
            from_block(&function.body, &mut collected);
        }
        collected
    }

    fn opens_with_registry(src: &str) -> Vec<(String, bool)> {
        let registry = synthetic_registry();
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        let resolved = resolve_program_with_registry(&prog.items, &registry)
            .unwrap_or_else(|e| panic!("sema error: {e:?}"));
        call_opens(&resolved)
    }

    /// Приёмник, доказанно ядровый, компилируется закрытым даже с
    /// реестром: это и есть возврат измеренного горячего пути.
    #[test]
    fn a_proven_core_receiver_closes_its_method_calls() {
        let opens = opens_with_registry(
            "ф = Новый ЗаписьТекста(\"о.тмп\");\n\
             ф.Записать(\"х\");\n\
             ф.Закрыть();\n\
             с = Новый Структура(\"а\", 1);\n\
             с.Вставить(\"б\", 2);",
        );
        assert_eq!(
            opens,
            vec![
                ("Записать".to_string(), false),
                ("Закрыть".to_string(), false),
                ("Вставить".to_string(), false),
            ]
        );
    }

    /// `Записать`/`Закрыть` больше не выделены: спец-опкодов у
    /// `ЗаписьТекста` нет, и пара «метод, арность» ничего не решает.
    /// Доказанный ядровой приёмник закрывается ВСЕГДА, каким бы ни был
    /// метод, — а применим ли метод к этому получателю, решает рантайм
    /// ровно так же, как на открытом пути (проверено: `Структура.Записать`
    /// по-прежнему ловится `Попыткой`).
    #[test]
    fn a_core_receiver_closes_every_method_not_just_some() {
        let opens = opens_with_registry(
            "с = Новый Структура(\"а\", 1);\n\
             с.Записать(1);\n\
             с.Закрыть();",
        );
        assert_eq!(
            opens,
            vec![
                ("Записать".to_string(), false),
                ("Закрыть".to_string(), false),
            ]
        );
    }

    /// Конструктор реестра (компонент) — не ядровый приёмник.
    #[test]
    fn a_registry_constructed_receiver_stays_open() {
        let opens = opens_with_registry(
            "т = Новый Тестовый();\n\
             т.Добавить(1);",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), true)]);
    }

    /// Любая перепривязка не-`Новый` правым выражением снимает вердикт.
    #[test]
    fn a_reassignment_from_an_unknown_value_reopens_the_calls() {
        let opens = opens_with_registry(
            "м = Новый Массив;\n\
             м = 1;\n\
             м.Добавить(1);",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), true)]);
    }

    /// Роль переменной цикла — перепривязка на каждой итерации.
    #[test]
    fn loop_variables_are_not_tracked() {
        let opens = opens_with_registry(
            "м = Новый Массив;\n\
             Для м = 1 По 2 Цикл КонецЦикла;\n\
             м.Добавить(1);\n\
             к = Новый Массив;\n\
             Для Каждого к Из м Цикл КонецЦикла;\n\
             к.Добавить(1);",
        );
        assert_eq!(
            opens,
            vec![
                ("Добавить".to_string(), true),
                ("Добавить".to_string(), true),
            ]
        );
    }

    /// Передача голым именем в by-ref параметр пользовательской функции
    /// может переприсвоить слот; `Знач` — не может.
    #[test]
    fn by_ref_argument_passing_kills_the_verdict_but_by_val_does_not() {
        let opens = opens_with_registry(
            "Процедура ПоСсылке(х)\nКонецПроцедуры\n\
             Процедура ПоЗначению(Знач х)\nКонецПроцедуры\n\
             м = Новый Массив;\n\
             ПоСсылке(м);\n\
             м.Добавить(1);\n\
             к = Новый Массив;\n\
             ПоЗначению(к);\n\
             к.Добавить(1);",
        );
        assert_eq!(
            opens,
            vec![
                ("Добавить".to_string(), true),
                ("Добавить".to_string(), false),
            ]
        );
    }

    /// `Выполнить`/`Вычислить` видят слоты кадра — динамика в области
    /// снимает отслеживание её переменных.
    #[test]
    fn dynamic_execution_in_scope_reopens_the_calls() {
        let opens = opens_with_registry(
            "м = Новый Массив;\n\
             Выполнить(\"м = 1\");\n\
             м.Добавить(1);",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), true)]);

        let opens = opens_with_registry(
            "м = Новый Массив;\n\
             х = Вычислить(\"1\");\n\
             м.Добавить(1);",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), true)]);
    }

    /// Модульная переменная отслеживается по всему модулю: присваивание
    /// в любой функции засчитывается, затенение своей `Перем` — нет.
    #[test]
    fn module_variables_aggregate_sites_across_functions() {
        // Функция переприсваивает модульную — вердикта нет ни у неё, ни
        // на верхнем уровне.
        let opens = opens_with_registry(
            "Перем м;\n\
             Процедура П()\n\
                 м = 1;\n\
             КонецПроцедуры\n\
             м = Новый Массив;\n\
             м.Добавить(1);",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), true)]);

        // Функция работает со СВОЕЙ `Перем` — модульная чиста, и оба
        // вызова закрыты: локальный тоже доказан (`Новый Соответствие`).
        let opens = opens_with_registry(
            "Перем м;\n\
             Процедура П()\n\
                 Перем м;\n\
                 м = Новый Соответствие;\n\
                 м.Вставить(1, 2);\n\
             КонецПроцедуры\n\
             м = Новый Массив;\n\
             м.Добавить(1);\n\
             П();",
        );
        assert_eq!(
            opens,
            vec![
                ("Добавить".to_string(), false),
                ("Вставить".to_string(), false),
            ]
        );

        // Чтение модульной из функции пользуется модульным вердиктом.
        let opens = opens_with_registry(
            "Перем м;\n\
             Процедура П()\n\
                 м.Добавить(2);\n\
             КонецПроцедуры\n\
             м = Новый Массив;\n\
             П();",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), false)]);
    }

    /// Параметры приходят с неизвестным типом и не отслеживаются.
    #[test]
    fn function_parameters_are_never_tracked() {
        let opens = opens_with_registry(
            "Процедура П(м)\n\
                 м.Добавить(1);\n\
             КонецПроцедуры\n\
             П(Новый Массив);",
        );
        assert_eq!(opens, vec![("Добавить".to_string(), true)]);
    }
}
