use bsl_rt::{BslValue, NameInterner, ShapeTable};
use bsl_sema::{
    LabelId, RExpr, RStmt, ResolvedArg, ResolvedFunction, ResolvedParam, ResolvedProgram,
};
use bsl_syntax::{BinaryOp, UnaryOp};

use bsl_bytecode::{
    ArgMode, Chunk, ExceptionRange, Instr, LibraryRequirement, Program, analysis, bundle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    TooManyLocals,
    TooManyRegisters,
    TooManyConstants,
    TooManyArgModeTables,
    TooManyShapes,
    TooManyNames,
    /// Номер модульной переменной не помещается в `u16` при передаче её по
    /// ссылке (`ArgMode::ByRefModuleVar`). Модуль с четвертью миллиона
    /// переменных — тот же класс, что `JumpTargetOutOfRange`: `as u16` молча
    /// обрезал бы слот, и параметр по ссылке алиасил бы чужую переменную.
    TooManyModuleVars,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    /// Фрагмент `Выполнить` зовёт функцию с номером, которого нет ни среди
    /// его собственных объявлений, ни среди функций окружающей программы.
    /// Корректный резолвинг такого не даёт — но текст фрагмента приходит из
    /// рантайма, поэтому это ошибка, а не паника.
    UnknownFunction,
    /// Разрешённое дерево сослалось на компонент, которого нет в полном
    /// списке требований программы.
    UnknownLibrary(String),
    /// Цель перехода не помещается в `i16`, то есть чанк длиннее
    /// `i16::MAX` инструкций.
    ///
    /// До этой проверки `as i16` усекал цель молча: в чанке на сорок тысяч
    /// инструкций цель 40004 превращалась в -25532, VM получала `pc` далеко
    /// за концом чанка, принимала это за нормальное завершение и заканчивала
    /// программу БЕЗ вывода и БЕЗ ошибки. Неверный ответ хуже отказа,
    /// поэтому здесь отказ.
    JumpTargetOutOfRange,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Все пределы — в номерах регистров и индексах таблиц чанка, то
        // есть в ширине полей `Instr`; менять её нельзя без отдельного
        // измерения (см. `size_of::<Instr>()`).
        let what = match self {
            CompileError::TooManyLocals => "слишком много локальных переменных в кадре",
            CompileError::TooManyRegisters => "слишком много регистров в кадре",
            CompileError::TooManyConstants => "слишком много констант в чанке",
            CompileError::TooManyArgModeTables => "слишком много наборов режимов аргументов",
            CompileError::TooManyShapes => "слишком много форм структур",
            CompileError::TooManyNames => "слишком много имён",
            CompileError::TooManyModuleVars => "слишком много переменных модуля для ссылки",
            CompileError::BreakOutsideLoop => "«Прервать» вне цикла",
            CompileError::ContinueOutsideLoop => "«Продолжить» вне цикла",
            CompileError::UnknownFunction => "вызов функции, которой нет в модуле",
            CompileError::JumpTargetOutOfRange => "цель перехода не помещается в чанк",
            CompileError::UnknownLibrary(name) => {
                return write!(f, "компонент «{name}» не объявлен в требованиях");
            }
        };
        f.write_str(what)
    }
}

impl std::error::Error for CompileError {}

/// Компилирует весь модуль: чанк верхнего уровня плюс чанк на каждую
/// `Процедура`/`Функция`, в том порядке, в котором их видит `bsl-sema`
/// (`Call.func` в разрешённом дереве — индекс в `resolved.functions`;
/// здесь он сдвигается на 1, потому что `chunks[0]` — верхний уровень).
///
/// Имена полей и формы структур интернируются ОДИН РАЗ на весь модуль
/// (`names`/`shapes` ниже) — а не по чанку — чтобы одинаковый список полей
/// в разных функциях модуля давал одну и ту же форму: это и есть смысл
/// "глобального" интернирования из брифа применительно к тому, что реально
/// компилируется за один проход.
///
/// # Errors
///
/// Возвращает [`CompileError`], если программа превышает лимиты формата байт-кода
/// или содержит `Прервать`/`Продолжить` вне цикла.
/// Какие оптимизирующие проходы включены. Все по умолчанию выключены:
/// ни один из них ещё не проходил ворота допуска, описанные в
/// `docs/research/performance/ssa-hotspot-analysis.md`, поэтому включает их только тот, кто
/// делает это осознанно.
///
/// Свёртка констант разделена на два переключателя не ради
/// настраиваемости, а ради честного замера. Ворота снимают число для
/// одного прохода; если ранняя свёртка и поздний проход делят флаг,
/// измеренное ускорение принадлежит паре, а не тому, что уходит в main —
/// ровно та подмена, из-за которой недействительны числа шага 6. Сверх
/// того, поздний проход оправдан только константами, возникшими уже
/// ПОСЛЕ кодогенерации, и увидеть их можно, лишь включив его отдельно.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Optimizations {
    /// Свёртка констант в кодогене, до эмиссии инструкций.
    pub const_fold: bool,
    /// Позднее распространение и свёртка констант над готовым байт-кодом.
    pub const_prop: bool,
    /// Устранение доказанно мёртвых копий.
    pub copy_elim: bool,
}

impl Optimizations {
    /// Все проходы включены.
    #[must_use]
    pub fn all() -> Self {
        Self {
            const_fold: true,
            const_prop: true,
            copy_elim: true,
        }
    }
}

pub fn compile_program(resolved: &ResolvedProgram) -> Result<Program, CompileError> {
    compile_program_with(resolved, Optimizations::default())
}

/// То же, но с явно выбранными проходами.
///
/// # Errors
///
/// Те же отказы кодогена, что и у [`compile_program`].
pub fn compile_program_with(
    resolved: &ResolvedProgram,
    opts: Optimizations,
) -> Result<Program, CompileError> {
    let mut names = NameInterner::new();
    let mut shapes = ShapeTable::new();
    let chunks = compile_module_chunks(resolved, &mut names, &mut shapes, opts)?;
    Ok(assemble_program(
        resolved,
        chunks,
        names.into_names(),
        shapes.into_shapes(),
    ))
}

/// Компилирует каталог конфигурации: модули в порядке манифеста и, если
/// есть, entry. Все программы делят ОДНО пространство имён и форм —
/// значения пересекают границы модулей вместе со своими `NameId`, и
/// разные интернеры разнесли бы одно написание по разным номерам. Каждая
/// программа уносит полную финальную копию общих таблиц.
///
/// # Errors
///
/// Ошибка компиляции первого сбойного модуля.
pub fn compile_configuration(
    modules: &[(String, &ResolvedProgram)],
    entry: Option<&ResolvedProgram>,
    opts: Optimizations,
) -> Result<(bsl_bytecode::ConfigurationProgram, Option<Program>), CompileError> {
    let mut names = NameInterner::new();
    let mut shapes = ShapeTable::new();
    let mut compiled = Vec::with_capacity(modules.len());
    for (_, resolved) in modules {
        compiled.push(compile_module_chunks(
            resolved,
            &mut names,
            &mut shapes,
            opts,
        )?);
    }
    let entry_chunks = match entry {
        Some(resolved) => Some(compile_module_chunks(
            resolved,
            &mut names,
            &mut shapes,
            opts,
        )?),
        None => None,
    };
    let final_names = names.into_names();
    let final_shapes = shapes.into_shapes();
    let catalog = bsl_bytecode::ConfigurationProgram {
        modules: modules
            .iter()
            .zip(compiled)
            .map(|((name, resolved), chunks)| bsl_bytecode::ModuleProgram {
                name: name.clone(),
                program: assemble_program(
                    resolved,
                    chunks,
                    final_names.clone(),
                    final_shapes.clone(),
                ),
            })
            .collect(),
    };
    let entry_program = entry.zip(entry_chunks).map(|(resolved, chunks)| {
        assemble_program(resolved, chunks, final_names.clone(), final_shapes)
    });
    Ok((catalog, entry_program))
}

/// Компилирует transient entry поверх уже собранного каталога: таблицы
/// имён и форм entry начинаются с каталожных, поэтому `NameId` каталога
/// остаются валидными в его программе (правило то же, что у динамических
/// фрагментов — префикс плюс новые имена).
///
/// # Errors
///
/// Ошибка генерации байт-кода entry.
pub fn compile_entry_program(
    resolved: &ResolvedProgram,
    base_names: &[String],
    base_shapes: &[std::rc::Rc<bsl_rt::Shape>],
    opts: Optimizations,
) -> Result<Program, CompileError> {
    let mut names = NameInterner::from_existing(base_names.to_vec());
    let mut shapes = ShapeTable::from_existing(base_shapes.to_vec());
    let chunks = compile_module_chunks(resolved, &mut names, &mut shapes, opts)?;
    Ok(assemble_program(
        resolved,
        chunks,
        names.into_names(),
        shapes.into_shapes(),
    ))
}

fn compile_module_chunks(
    resolved: &ResolvedProgram,
    names: &mut NameInterner,
    shapes: &mut ShapeTable,
    opts: Optimizations,
) -> Result<Vec<Chunk>, CompileError> {
    let mut chunks = Vec::with_capacity(resolved.functions.len() + 1);
    chunks.push(compile_chunk(
        &resolved.top_level.locals,
        &[],
        &resolved.top_level.body,
        &resolved.functions,
        &[],
        &resolved.requirements,
        resolved.top_level.uses_dynamic,
        false,
        false,
        names,
        shapes,
        opts,
    )?);
    for f in &resolved.functions {
        chunks.push(compile_chunk(
            &f.locals,
            &f.params,
            &f.body,
            &resolved.functions,
            &[],
            &resolved.requirements,
            f.uses_dynamic,
            f.is_procedure,
            f.is_async,
            names,
            shapes,
            opts,
        )?);
    }
    // Инварианты графа проверяются на КАЖДОЙ компиляции в отладочной
    // сборке. Отдельный тест над корпусом покрывает лишь то, что
    // резолвится без реестра компонентов (сорок скриптов из восьмидесяти
    // шести), а здесь через проверку проходит всё, что вообще
    // компилируется, — включая фикстуры с компонентами и связанные
    // модули. В release-сборку это не попадает: анализ пока ничего не
    // решает, и платить за него на каждом запуске незачем.
    #[cfg(debug_assertions)]
    {
        let mut bodies: Vec<(&[RStmt], usize)> =
            vec![(&resolved.top_level.body, resolved.top_level.locals.len())];
        bodies.extend(
            resolved
                .functions
                .iter()
                .map(|f| (f.body.as_slice(), f.locals.len())),
        );
        for (body, n_slots) in bodies {
            let graph = crate::cfg::build(body);
            debug_assert!(
                crate::cfg::verify(&graph).is_ok(),
                "инвариант графа потока управления: {:?}",
                crate::cfg::verify(&graph)
            );
            let form = crate::ssa::build(&graph, n_slots);
            debug_assert!(
                crate::ssa::verify(&graph, &form).is_ok(),
                "инвариант SSA: {:?}",
                crate::ssa::verify(&graph, &form)
            );
        }
    }

    for (i, chunk) in chunks.iter_mut().enumerate() {
        let overlap = analysis::module_overlap(i, resolved.module_vars.len());
        // Оба прохода над ГОТОВЫМ байт-кодом выключены по умолчанию: пока
        // проход не прошёл свои ворота (чередующийся A/B на зафиксированной
        // частоте), он не должен попадать в обычную сборку — иначе не с чем
        // сравнивать. Свёртка в кодогене живёт не здесь, а в `compile_chunk`:
        // ей нужен литерал, а тут его уже нет.
        if opts.const_prop {
            analysis::const_propagate(chunk, overlap);
        }
        if opts.copy_elim {
            analysis::copy_propagate(chunk, overlap);
        }
        chunk.bundle_len = bundle::compute(chunk, overlap);
    }
    Ok(chunks)
}

fn assemble_program(
    resolved: &ResolvedProgram,
    chunks: Vec<Chunk>,
    names: Vec<String>,
    shapes: Vec<std::rc::Rc<bsl_rt::Shape>>,
) -> Program {
    Program {
        requirements: resolved.requirements.clone(),
        chunks,
        names,
        shapes,
        top_level_locals: resolved.top_level.locals.clone(),
        function_names: resolved.functions.iter().map(|f| f.name.clone()).collect(),
        exported_functions: resolved.functions.iter().map(|f| f.export).collect(),
        module_vars: resolved.module_vars.clone(),
        exported_module_vars: resolved.module_var_exports.clone(),
        module_base: 0,
        links: resolved
            .links
            .iter()
            .map(|link| match *link {
                bsl_sema::ResolvedLink::Function { module, func } => {
                    bsl_bytecode::LinkEntry::Function {
                        module: bsl_bytecode::ModuleId::new(module),
                        func,
                    }
                }
                bsl_sema::ResolvedLink::Variable { module, slot } => {
                    bsl_bytecode::LinkEntry::Variable {
                        module: bsl_bytecode::ModuleId::new(module),
                        slot,
                    }
                }
            })
            .collect(),
    }
}

pub use bsl_bytecode::SnippetUnit;

/// Компилирует фрагмент для `Выполнить`/`Вычислить`: `all_locals` — уже
/// расширенный список (существующие переменные верхнего уровня + новые,
/// объявленные во фрагменте, см. `bsl_sema::resolve_snippet_stmts`),
/// `program_names` — таблица имён полей ОСНОВНОЙ программы, которой нужно
/// ЗАСЕЯТЬ интернер фрагмента: иначе `GetProp`/`SetProp` на структуру,
/// созданную статическим кодом, получили бы другой `NameId` для того же
/// имени поля и не нашли бы его (см. `bsl-vm`). Вызовов пользовательских
/// процедур/функций фрагмент делать не может — `functions` пуст: снаружи
/// это `resolve_snippet_stmts`, у которой таблица сигнатур тоже всегда
/// пуста, так что `RExpr::Call` сюда прийти не может физически.
/// Возвращает чанк, полную (старую + новую) таблицу имён полей и полную
/// таблицу форм. Обе таблицы — новые, ЛОКАЛЬНЫЕ для этого фрагмента: имена
/// засеяны из `program_names` для согласованности `NameId` с уже
/// существующими объектами (см. модуль), но формы, в отличие от имён, у
/// фрагмента всегда СВОИ — `shape`-индексы внутри чанка ссылаются именно
/// на возвращаемый отсюда список, а не на какой-то внешний. Вызывающий
/// ОБЯЗАН передать именно ЭТОТ список форм в `Program`, с которым чанк
/// будет исполняться — иначе `NewStructure` попадёт по чужому индексу.
///
/// REPL сохраняет оба списка в сессии, чтобы объект, созданный одной
/// строкой, и обращение к его (новому) полю в следующей строке получили
/// одинаковый `NameId`/форму. `Выполнить`/`Вычислить` внутри уже
/// работающего скрипта их просто отбрасывает — та же логика, что и с
/// новыми локалями: нет материализованного кадра, чтобы сохранить между
/// вызовами.
///
/// # Errors
///
/// Возвращает [`CompileError`] при превышении лимитов байт-кода или при ссылке на
/// неизвестную функцию окружающей программы.
pub fn compile_snippet(
    all_locals: &[String],
    body: &[RStmt],
    program_names: &[String],
    callee_params: &[Vec<bool>],
) -> Result<SnippetUnit, CompileError> {
    compile_snippet_with_requirements(
        all_locals,
        body,
        program_names,
        callee_params,
        &[LibraryRequirement::bsl_rt()],
    )
}

/// Компилирует динамический фрагмент с его собственным замыканием
/// runtime-компонентов.
///
/// # Errors
///
/// Возвращает [`CompileError`] по тем же причинам, что
/// [`compile_snippet`], а также если выражение ссылается на библиотеку вне
/// `requirements`.
pub fn compile_snippet_with_requirements(
    all_locals: &[String],
    body: &[RStmt],
    program_names: &[String],
    callee_params: &[Vec<bool>],
    requirements: &[LibraryRequirement],
) -> Result<SnippetUnit, CompileError> {
    let mut names = NameInterner::new();
    for n in program_names {
        names.intern(n);
    }
    let mut shapes = ShapeTable::new();
    // Фрагмент всегда получает таблицу имён: он и сам может содержать
    // вложенный `Выполнить`, а стоимость на одноразовом чанке никакая.
    let chunk = compile_chunk(
        all_locals,
        &[],
        body,
        &[],
        callee_params,
        requirements,
        true,
        false,
        false,
        &mut names,
        &mut shapes,
        // ПРИНЯТОЕ РЕШЕНИЕ, а не следствие: фрагмент компилируется без
        // оптимизаций, какие бы проходы ни выбрал хост. Отсюда прямая
        // асимметрия — `--optimize` означает разные вещи для статического
        // байт-кода и для `Выполнить`/`Вычислить`, — и названа она здесь
        // именно потому, что сама собой не разумеется.
        //
        // Причина в том, что выбор проходов сюда нечем донести: фрагмент
        // компилируется через нейтральный контракт
        // `bsl_bytecode::DynamicCompiler`, который об `Optimizations` не
        // знает и знать не должен — он лежит в крейте представления.
        // Протащить выбор значило бы либо расширить контракт, либо завести
        // второй канал мимо него; и то и другое — отдельная работа с
        // отдельным обоснованием, а не довесок к свёртке констант.
        //
        // Заметить стоит и то, что снять асимметрию было бы небесполезно:
        // `Вычислить("2 + 3")` — предельный случай для свёртки, у него
        // литералами являются ВСЕ операнды.
        Optimizations::default(),
    )?;
    // Разметку VLIW-бандлов фрагмента считает НЕ здесь, а
    // `run_dynamic_snippet` в bsl-vm: только там известно, накладывается ли
    // модульный блок фрагмента на регистры кадра — у верхнего уровня
    // `module_base == 0` и накладывается, у вложенного `Выполнить` нет, — а
    // от этого зависит пересечение модульных слотов. Прежний расчёт с
    // `None` опирался на неверную посылку «у фрагмента `module_base != 0`».
    // Пустой `bundle_len` равнозначен поинструкционному исполнению и
    // безопасен до пересчёта.
    Ok(SnippetUnit {
        chunk,
        names: names.into_names(),
        shapes: shapes.into_shapes(),
    })
}

// Одиннадцать аргументов — это и есть весь входной контекст чанка;
// структура ради их упаковки ничего бы не объяснила.
#[allow(clippy::too_many_arguments)]
fn compile_chunk(
    locals: &[String],
    params: &[ResolvedParam],
    body: &[RStmt],
    functions: &[ResolvedFunction],
    callee_params: &[Vec<bool>],
    requirements: &[LibraryRequirement],
    materialize_locals: bool,
    is_procedure: bool,
    is_async: bool,
    names: &mut NameInterner,
    shapes: &mut ShapeTable,
    opts: Optimizations,
) -> Result<Chunk, CompileError> {
    let n_locals: u8 = locals
        .len()
        .try_into()
        .map_err(|_| CompileError::TooManyLocals)?;
    let n_params: u8 = params
        .len()
        .try_into()
        .map_err(|_| CompileError::TooManyLocals)?;
    let mut c = Compiler {
        instrs: Vec::new(),
        consts: Vec::new(),
        call_arg_modes: Vec::new(),
        exception_ranges: Vec::new(),
        next_reg: n_locals,
        max_reg: n_locals,
        loop_stack: Vec::new(),
        label_targets: Vec::new(),
        goto_patches: Vec::new(),
        functions,
        callee_params: callee_params.to_vec(),
        requirements,
        names,
        shapes,
        opts,
        unfoldable: std::collections::HashSet::default(),
    };
    c.compile_param_defaults(params)?;
    c.compile_block(body)?;
    c.patch_gotos()?;
    let prop_cache = c
        .instrs
        .iter()
        .map(|_| std::cell::RefCell::new(None))
        .collect();
    let method_cache = c
        .instrs
        .iter()
        .map(|_| std::cell::RefCell::new(None))
        .collect();
    Ok(Chunk {
        touches_objects: c.instrs.iter().any(Instr::touches_objects),
        param_by_val: params.iter().map(|p| p.by_val).collect(),
        param_has_default: params.iter().map(|p| p.default.is_some()).collect(),
        is_procedure,
        is_async,
        instrs: c.instrs,
        consts: c.consts,
        call_arg_modes: c.call_arg_modes,
        exception_ranges: c.exception_ranges,
        n_params,
        n_locals,
        n_regs: c.max_reg,
        local_names: if materialize_locals {
            locals.to_vec()
        } else {
            Vec::new()
        },
        prop_cache,
        method_cache,
        // Заполняется вызывающим: ширина бандлов зависит от места чанка в
        // программе (перекрытие модульных слотов с регистрами кадра 0),
        // которого здесь не видно. Пустой вектор — легальное состояние
        // «все бандлы одиночные».
        bundle_len: Vec::new(),
    })
}

/// Список прыжков `Прервать`/`Продолжить`, которые патчатся, когда становится
/// известен конец цикла (`Прервать`) или точка повтора (`Продолжить`) — для
/// Для `Для` точка повтора — это шаг инкремента, известный только после компиляции тела.
struct LoopCtx {
    break_patches: Vec<usize>,
    continue_patches: Vec<usize>,
}

// Заходы в `Compiler::fold_const` — только для внутрикрейтовых тестов
// сложности. Обычной сборке счётчик не достаётся.
//
// Доказывать линейность порогом времени значило бы завести тест, который
// краснеет от посторонней нагрузки на машине; заходы же считаются точно и
// одинаково при любой загрузке.
#[cfg(test)]
thread_local! {
    static FOLD_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Хеш для ключей-адресов в `Compiler::unfoldable`.
///
/// Ключи — адреса узлов дерева, а не пользовательские данные, поэтому
/// защита `SipHash` от подобранных коллизий здесь не нужна, а цена её
/// заметна: таблица трогается несколько раз на каждый узел выражения.
/// Умножение на нечётную константу с перемешиванием старших разрядов
/// (`fxhash`) распределяет выровненные адреса достаточно: на синтетике из
/// 4000 операторов замена вернула цену компиляции с +1,28 % к +0,51 %.
#[derive(Default, Clone, Copy)]
struct PtrHasher(u64);

impl std::hash::Hasher for PtrHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    // Ключ всегда `usize`, и `write_usize` ниже — единственный путь, каким
    // он сюда попадает. Общий байтовый путь оставлен корректным, но
    // медленным: им никто не пользуется.
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }

    fn write_usize(&mut self, n: usize) {
        let h = (n as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
        self.0 = h ^ (h >> 32);
    }
}

type BuildPtrHasher = std::hash::BuildHasherDefault<PtrHasher>;

struct Compiler<'a> {
    instrs: Vec<Instr>,
    consts: Vec<BslValue>,
    call_arg_modes: Vec<Vec<ArgMode>>,
    exception_ranges: Vec<ExceptionRange>,
    /// Вершина свободных регистров: параметры+локалы занимают
    /// `0..n_locals`, дальше — стек временных регистров, растущий/
    /// сжимающийся вокруг компиляции каждого подвыражения (тот же приём,
    /// что в компиляторе Lua).
    next_reg: u8,
    max_reg: u8,
    loop_stack: Vec<LoopCtx>,
    /// `LabelId` -> абсолютный `pc`. Сама метка инструкции не занимает.
    label_targets: Vec<Option<usize>>,
    /// Ещё не пропатченные `Instr::Jump`: индекс инструкции и целевая метка.
    goto_patches: Vec<(usize, LabelId)>,
    /// Сигнатуры всех функций модуля — нужны при компиляции вызова, чтобы
    /// решить режим передачи каждого аргумента (`Знач` смотрится у
    /// вызываемой функции, а не у самого вызова).
    functions: &'a [ResolvedFunction],
    /// Режимы параметров вызываемых функций по их номеру — заполняется
    /// только для фрагментов `Выполнить`, где `functions` пуст.
    callee_params: Vec<Vec<bool>>,
    requirements: &'a [LibraryRequirement],
    /// Общие на весь модуль — см. `compile_program`.
    names: &'a mut NameInterner,
    shapes: &'a mut ShapeTable,
    opts: Optimizations,
    /// Узлы, о которых уже доказано, что сворачивать в них нечего.
    /// См. `Compiler::fold_const` — без этой памяти обход квадратичен.
    unfoldable: std::collections::HashSet<usize, BuildPtrHasher>,
}

impl<'a> Compiler<'a> {
    fn alloc_temp(&mut self) -> Result<u8, CompileError> {
        let r = self.next_reg;
        let next = r.checked_add(1).ok_or(CompileError::TooManyRegisters)?;
        self.next_reg = next;
        if next > self.max_reg {
            self.max_reg = next;
        }
        Ok(r)
    }

    fn free_temp(&mut self, n: u8) {
        self.next_reg -= n;
    }

    fn add_const(&mut self, v: BslValue) -> Result<u16, CompileError> {
        let k = self.consts.len();
        let k: u16 = k.try_into().map_err(|_| CompileError::TooManyConstants)?;
        self.consts.push(v);
        Ok(k)
    }

    fn add_arg_modes(&mut self, modes: Vec<ArgMode>) -> Result<u16, CompileError> {
        let id = self.call_arg_modes.len();
        let id: u16 = id
            .try_into()
            .map_err(|_| CompileError::TooManyArgModeTables)?;
        self.call_arg_modes.push(modes);
        Ok(id)
    }

    /// Значение выражения, вычислимое целиком на компиляции, или `None`, если
    /// вычислить его нечем.
    ///
    /// Арифметика берётся та же, что исполняет VM (`BslValue::add` и соседи),
    /// поэтому свёрнутая константа равна тому, что дал бы прогон; второй
    /// редакции правил здесь нет. Ограничений ровно два, и оба обязательные:
    ///
    /// - **только числа.** Единственный лист свёртки — `RExpr::Number`, так
    ///   что до операций доходят одни числа. Приведение строк, дат и булевых
    ///   живёт в обёртках `bsl-vm`, и повторять его тут значило бы завести
    ///   второй экземпляр правил приведения;
    /// - **операция с ошибкой не сворачивается.** `1 / 0` обязано бросить на
    ///   исполнении — в том числе внутри `Попытка`, где исключение
    ///   перехватывают, — поэтому `Err` даёт `None`, и инструкция остаётся на
    ///   месте.
    ///
    /// Сравнения не сворачиваются: за ними стоит удаление недостижимых
    /// ветвей, а его в первой версии нет, и половинчатая свёртка условия
    /// оставила бы переход, читающий заведомо известный регистр.
    ///
    /// # Сложность
    ///
    /// Обход запоминает ОТКАЗЫ (`Compiler::unfoldable`), и без этого он
    /// квадратичен — не гипотетически, а измеренно. Рассуждение «глубину
    /// ограничивает лимит регистров кадра» верно только для бинарного пути:
    /// там каждый уровень занимает временный регистр, и рекурсия упирается в
    /// 255. У цепочки унарных минусов `- - - … - А` временный регистр не
    /// выделяется вовсе — операнд компилируется в тот же `dst`, — поэтому
    /// ничто её не ограничивает, и без памяти об отказах свёртка обходила бы
    /// остаток цепочки заново на каждом уровне. Замерено на глубинах 100,
    /// 200 и 400: накладные 0,13, 0,52 и 2,12 млн инструкций компиляции,
    /// то есть вчетверо на каждое удвоение.
    ///
    /// Успехи не запоминаются, и это не упущение: удачная свёртка немедленно
    /// выпускает `LoadConst` и обрывает рекурсию, поэтому свёрнутое поддерево
    /// обходится не более двух раз — один раз отказавшим предком, один раз
    /// собой. Повторяются только отказы, их и хватает запомнить.
    fn fold_const(&mut self, e: &RExpr) -> Option<BslValue> {
        #[cfg(test)]
        FOLD_VISITS.with(|c| c.set(c.get() + 1));
        // Адрес узла — его устойчивое имя: разрешённое дерево живёт
        // неизменным всю компиляцию, узлы не переезжают и не освобождаются.
        let key = std::ptr::from_ref(e) as usize;
        if self.unfoldable.contains(&key) {
            return None;
        }
        let folded = self.fold_uncached(e);
        if folded.is_none() {
            self.unfoldable.insert(key);
        }
        folded
    }

    /// Собственно свёртка, без обращения к памяти об отказах.
    ///
    /// Отдельно от [`Compiler::fold_const`] ради `?`: короткое замыкание
    /// здесь не стилистическое. Отказ левого операнда делает правый
    /// ненужным, а вычислить его — значит клонировать `BslNumber` с
    /// выделением памяти. Редакция, вычислявшая оба операнда всегда, стоила
    /// на синтетике из 4000 операторов +1,64 % инструкций компиляции против
    /// +0,18 % у этой.
    fn fold_uncached(&mut self, e: &RExpr) -> Option<BslValue> {
        match e {
            RExpr::Number(n) => Some(BslValue::Number(n.clone())),
            RExpr::Unary {
                op: UnaryOp::Neg,
                expr,
            } => self.fold_const(expr)?.neg().ok(),
            RExpr::Binary { op, lhs, rhs } => {
                let a = self.fold_const(lhs)?;
                let b = self.fold_const(rhs)?;
                match op {
                    BinaryOp::Add => a.add(&b).ok(),
                    BinaryOp::Sub => a.sub(&b).ok(),
                    BinaryOp::Mul => a.mul(&b).ok(),
                    BinaryOp::Div => a.div(&b).ok(),
                    BinaryOp::Mod => a.rem(&b).ok(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Заменить целиком вычислимое выражение одной готовой константой.
    /// `true` — инструкция уже выпущена, компилировать поддерево не нужно.
    ///
    /// # Errors
    ///
    /// [`CompileError::TooManyConstants`], если таблица констант чанка
    /// переполнена.
    fn emit_folded(&mut self, e: &RExpr, dst: u8) -> Result<bool, CompileError> {
        if !self.opts.const_fold {
            return Ok(false);
        }
        let Some(v) = self.fold_const(e) else {
            return Ok(false);
        };
        let k = self.add_const(v)?;
        self.emit(Instr::LoadConst { dst, k });
        Ok(true)
    }

    fn emit(&mut self, i: Instr) -> usize {
        self.instrs.push(i);
        self.instrs.len() - 1
    }

    /// Абсолютный индекс инструкции как цель перехода.
    ///
    /// Ширину `Instr` в восемь байт заодно не пересматриваем: она измерена,
    /// а расширение цели до `i32` её меняет. Отсюда предел — чанк не длиннее
    /// `i16::MAX` инструкций, и он честный, а не молчаливый.
    fn jump_target(target: usize) -> Result<i16, CompileError> {
        i16::try_from(target).map_err(|_| CompileError::JumpTargetOutOfRange)
    }

    fn patch_jump(&mut self, idx: usize, target: usize) -> Result<(), CompileError> {
        let target = Self::jump_target(target)?;
        match &mut self.instrs[idx] {
            Instr::Jump { target: t } => *t = target,
            Instr::JumpIfFalse { target: t, .. } => *t = target,
            Instr::JumpIfTrue { target: t, .. } => *t = target,
            Instr::JumpIfNotEqConst { target: t, .. } => *t = target,
            Instr::JumpIfNotLtConst { target: t, .. } => *t = target,
            Instr::JumpIfNotSkipped { target: t, .. } => *t = target,
            other => unreachable!("patch_jump on non-jump instruction: {other:?}"),
        }
        Ok(())
    }

    fn here(&self) -> usize {
        self.instrs.len()
    }

    /// Выпускает переход по ложному условию. Равенство локальной и литерала
    /// не требует ни временного регистра, ни отдельного булева значения.
    fn compile_jump_if_false(&mut self, cond: &RExpr) -> Result<usize, CompileError> {
        if let RExpr::Binary {
            op: BinaryOp::Eq,
            lhs,
            rhs,
        } = cond
        {
            let pair = match (&**lhs, &**rhs) {
                (RExpr::Local(src), literal) | (literal, RExpr::Local(src)) => {
                    condition_literal(literal).map(|value| (*src, value))
                }
                _ => None,
            };
            if let Some((src, value)) = pair {
                let k = self.add_const(value)?;
                return Ok(self.emit(Instr::JumpIfNotEqConst {
                    src: src as u8,
                    k,
                    target: 0,
                }));
            }
        }
        if let RExpr::Binary {
            op: BinaryOp::Lt,
            lhs,
            rhs,
        } = cond
            && let (RExpr::Local(src), RExpr::Number(number)) = (&**lhs, &**rhs)
        {
            let k = self.add_const(BslValue::Number(number.clone()))?;
            return Ok(self.emit(Instr::JumpIfNotLtConst {
                src: *src as u8,
                k,
                target: 0,
            }));
        }

        let reg = self.alloc_temp()?;
        self.compile_expr(cond, reg)?;
        self.free_temp(1);
        Ok(self.emit(Instr::JumpIfFalse {
            cond: reg,
            target: 0,
        }))
    }

    fn define_label(&mut self, id: LabelId) {
        let index = id.0 as usize;
        if self.label_targets.len() <= index {
            self.label_targets.resize(index + 1, None);
        }
        let target = self.here();
        let old = self.label_targets[index].replace(target);
        debug_assert!(old.is_none(), "дубль метки отсеян в sema");
    }

    fn patch_gotos(&mut self) -> Result<(), CompileError> {
        for (jump, label) in std::mem::take(&mut self.goto_patches) {
            let target = self
                .label_targets
                .get(label.0 as usize)
                .and_then(|target| *target)
                .expect("неизвестная метка отсеяна в sema");
            self.patch_jump(jump, target)?;
        }
        Ok(())
    }

    /// Пролог функции: для каждого параметра со значением по умолчанию —
    /// `JumpIfNotSkipped` вокруг кода, вычисляющего дефолт прямо в слот
    /// параметра. Порядок — по объявлению, поэтому дефолт вида `Ф(а, б =
    /// а + 1)` видит уже гарантированно установленный `а` (свой ли,
    /// пропущенный ли — его собственный `JumpIfNotSkipped` идёт раньше).
    ///
    /// Измерено на 8.3.27.2130: САМО такое объявление платформа не
    /// компилирует — умолчанием у неё может быть только литерал, и модуль
    /// с `б = а + 1` не собирается вовсе (проба ушла в таймаут с пустым
    /// выводом, то есть в несобравшийся модуль формы). Мы его принимаем;
    /// это разрешительное расхождение, и тест
    /// `skipped_call_argument_default_may_reference_earlier_parameter`
    /// закрепляет НАШЕ поведение, а не платформенное. Сужать это здесь
    /// без отдельной задачи нельзя: нужен измеренный текст ошибки.
    /// Обязательный параметр ПОСЛЕ необязательного платформа, наоборот,
    /// принимает — это измерено и лежит в фикстуре `default-args`.
    fn compile_param_defaults(&mut self, params: &[ResolvedParam]) -> Result<(), CompileError> {
        for (i, p) in params.iter().enumerate() {
            if let Some(default) = &p.default {
                let slot = i as u8;
                let j = self.emit(Instr::JumpIfNotSkipped {
                    src: slot,
                    target: 0,
                });
                self.compile_expr(default, slot)?;
                let end = self.here();
                self.patch_jump(j, end)?;
            }
        }
        Ok(())
    }

    // --- Выражения ------------------------------------------------------

    fn compile_expr(&mut self, e: &RExpr, dst: u8) -> Result<(), CompileError> {
        match e {
            RExpr::Number(n) => {
                let k = self.add_const(BslValue::Number(n.clone()))?;
                self.emit(Instr::LoadConst { dst, k });
            }
            RExpr::Date(d) => {
                let k = self.add_const(BslValue::Date(*d))?;
                self.emit(Instr::LoadConst { dst, k });
            }
            RExpr::Bool(val) => {
                self.emit(Instr::LoadBool { dst, val: *val });
            }
            RExpr::Undefined => {
                self.emit(Instr::LoadUndefined { dst });
            }
            RExpr::Null => {
                self.emit(Instr::LoadNull { dst });
            }
            RExpr::Await(expr) => {
                let promise = self.alloc_temp()?;
                self.compile_expr(expr, promise)?;
                self.emit(Instr::Await { dst, promise });
                self.free_temp(1);
            }
            RExpr::Local(slot) => {
                let src = *slot as u8;
                if src != dst {
                    self.emit(Instr::Move { dst, src });
                }
            }
            RExpr::ModuleVar(slot) => {
                self.emit(Instr::GetModuleVar {
                    dst,
                    slot: *slot as u16,
                });
            }
            RExpr::Unary { op, expr } => {
                if self.emit_folded(e, dst)? {
                    return Ok(());
                }
                self.compile_expr(expr, dst)?;
                match op {
                    UnaryOp::Neg => {
                        self.emit(Instr::Neg { dst, src: dst });
                    }
                    UnaryOp::Not => {
                        self.emit(Instr::Not { dst, src: dst });
                    }
                }
            }
            // `И`/`ИЛИ` короткозамкнутые в BSL: правый операнд не должен
            // вычисляться, если результат уже решён левым (защитный идиом
            // `ЗначениеЗаполнено(х) И х.Свойство = 1` иначе всегда падает).
            // Оба операнда компилируются прямо в `dst`, а не во временные
            // регистры: после первого `JumpIfFalse`/`JumpIfTrue` в `dst`
            // уже лежит финальный результат, если он решился на левом.
            //
            // Второй `JumpIfFalse`/`JumpIfTrue` (тоже прыгающий в `end`)
            // существует не ради ветвления — обе его ветки сходятся в одной
            // точке — а ради строгой проверки булевости правого операнда:
            // без него `Ложь ИЛИ 1` тихо вернул бы `1` вместо ошибки типа.
            // `И`/`ИЛИ` отдают БУЛЕВО, а не последний вычисленный операнд:
            // `1 И 1` на платформе даёт «Да», а не единицу (измерено,
            // `COND.AND_BOTH_NUMBERS`). Поэтому результат материализуется
            // `LoadBool`, а не остаётся в регистре от операнда. Короткое
            // замыкание при этом сохраняется: правый операнд не исполняется,
            // если левый уже решил исход.
            RExpr::Binary {
                op: BinaryOp::And,
                lhs,
                rhs,
            } => {
                self.compile_expr(lhs, dst)?;
                let short = self.emit(Instr::JumpIfFalse {
                    cond: dst,
                    target: 0,
                });
                self.compile_expr(rhs, dst)?;
                let checked = self.emit(Instr::JumpIfFalse {
                    cond: dst,
                    target: 0,
                });
                self.emit(Instr::LoadBool { dst, val: true });
                let to_end = self.emit(Instr::Jump { target: 0 });
                let on_false = self.here();
                self.emit(Instr::LoadBool { dst, val: false });
                let end = self.here();
                self.patch_jump(short, on_false)?;
                self.patch_jump(checked, on_false)?;
                self.patch_jump(to_end, end)?;
            }
            RExpr::Binary {
                op: BinaryOp::Or,
                lhs,
                rhs,
            } => {
                self.compile_expr(lhs, dst)?;
                let short = self.emit(Instr::JumpIfTrue {
                    cond: dst,
                    target: 0,
                });
                self.compile_expr(rhs, dst)?;
                let checked = self.emit(Instr::JumpIfTrue {
                    cond: dst,
                    target: 0,
                });
                self.emit(Instr::LoadBool { dst, val: false });
                let to_end = self.emit(Instr::Jump { target: 0 });
                let on_true = self.here();
                self.emit(Instr::LoadBool { dst, val: true });
                let end = self.here();
                self.patch_jump(short, on_true)?;
                self.patch_jump(checked, on_true)?;
                self.patch_jump(to_end, end)?;
            }
            RExpr::Binary { op, lhs, rhs } => {
                // Свёртка идёт ПЕРЕД выбором формы операндов: у
                // `2 + 3` выбирать нечего, а `AddConst` ниже поглотил бы
                // литерал в опкод и оставил сложение на исполнение.
                if self.emit_folded(e, dst)? {
                    return Ok(());
                }
                // Для `локальная + число` константа остаётся в таблице
                // чанка: отдельный регистр и отдельный dispatch для её
                // загрузки не нужны. Обратную форму не переставляем —
                // сложение BSL зависит от типа левого операнда.
                if matches!(op, BinaryOp::Add)
                    && let (RExpr::Local(src), RExpr::Number(number)) = (&**lhs, &**rhs)
                {
                    let k = self.add_const(BslValue::Number(number.clone()))?;
                    self.emit(Instr::AddConst {
                        dst,
                        src: *src as u8,
                        k,
                    });
                    return Ok(());
                }
                // Числовой литерал не имеет побочных эффектов и не может
                // изменить соседнюю локальную переменную. Поэтому локальную
                // читаем прямо из её слота, а временный регистр нужен только
                // литералу. Порядок операндов сохраняется: для вычитания,
                // деления и сравнений он существенен.
                if let (RExpr::Local(a), RExpr::Number(_)) = (&**lhs, &**rhs) {
                    let b = self.alloc_temp()?;
                    self.compile_expr(rhs, b)?;
                    self.emit(binop_instr(*op, dst, *a as u8, b));
                    self.free_temp(1);
                    return Ok(());
                }
                if let (RExpr::Number(_), RExpr::Local(b)) = (&**lhs, &**rhs) {
                    let a = self.alloc_temp()?;
                    self.compile_expr(lhs, a)?;
                    self.emit(binop_instr(*op, dst, a, *b as u8));
                    self.free_temp(1);
                    return Ok(());
                }
                // Оба операнда — просто переменные: копировать их во
                // временные регистры незачем, инструкция читает любой.
                //
                // Условие именно на ОБА, а не на каждый по отдельности.
                // Копия снимает значение операнда ДО вычисления соседнего,
                // и если бы соседний мог что-то изменить (вызов функции,
                // меняющий эту же переменную через параметр по ссылке или
                // через область модуля), то чтение без копии сдвинулось бы
                // во времени и дало другой результат. Две переменные
                // изменить друг друга не могут.
                //
                // Заодно это включает дописывание строки на месте: у
                // `Текст = Текст + Кусок` приёмник и левый операнд теперь
                // ОДИН регистр, а по этому признаку VM забирает значение
                // из регистра во владение (см. `Instr::Add`).
                if let (RExpr::Local(a), RExpr::Local(b)) = (&**lhs, &**rhs) {
                    self.emit(binop_instr(*op, dst, *a as u8, *b as u8));
                    return Ok(());
                }
                let a = self.alloc_temp()?;
                self.compile_expr(lhs, a)?;
                let b = self.alloc_temp()?;
                self.compile_expr(rhs, b)?;
                self.emit(binop_instr(*op, dst, a, b));
                self.free_temp(2);
            }
            RExpr::Call { func, args } => {
                self.compile_call(*func, args, dst)?;
            }
            RExpr::CallImported {
                link,
                param_by_val,
                args,
            } => {
                self.compile_call_imported(*link, param_by_val, args, dst)?;
            }
            RExpr::ImportedVar(link) => {
                let link_slot =
                    u16::try_from(*link).map_err(|_| CompileError::TooManyModuleVars)?;
                self.emit(Instr::GetImportedVar { dst, link_slot });
            }
            RExpr::CallBuiltinFn { builtin, args } => {
                let base = self.next_reg;
                for a in args {
                    let r = self.alloc_temp()?;
                    self.compile_expr(a, r)?;
                }
                let count: u8 = args
                    .len()
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.free_temp(count);
                self.emit(Instr::CallBuiltin {
                    dst,
                    builtin: *builtin,
                    base,
                    count,
                });
            }
            RExpr::CallComponent {
                library,
                function,
                args,
                ..
            } => {
                let base = self.next_reg;
                for arg in args {
                    let register = self.alloc_temp()?;
                    self.compile_expr(arg, register)?;
                }
                let count: u8 = args
                    .len()
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.free_temp(count);
                let library_index = self
                    .requirements
                    .iter()
                    .position(|requirement| requirement.package == library.as_str())
                    .ok_or_else(|| CompileError::UnknownLibrary(library.as_str().to_string()))?;
                let library: u8 = library_index
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.emit(Instr::CallComponent {
                    dst,
                    library,
                    function: function.get(),
                    base,
                    count,
                });
            }
            RExpr::CallMethod {
                obj,
                method,
                open,
                args,
            } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let base = self.next_reg;
                for a in args {
                    let r = self.alloc_temp()?;
                    self.compile_expr(a, r)?;
                }
                let count: u8 = args
                    .len()
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.free_temp(count);
                let builtin = bsl_rt::BuiltinMethod::lookup(method);
                if *open {
                    let method: u16 = self
                        .names
                        .intern(method)
                        .index()
                        .try_into()
                        .map_err(|_| CompileError::TooManyNames)?;
                    self.emit(Instr::CallObjectMethod {
                        dst,
                        obj: o,
                        method,
                        base,
                        count,
                    });
                } else {
                    match (builtin, args.len()) {
                        (Some(method), _) => {
                            self.emit(Instr::CallMethod {
                                dst,
                                obj: o,
                                method,
                                base,
                                count,
                            });
                        }
                        (None, _) => {
                            let method: u16 = self
                                .names
                                .intern(method)
                                .index()
                                .try_into()
                                .map_err(|_| CompileError::TooManyNames)?;
                            self.emit(Instr::CallObjectMethod {
                                dst,
                                obj: o,
                                method,
                                base,
                                count,
                            });
                        }
                    }
                }
                self.free_temp(1);
            }
            RExpr::Str(s) => {
                let k = self.add_const(BslValue::Str(bsl_rt::BslString::from_str(s)))?;
                self.emit(Instr::LoadConst { dst, k });
            }
            RExpr::Index { obj, index } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let i = self.alloc_temp()?;
                self.compile_expr(index, i)?;
                self.emit(Instr::GetIndex {
                    dst,
                    obj: o,
                    idx: i,
                });
                self.free_temp(2);
            }
            RExpr::Field { obj, name } => {
                // Свойство всегда закрытое — обоснование у резолвера, в
                // `AExpr::Field`. Открытые `GetObjectProp`/`SetObjectProp`
                // компилятор не выпускает вовсе; они живут в формате ради
                // уже сериализованного байт-кода, поэтому их читает
                // `text.rs` и исполняют интерпретатор и JIT.
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let name = self.names.intern(name);
                self.emit(Instr::GetProp { dst, obj: o, name });
                self.free_temp(1);
            }
            RExpr::NewArray { dims } => {
                let base = self.next_reg;
                for d in dims {
                    let r = self.alloc_temp()?;
                    self.compile_expr(d, r)?;
                }
                let count: u8 = dims
                    .len()
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.free_temp(count);
                self.emit(Instr::NewArray { dst, base, count });
            }
            RExpr::NewStructure { keys, values } => {
                let name_ids: Vec<bsl_rt::NameId> =
                    keys.iter().map(|k| self.names.intern(k)).collect();
                let shape_id: u16 = self
                    .shapes
                    .intern(&name_ids)
                    .try_into()
                    .map_err(|_| CompileError::TooManyShapes)?;
                let base = self.next_reg;
                for v in values {
                    let r = self.alloc_temp()?;
                    self.compile_expr(v, r)?;
                }
                let count: u8 = values
                    .len()
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.free_temp(count);
                self.emit(Instr::NewStructure {
                    dst,
                    shape: shape_id,
                    base,
                    count,
                });
            }
            RExpr::CreateObject {
                library,
                constructor,
                args,
            } => {
                let base = self.next_reg;
                for argument in args {
                    let register = self.alloc_temp()?;
                    self.compile_expr(argument, register)?;
                }
                let count: u8 = args
                    .len()
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.free_temp(count);
                let library_index = self
                    .requirements
                    .iter()
                    .position(|requirement| requirement.package == library.as_str())
                    .ok_or_else(|| CompileError::UnknownLibrary(library.as_str().to_string()))?;
                let library: u8 = library_index
                    .try_into()
                    .map_err(|_| CompileError::TooManyRegisters)?;
                self.emit(Instr::CreateObject {
                    dst,
                    library,
                    constructor: constructor.get(),
                    base,
                    count,
                });
            }
            RExpr::NewTable => {
                self.emit(Instr::NewTable { dst });
            }
            RExpr::NewTypeDescription(names) => {
                let names_reg = self.alloc_temp()?;
                self.compile_expr(names, names_reg)?;
                self.emit(Instr::NewTypeDescription {
                    dst,
                    names: names_reg,
                });
                self.free_temp(1);
            }
            RExpr::NewValueComparison => {
                self.emit(Instr::NewValueComparison { dst });
            }
            RExpr::NewMap => {
                self.emit(Instr::NewMap { dst });
            }
            // Член перечисления — обычная константа чанка: значение
            // известно на этапе компиляции целиком.
            RExpr::EnumMember(v) => {
                let k = self.add_const(BslValue::Enum(*v))?;
                self.emit(Instr::LoadConst { dst, k });
            }
            // Голое имя перечисления — той же природы константа.
            RExpr::EnumTypeRef(k) => {
                let c = self.add_const(BslValue::EnumType(*k))?;
                self.emit(Instr::LoadConst { dst, k: c });
            }
            // Тернарный оператор — короткое замыкание, как `И`/`ИЛИ`, и
            // компилируется так же: переходами, а не тремя вычисленными
            // регистрами. Невыбранная ветвь ФИЗИЧЕСКИ не исполняется —
            // измерено на `?(Истина, "ок", 1 / Число("0"))`, где деление на
            // ноль не срабатывает. Отдельного опкода поэтому не нужно.
            RExpr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let to_else = self.compile_jump_if_false(cond)?;
                self.compile_expr(then_expr, dst)?;
                let to_end = self.emit(Instr::Jump { target: 0 });
                let else_at = self.here();
                self.compile_expr(else_expr, dst)?;
                let end = self.here();
                self.patch_jump(to_else, else_at)?;
                self.patch_jump(to_end, end)?;
            }
            RExpr::NewTextWriter { path } => {
                let path_reg = self.alloc_temp()?;
                self.compile_expr(path, path_reg)?;
                self.emit(Instr::NewTextWriter {
                    dst,
                    path: path_reg,
                });
                self.free_temp(1);
            }
            RExpr::DynEval(e) => {
                let s = self.alloc_temp()?;
                self.compile_expr(e, s)?;
                self.emit(Instr::RunDynamic {
                    src: s,
                    dst,
                    is_eval: true,
                });
                self.free_temp(1);
            }
        }
        Ok(())
    }

    /// Аргументы без `Знач`, чья форма на месте вызова — голая локальная
    /// переменная, передаются по ссылке (алиас на слот вызывающего, без
    /// материализации значения); всё остальное — обычным значением в
    /// регистре `base + i`. Само решение "по ссылке или нет" статично и
    /// целиком принимается здесь, в компиляторе.
    fn compile_call(
        &mut self,
        func: u32,
        args: &[ResolvedArg],
        dst: u8,
    ) -> Result<(), CompileError> {
        // Индекс приходит из резолвинга, но во ФРАГМЕНТЕ (`Выполнить`) он
        // указывает на функцию окружающей программы, которой у компилятора
        // фрагмента в `functions` нет. Поэтому режимы параметров берутся из
        // отдельной таблицы, а выход за её границу — ошибка компиляции, не
        // паника: текст фрагмента приходит из рантайма.
        let by_val: Vec<bool> = match self.functions.get(func as usize) {
            Some(f) => f.params.iter().map(|p| p.by_val).collect(),
            None => self
                .callee_params
                .get(func as usize)
                .cloned()
                .ok_or(CompileError::UnknownFunction)?,
        };
        let base = self.next_reg;
        let mut modes = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let arg = match arg {
                ResolvedArg::Value(e) => e,
                // Пропущенная позиция: считать нечего, но временный
                // регистр всё равно занимаем — диапазон [base,base+argc)
                // обязан остаться непрерывным, а слот параметра, на
                // который он превратится, заполнит пролог умолчаний
                // вызванной функции.
                ResolvedArg::Default => {
                    self.alloc_temp()?;
                    modes.push(ArgMode::Default);
                    continue;
                }
            };
            let by_val = *by_val.get(i).unwrap_or(&true);
            if !by_val && let RExpr::Local(slot) = arg {
                self.alloc_temp()?; // держим диапазон [base,base+argc) непрерывным
                modes.push(ArgMode::ByRefLocal(*slot as u8));
                continue;
            }
            // ИЗМЕРЕНО(CALL.BYREF.MODULEVAR): 8.3.27 передаёт модульную
            // переменную в параметр без `Знач` ПО ССЫЛКЕ и из тела модуля, и
            // изнутри процедуры (`изменено|изменено`). Прежде образец ловил
            // только `RExpr::Local`, и `RExpr::ModuleVar` уходил копией
            // (`ArgMode::Value`) — второй случай расходился с платформой.
            if !by_val && let RExpr::ModuleVar(slot) = arg {
                self.alloc_temp()?; // держим диапазон [base,base+argc) непрерывным
                let slot = u16::try_from(*slot).map_err(|_| {
                    // Слот модульной переменной шире 16 бит — тот же класс,
                    // что чинил этап 1 предыдущего плана: `as` молча обрезал бы.
                    CompileError::TooManyModuleVars
                })?;
                modes.push(ArgMode::ByRefModuleVar(slot));
                continue;
            }
            let r = self.alloc_temp()?;
            self.compile_expr(arg, r)?;
            modes.push(ArgMode::Value);
        }
        let argc: u8 = args
            .len()
            .try_into()
            .map_err(|_| CompileError::TooManyRegisters)?;
        self.free_temp(argc);
        let arg_modes = self.add_arg_modes(modes)?;
        let func_chunk: u16 = (func + 1)
            .try_into()
            .map_err(|_| CompileError::TooManyRegisters)?;
        self.emit(Instr::Call {
            func: func_chunk,
            base,
            arg_modes,
            ret: dst,
        });
        Ok(())
    }

    /// Межмодульный вызов: режимы аргументов строятся по `param_by_val`
    /// целевой функции, снятому резолвером с её модуля, — чужой
    /// `ResolvedProgram` компилятору не виден. Импортированная переменная
    /// без `Знач` уходит по ссылке своим режимом `byimport`.
    fn compile_call_imported(
        &mut self,
        link: u32,
        param_by_val: &[bool],
        args: &[ResolvedArg],
        dst: u8,
    ) -> Result<(), CompileError> {
        let base = self.next_reg;
        let mut modes = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let arg = match arg {
                ResolvedArg::Value(e) => e,
                ResolvedArg::Default => {
                    self.alloc_temp()?;
                    modes.push(ArgMode::Default);
                    continue;
                }
            };
            let by_val = *param_by_val.get(i).unwrap_or(&true);
            if !by_val && let RExpr::Local(slot) = arg {
                self.alloc_temp()?;
                modes.push(ArgMode::ByRefLocal(*slot as u8));
                continue;
            }
            if !by_val && let RExpr::ModuleVar(slot) = arg {
                self.alloc_temp()?;
                let slot = u16::try_from(*slot).map_err(|_| CompileError::TooManyModuleVars)?;
                modes.push(ArgMode::ByRefModuleVar(slot));
                continue;
            }
            if !by_val && let RExpr::ImportedVar(var_link) = arg {
                self.alloc_temp()?;
                let var_link =
                    u16::try_from(*var_link).map_err(|_| CompileError::TooManyModuleVars)?;
                modes.push(ArgMode::ByRefImportedVar(var_link));
                continue;
            }
            let r = self.alloc_temp()?;
            self.compile_expr(arg, r)?;
            modes.push(ArgMode::Value);
        }
        let argc: u8 = args
            .len()
            .try_into()
            .map_err(|_| CompileError::TooManyRegisters)?;
        self.free_temp(argc);
        let arg_modes = self.add_arg_modes(modes)?;
        let link_slot: u16 = link
            .try_into()
            .map_err(|_| CompileError::TooManyModuleVars)?;
        self.emit(Instr::CallImported {
            link_slot,
            base,
            arg_modes,
            ret: dst,
        });
        Ok(())
    }

    // --- Операторы --------------------------------------------------------

    fn compile_block(&mut self, stmts: &[RStmt]) -> Result<(), CompileError> {
        for s in stmts {
            self.compile_stmt(s)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, s: &RStmt) -> Result<(), CompileError> {
        match s {
            RStmt::AssignLocal { slot, value } => {
                self.compile_expr(value, *slot as u8)?;
            }
            RStmt::AssignModuleVar { slot, value } => {
                // Через временный регистр: у модульной переменной нет
                // регистра в этом кадре, писать прямо некуда.
                let v = self.alloc_temp()?;
                self.compile_expr(value, v)?;
                self.emit(Instr::SetModuleVar {
                    slot: *slot as u16,
                    src: v,
                });
                self.free_temp(1);
            }
            RStmt::AssignImportedVar { link, value } => {
                // Как модульная, только слот живёт в чужом модуле и
                // адресуется записью таблицы связей.
                let v = self.alloc_temp()?;
                self.compile_expr(value, v)?;
                let link_slot =
                    u16::try_from(*link).map_err(|_| CompileError::TooManyModuleVars)?;
                self.emit(Instr::SetImportedVar { link_slot, src: v });
                self.free_temp(1);
            }
            RStmt::AssignIndex { obj, index, value } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let i = self.alloc_temp()?;
                self.compile_expr(index, i)?;
                let v = self.alloc_temp()?;
                self.compile_expr(value, v)?;
                self.emit(Instr::SetIndex {
                    obj: o,
                    idx: i,
                    src: v,
                });
                self.free_temp(3);
            }
            RStmt::AssignField { obj, name, value } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let v = self.alloc_temp()?;
                self.compile_expr(value, v)?;
                let name = self.names.intern(name);
                self.emit(Instr::SetProp {
                    obj: o,
                    name,
                    src: v,
                });
                self.free_temp(2);
            }
            RStmt::ExprStmt(e) => {
                // Результат вызова-как-оператора отбрасывается, но регистр
                // под него всё равно нужен на время компиляции выражения.
                let r = self.alloc_temp()?;
                self.compile_expr(e, r)?;
                self.free_temp(1);
            }
            RStmt::Return(opt) => match opt {
                Some(e) => {
                    let r = self.alloc_temp()?;
                    self.compile_expr(e, r)?;
                    self.emit(Instr::Return { src: Some(r) });
                    self.free_temp(1);
                }
                None => {
                    self.emit(Instr::Return { src: None });
                }
            },
            RStmt::Label(id) => self.define_label(*id),
            RStmt::Goto(id) => {
                let jump = self.emit(Instr::Jump { target: 0 });
                self.goto_patches.push((jump, *id));
            }
            RStmt::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let mut end_patches = Vec::new();

                let mut jf = self.compile_jump_if_false(cond)?;
                self.compile_block(then_branch)?;
                end_patches.push(self.emit(Instr::Jump { target: 0 }));

                for (c, body) in elsif_branches {
                    self.patch_jump(jf, self.here())?;
                    jf = self.compile_jump_if_false(c)?;
                    self.compile_block(body)?;
                    end_patches.push(self.emit(Instr::Jump { target: 0 }));
                }

                self.patch_jump(jf, self.here())?;
                if let Some(else_body) = else_branch {
                    self.compile_block(else_body)?;
                }

                let end = self.here();
                for p in end_patches {
                    self.patch_jump(p, end)?;
                }
            }
            RStmt::While { cond, body } => {
                let cond_pc = self.here();
                let jf = self.compile_jump_if_false(cond)?;

                self.loop_stack.push(LoopCtx {
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });
                self.compile_block(body)?;
                self.emit(Instr::Jump {
                    target: Self::jump_target(cond_pc)?,
                });

                let end = self.here();
                self.patch_jump(jf, end)?;
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches {
                    self.patch_jump(p, end)?;
                }
                // `Продолжить` в `Пока` — это "перепроверить условие".
                for p in ctx.continue_patches {
                    self.patch_jump(p, cond_pc)?;
                }
            }
            RStmt::ForNumeric {
                slot,
                from,
                to,
                body,
            } => {
                let slot = *slot as u8;
                // Границы вычисляются один раз: `from` кладём прямо в
                // регистр переменной цикла, `to` — в отдельный регистр,
                // живущий на протяжении всего цикла.
                self.compile_expr(from, slot)?;
                let bound = self.alloc_temp()?;
                self.compile_expr(to, bound)?;

                let cmp = self.alloc_temp()?;
                self.emit(Instr::Le {
                    dst: cmp,
                    a: slot,
                    b: bound,
                });
                self.free_temp(1);
                let jf = self.emit(Instr::JumpIfFalse {
                    cond: cmp,
                    target: 0,
                });

                self.loop_stack.push(LoopCtx {
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });
                let body_pc = self.here();
                self.compile_block(body)?;

                let incr_pc = self.here();
                if body.is_empty() {
                    self.emit(Instr::NumericForNextI64 {
                        counter: slot,
                        bound,
                        target: Self::jump_target(body_pc)?,
                    });
                } else {
                    self.emit(Instr::NumericForNext {
                        counter: slot,
                        bound,
                        target: Self::jump_target(body_pc)?,
                    });
                }

                let end = self.here();
                self.patch_jump(jf, end)?;
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches {
                    self.patch_jump(p, end)?;
                }
                for p in ctx.continue_patches {
                    self.patch_jump(p, incr_pc)?;
                }

                self.free_temp(1); // bound
            }
            RStmt::ForEach { slot, iter, body } => {
                let slot = *slot as u8;
                // Коллекция вычисляется один раз, как и границы `Для`.
                let iter_reg = self.alloc_temp()?;
                self.compile_expr(iter, iter_reg)?;
                let len_reg = self.alloc_temp()?;
                self.emit(Instr::CollectionLen {
                    dst: len_reg,
                    obj: iter_reg,
                });
                let idx_reg = self.alloc_temp()?;
                let zero_k =
                    self.add_const(BslValue::Number(bsl_number::BslNumber::from_i64(0)))?;
                self.emit(Instr::LoadConst {
                    dst: idx_reg,
                    k: zero_k,
                });

                let cond_pc = self.here();
                let cmp = self.alloc_temp()?;
                self.emit(Instr::Lt {
                    dst: cmp,
                    a: idx_reg,
                    b: len_reg,
                });
                self.free_temp(1);
                let jf = self.emit(Instr::JumpIfFalse {
                    cond: cmp,
                    target: 0,
                });

                self.emit(Instr::GetIndex {
                    dst: slot,
                    obj: iter_reg,
                    idx: idx_reg,
                });

                self.loop_stack.push(LoopCtx {
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });
                self.compile_block(body)?;

                let incr_pc = self.here();
                let one = self.alloc_temp()?;
                let one_k = self.add_const(BslValue::Number(bsl_number::BslNumber::from_i64(1)))?;
                self.emit(Instr::LoadConst { dst: one, k: one_k });
                self.emit(Instr::Add {
                    dst: idx_reg,
                    a: idx_reg,
                    b: one,
                });
                self.free_temp(1);
                self.emit(Instr::Jump {
                    target: Self::jump_target(cond_pc)?,
                });

                let end = self.here();
                self.patch_jump(jf, end)?;
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches {
                    self.patch_jump(p, end)?;
                }
                for p in ctx.continue_patches {
                    self.patch_jump(p, incr_pc)?;
                }

                self.free_temp(3); // iter_reg, len_reg, idx_reg
            }
            RStmt::Try { body, except_body } => {
                let start = self.here();
                self.compile_block(body)?;
                let end = self.here();
                // Тело завершилось без исключения — обработчик пропускаем.
                let skip_handler = self.emit(Instr::Jump { target: 0 });

                let handler_pc = self.here();
                self.compile_block(except_body)?;

                let after = self.here();
                self.patch_jump(skip_handler, after)?;
                self.exception_ranges.push(ExceptionRange {
                    start_pc: start,
                    end_pc: end,
                    handler_pc,
                });
            }
            RStmt::Raise(opt) => match opt {
                Some(e) => {
                    let r = self.alloc_temp()?;
                    self.compile_expr(e, r)?;
                    self.emit(Instr::Raise { src: Some(r) });
                    self.free_temp(1);
                }
                None => {
                    self.emit(Instr::Raise { src: None });
                }
            },
            RStmt::Execute(e) => {
                let s = self.alloc_temp()?;
                self.compile_expr(e, s)?;
                let d = self.alloc_temp()?; // результат отбрасывается, но регистр нужен
                self.emit(Instr::RunDynamic {
                    src: s,
                    dst: d,
                    is_eval: false,
                });
                self.free_temp(2);
            }
            RStmt::Break => {
                let idx = self.emit(Instr::Jump { target: 0 });
                self.loop_stack
                    .last_mut()
                    .ok_or(CompileError::BreakOutsideLoop)?
                    .break_patches
                    .push(idx);
            }
            RStmt::Continue => {
                let idx = self.emit(Instr::Jump { target: 0 });
                self.loop_stack
                    .last_mut()
                    .ok_or(CompileError::ContinueOutsideLoop)?
                    .continue_patches
                    .push(idx);
            }
        }
        Ok(())
    }
}

/// Литерал, который можно безопасно перенести в таблицу констант условного
/// перехода. Значение уже полностью известно и не имеет побочных эффектов.
fn condition_literal(expr: &RExpr) -> Option<BslValue> {
    match expr {
        RExpr::Number(number) => Some(BslValue::Number(number.clone())),
        RExpr::Date(date) => Some(BslValue::Date(*date)),
        RExpr::Bool(value) => Some(BslValue::Boolean(*value)),
        RExpr::Undefined => Some(BslValue::Undefined),
        RExpr::Null => Some(BslValue::Null),
        RExpr::Str(value) => Some(BslValue::Str(bsl_rt::BslString::from_str(value))),
        _ => None,
    }
}

fn binop_instr(op: BinaryOp, dst: u8, a: u8, b: u8) -> Instr {
    match op {
        BinaryOp::Add => Instr::Add { dst, a, b },
        BinaryOp::Sub => Instr::Sub { dst, a, b },
        BinaryOp::Mul => Instr::Mul { dst, a, b },
        BinaryOp::Div => Instr::Div { dst, a, b },
        BinaryOp::Mod => Instr::Mod { dst, a, b },
        BinaryOp::Eq => Instr::Eq { dst, a, b },
        BinaryOp::NotEq => Instr::NotEq { dst, a, b },
        BinaryOp::Lt => Instr::Lt { dst, a, b },
        BinaryOp::Gt => Instr::Gt { dst, a, b },
        BinaryOp::Le => Instr::Le { dst, a, b },
        BinaryOp::Ge => Instr::Ge { dst, a, b },
        // Перехватываются раньше в `compile_expr` (короткое замыкание) —
        // сюда никогда не доходят.
        BinaryOp::And | BinaryOp::Or => unreachable!("short-circuit ops handled in compile_expr"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Сколько раз свёртка зашла в `fold_const`, компилируя цепочку из
    /// `depth` унарных минусов над переменной.
    ///
    /// Цепочка именно унарная: у неё операнд компилируется в тот же
    /// регистр, `alloc_temp` не вызывается, и глубину ничто не
    /// ограничивает. На бинарной цепочке этот тест ничего бы не доказал —
    /// её обрывает лимит регистров кадра.
    ///
    /// Стек берётся с запасом: фронтенд рекурсивен, и двух мегабайт
    /// обычного тестового потока на такие глубины не хватает — предел
    /// принадлежит разбору с резолвингом, а не свёртке.
    fn fold_visits_at(depth: usize) -> usize {
        let src = format!("А = 1;\nБ = {}А;\n", "- ".repeat(depth));
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                let parsed = bsl_syntax::parse(&src).expect("разбор");
                let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
                FOLD_VISITS.with(|c| c.set(0));
                compile_program_with(
                    &resolved,
                    Optimizations {
                        const_fold: true,
                        ..Optimizations::default()
                    },
                )
                .expect("компиляция");
                FOLD_VISITS.with(std::cell::Cell::get)
            })
            .expect("поток")
            .join()
            .expect("компиляция цепочки")
    }

    /// Обход свёртки обязан быть линейным по размеру выражения.
    ///
    /// До появления памяти об отказах он был квадратичным: `fold_const`
    /// спускался по остатку цепочки заново на каждом уровне. Порог здесь
    /// не «около двух», а «меньше трёх» — линейному росту отвечает 2,
    /// квадратичному 4, и промахнуться между ними невозможно.
    #[test]
    fn folding_visits_grow_linearly_with_expression_depth() {
        let small = fold_visits_at(200);
        let large = fold_visits_at(400);

        assert!(small > 0, "счётчик заходов не работает");
        assert!(
            large < small * 3,
            "обход не линеен: {small} заходов на 200 уровнях и {large} на 400"
        );
    }

    /// Граница бинарной вложенности — та, о которой говорит документ:
    /// каждый уровень занимает временный регистр, кадр вмещает 255.
    /// Отказ обязан быть внятной ошибкой, а не молчаливой порчей.
    #[test]
    fn a_binary_chain_compiles_to_the_frame_limit_and_refuses_past_it() {
        let compile_chain = |depth: usize| {
            let src = format!("Б = 1;\nА = Б{};\n", " + 1".repeat(depth));
            std::thread::Builder::new()
                .stack_size(64 * 1024 * 1024)
                .spawn(move || {
                    let parsed = bsl_syntax::parse(&src).expect("разбор");
                    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
                    compile_program_with(
                        &resolved,
                        Optimizations {
                            const_fold: true,
                            ..Optimizations::default()
                        },
                    )
                    .map(|_| ())
                })
                .expect("поток")
                .join()
                .expect("компиляция цепочки")
        };

        assert!(
            compile_chain(253).is_ok(),
            "253 уровня обязаны компилироваться"
        );
        assert!(
            matches!(compile_chain(260), Err(CompileError::TooManyRegisters)),
            "на 260 уровнях ожидался отказ по регистрам кадра"
        );
    }
}
