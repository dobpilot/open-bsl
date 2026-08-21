use bsl_rt::{BslValue, NameInterner, ShapeTable};
use bsl_sema::{RExpr, RStmt, ResolvedFunction, ResolvedParam, ResolvedProgram};
use bsl_syntax::{BinaryOp, UnaryOp};

use crate::chunk::{Chunk, ExceptionRange, Program};
use crate::instr::{ArgMode, Instr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    TooManyLocals,
    TooManyRegisters,
    TooManyConstants,
    TooManyArgModeTables,
    TooManyShapes,
    TooManyNames,
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
}

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
pub fn compile_program(resolved: &ResolvedProgram) -> Result<Program, CompileError> {
    let mut names = NameInterner::new();
    let mut shapes = ShapeTable::new();
    let mut chunks = Vec::with_capacity(resolved.functions.len() + 1);
    chunks.push(compile_chunk(
        &resolved.top_level.locals,
        &[],
        &resolved.top_level.body,
        &resolved.functions,
        &[],
        &resolved.requirements,
        resolved.top_level.uses_dynamic,
        &mut names,
        &mut shapes,
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
            &mut names,
            &mut shapes,
        )?);
    }
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.bundle_len = crate::bundle::compute(
            chunk,
            crate::bundle::module_overlap(i, resolved.module_vars.len()),
        );
    }
    Ok(Program {
        requirements: resolved.requirements.clone(),
        chunks,
        names: names.into_names(),
        shapes: shapes.into_shapes(),
        top_level_locals: resolved.top_level.locals.clone(),
        function_names: resolved.functions.iter().map(|f| f.name.clone()).collect(),
        module_vars: resolved.module_vars.clone(),
        module_base: 0,
    })
}

/// Результат компиляции фрагмента: чанк, полная таблица имён полей и
/// полная таблица форм (подробности — в doc comment `compile_snippet`).
pub type SnippetOutput = (Chunk, Vec<String>, Vec<std::rc::Rc<bsl_rt::Shape>>);

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
) -> Result<SnippetOutput, CompileError> {
    compile_snippet_with_requirements(
        all_locals,
        body,
        program_names,
        callee_params,
        &[crate::LibraryRequirement::bsl_rt()],
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
    requirements: &[crate::LibraryRequirement],
) -> Result<SnippetOutput, CompileError> {
    let mut names = NameInterner::new();
    for n in program_names {
        names.intern(n);
    }
    let mut shapes = ShapeTable::new();
    // Фрагмент всегда получает таблицу имён: он и сам может содержать
    // вложенный `Выполнить`, а стоимость на одноразовом чанке никакая.
    let mut chunk = compile_chunk(
        all_locals,
        &[],
        body,
        &[],
        callee_params,
        requirements,
        true,
        &mut names,
        &mut shapes,
    )?;
    // У фрагмента блок модульных переменных лежит ЗА его регистрами
    // (`module_base != 0`, см. `Program::module_base`), поэтому перекрытия
    // модульных слотов с регистрами кадра нет.
    chunk.bundle_len = crate::bundle::compute(&chunk, None);
    Ok((chunk, names.into_names(), shapes.into_shapes()))
}

// Девять аргументов — это и есть весь входной контекст чанка; структура
#[allow(clippy::too_many_arguments)]
fn compile_chunk(
    locals: &[String],
    params: &[ResolvedParam],
    body: &[RStmt],
    functions: &[ResolvedFunction],
    callee_params: &[Vec<bool>],
    requirements: &[crate::LibraryRequirement],
    materialize_locals: bool,
    names: &mut NameInterner,
    shapes: &mut ShapeTable,
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
        functions,
        callee_params: callee_params.to_vec(),
        requirements,
        names,
        shapes,
    };
    c.compile_param_defaults(params)?;
    c.compile_block(body)?;
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
        param_by_val: params.iter().map(|p| p.by_val).collect(),
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
    /// Сигнатуры всех функций модуля — нужны при компиляции вызова, чтобы
    /// решить режим передачи каждого аргумента (`Знач` смотрится у
    /// вызываемой функции, а не у самого вызова).
    functions: &'a [ResolvedFunction],
    /// Режимы параметров вызываемых функций по их номеру — заполняется
    /// только для фрагментов `Выполнить`, где `functions` пуст.
    callee_params: Vec<Vec<bool>>,
    requirements: &'a [crate::LibraryRequirement],
    /// Общие на весь модуль — см. `compile_program`.
    names: &'a mut NameInterner,
    shapes: &'a mut ShapeTable,
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

    fn emit(&mut self, i: Instr) -> usize {
        self.instrs.push(i);
        self.instrs.len() - 1
    }

    fn patch_jump(&mut self, idx: usize, target: usize) {
        let target = target as i16;
        match &mut self.instrs[idx] {
            Instr::Jump { target: t } => *t = target,
            Instr::JumpIfFalse { target: t, .. } => *t = target,
            Instr::JumpIfTrue { target: t, .. } => *t = target,
            Instr::JumpIfNotSkipped { target: t, .. } => *t = target,
            other => unreachable!("patch_jump on non-jump instruction: {other:?}"),
        }
    }

    fn here(&self) -> usize {
        self.instrs.len()
    }

    /// Пролог функции: для каждого параметра со значением по умолчанию —
    /// `JumpIfNotSkipped` вокруг кода, вычисляющего дефолт прямо в слот
    /// параметра. Порядок — по объявлению, поэтому дефолт вида `Ф(а, б =
    /// а + 1)` видит уже гарантированно установленный `а` (свой ли,
    /// пропущенный ли — его собственный `JumpIfNotSkipped` идёт раньше).
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
                self.patch_jump(j, end);
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
            RExpr::Skipped => {
                self.emit(Instr::LoadSkipped { dst });
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
                self.patch_jump(short, on_false);
                self.patch_jump(checked, on_false);
                self.patch_jump(to_end, end);
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
                self.patch_jump(short, on_true);
                self.patch_jump(checked, on_true);
                self.patch_jump(to_end, end);
            }
            RExpr::Binary { op, lhs, rhs } => {
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
                        (Some(bsl_rt::BuiltinMethod::Write), 1) => {
                            self.emit(Instr::WriteText {
                                dst,
                                obj: o,
                                src: base,
                            });
                        }
                        (Some(bsl_rt::BuiltinMethod::Close), 0) => {
                            self.emit(Instr::CloseText { dst, obj: o });
                        }
                        // Переходный путь `bsl-cli`: regex-объект уже
                        // принадлежит компоненту, даже если старый CLI ещё
                        // компилирует без `RuntimeRegistry`.
                        (Some(bsl_rt::BuiltinMethod::RegexGetGroups), _) => {
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
                let c = self.alloc_temp()?;
                self.compile_expr(cond, c)?;
                let to_else = self.emit(Instr::JumpIfFalse { cond: c, target: 0 });
                self.free_temp(1);
                self.compile_expr(then_expr, dst)?;
                let to_end = self.emit(Instr::Jump { target: 0 });
                let else_at = self.here();
                self.compile_expr(else_expr, dst)?;
                let end = self.here();
                self.patch_jump(to_else, else_at);
                self.patch_jump(to_end, end);
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
            RExpr::NewUuid { arg } => {
                let arg_reg = self.alloc_temp()?;
                self.compile_expr(arg, arg_reg)?;
                self.emit(Instr::NewUuid { dst, arg: arg_reg });
                self.free_temp(1);
            }
            RExpr::NewBinaryData { path } => {
                let path_reg = self.alloc_temp()?;
                self.compile_expr(path, path_reg)?;
                self.emit(Instr::NewBinaryData {
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
    fn compile_call(&mut self, func: u32, args: &[RExpr], dst: u8) -> Result<(), CompileError> {
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
            let by_val = *by_val.get(i).unwrap_or(&true);
            if !by_val && let RExpr::Local(slot) = arg {
                self.alloc_temp()?; // держим диапазон [base,base+argc) непрерывным
                modes.push(ArgMode::ByRefLocal(*slot as u8));
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
            RStmt::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let mut end_patches = Vec::new();

                let r = self.alloc_temp()?;
                self.compile_expr(cond, r)?;
                self.free_temp(1);
                let mut jf = self.emit(Instr::JumpIfFalse { cond: r, target: 0 });
                self.compile_block(then_branch)?;
                end_patches.push(self.emit(Instr::Jump { target: 0 }));

                for (c, body) in elsif_branches {
                    self.patch_jump(jf, self.here());
                    let r = self.alloc_temp()?;
                    self.compile_expr(c, r)?;
                    self.free_temp(1);
                    jf = self.emit(Instr::JumpIfFalse { cond: r, target: 0 });
                    self.compile_block(body)?;
                    end_patches.push(self.emit(Instr::Jump { target: 0 }));
                }

                self.patch_jump(jf, self.here());
                if let Some(else_body) = else_branch {
                    self.compile_block(else_body)?;
                }

                let end = self.here();
                for p in end_patches {
                    self.patch_jump(p, end);
                }
            }
            RStmt::While { cond, body } => {
                let cond_pc = self.here();
                let r = self.alloc_temp()?;
                self.compile_expr(cond, r)?;
                self.free_temp(1);
                let jf = self.emit(Instr::JumpIfFalse { cond: r, target: 0 });

                self.loop_stack.push(LoopCtx {
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });
                self.compile_block(body)?;
                self.emit(Instr::Jump {
                    target: cond_pc as i16,
                });

                let end = self.here();
                self.patch_jump(jf, end);
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches {
                    self.patch_jump(p, end);
                }
                // `Продолжить` в `Пока` — это "перепроверить условие".
                for p in ctx.continue_patches {
                    self.patch_jump(p, cond_pc);
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
                        target: body_pc as i16,
                    });
                } else {
                    self.emit(Instr::NumericForNext {
                        counter: slot,
                        bound,
                        target: body_pc as i16,
                    });
                }

                let end = self.here();
                self.patch_jump(jf, end);
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches {
                    self.patch_jump(p, end);
                }
                for p in ctx.continue_patches {
                    self.patch_jump(p, incr_pc);
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
                    target: cond_pc as i16,
                });

                let end = self.here();
                self.patch_jump(jf, end);
                let ctx = self.loop_stack.pop().unwrap();
                for p in ctx.break_patches {
                    self.patch_jump(p, end);
                }
                for p in ctx.continue_patches {
                    self.patch_jump(p, incr_pc);
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
                self.patch_jump(skip_handler, after);
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
