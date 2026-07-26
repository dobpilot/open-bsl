use bsl_rt::{BslValue, NameInterner, ShapeTable};
use bsl_sema::{RExpr, RStmt, ResolvedFunction, ResolvedProgram};
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
    BreakOutsideLoop,
    ContinueOutsideLoop,
}

/// Компилирует весь модуль: чанк верхнего уровня плюс чанк на каждую
/// `Процедура`/`Функция`, в том порядке, в котором их видит `bsl-sema`
/// (`Call.func` в резолвнутом дереве — индекс в `resolved.functions`;
/// здесь он сдвигается на 1, потому что `chunks[0]` — верхний уровень).
///
/// Имена полей и формы структур интернируются ОДИН РАЗ на весь модуль
/// (`names`/`shapes` ниже) — а не по чанку — чтобы одинаковый список полей
/// в разных функциях модуля давал одну и ту же форму: это и есть смысл
/// "глобального" интернирования из брифа применительно к тому, что реально
/// компилируется за один проход.
pub fn compile_program(resolved: &ResolvedProgram) -> Result<Program, CompileError> {
    let mut names = NameInterner::new();
    let mut shapes = ShapeTable::new();
    let mut chunks = Vec::with_capacity(resolved.functions.len() + 1);
    chunks.push(compile_chunk(
        &resolved.top_level.locals,
        0,
        &resolved.top_level.body,
        &resolved.functions,
        &mut names,
        &mut shapes,
    )?);
    for f in &resolved.functions {
        chunks.push(compile_chunk(
            &f.locals,
            f.params.len(),
            &f.body,
            &resolved.functions,
            &mut names,
            &mut shapes,
        )?);
    }
    Ok(Program {
        chunks,
        names: names.into_names(),
        shapes: shapes.into_shapes(),
    })
}

fn compile_chunk(
    locals: &[String],
    n_params: usize,
    body: &[RStmt],
    functions: &[ResolvedFunction],
    names: &mut NameInterner,
    shapes: &mut ShapeTable,
) -> Result<Chunk, CompileError> {
    let n_locals: u8 = locals
        .len()
        .try_into()
        .map_err(|_| CompileError::TooManyLocals)?;
    let n_params: u8 = n_params
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
        names,
        shapes,
    };
    c.compile_block(body)?;
    Ok(Chunk {
        instrs: c.instrs,
        consts: c.consts,
        call_arg_modes: c.call_arg_modes,
        exception_ranges: c.exception_ranges,
        n_params,
        n_locals,
        n_regs: c.max_reg,
    })
}

/// Список прыжков `Прервать`/`Продолжить`, которые патчатся, когда становится
/// известен конец цикла (`Прервать`) или точка повтора (`Продолжить`) — для
/// `Для` это шаг инкремента, известный только после компиляции тела.
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
            other => unreachable!("patch_jump on non-jump instruction: {other:?}"),
        }
    }

    fn here(&self) -> usize {
        self.instrs.len()
    }

    // --- Выражения ------------------------------------------------------

    fn compile_expr(&mut self, e: &RExpr, dst: u8) -> Result<(), CompileError> {
        match e {
            RExpr::Number(n) => {
                let k = self.add_const(BslValue::Number(n.clone()))?;
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
            RExpr::Local(slot) => {
                let src = *slot as u8;
                if src != dst {
                    self.emit(Instr::Move { dst, src });
                }
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
            RExpr::Binary { op, lhs, rhs } => {
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
            RExpr::CallMethod { obj, method } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                self.emit(Instr::CallMethod {
                    dst,
                    obj: o,
                    method: *method,
                });
                self.free_temp(1);
            }
            RExpr::Str(s) => {
                let k = self.add_const(BslValue::Str(std::rc::Rc::from(s.as_str())))?;
                self.emit(Instr::LoadConst { dst, k });
            }
            RExpr::Index { obj, index } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let i = self.alloc_temp()?;
                self.compile_expr(index, i)?;
                self.emit(Instr::GetIndex { dst, obj: o, idx: i });
                self.free_temp(2);
            }
            RExpr::Field { obj, name } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let name_id = self.names.intern(name);
                self.emit(Instr::GetProp {
                    dst,
                    obj: o,
                    name: name_id,
                });
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
        }
        Ok(())
    }

    /// Аргументы без `Знач`, чья форма на месте вызова — голая локальная
    /// переменная, передаются по ссылке (алиас на слот вызывающего, без
    /// материализации значения); всё остальное — обычным значением в
    /// регистре `base + i`. Само решение "по ссылке или нет" статично и
    /// целиком принимается здесь, в компиляторе.
    fn compile_call(&mut self, func: u32, args: &[RExpr], dst: u8) -> Result<(), CompileError> {
        let params = &self.functions[func as usize].params;
        let base = self.next_reg;
        let mut modes = Vec::with_capacity(args.len());
        for (i, arg) in args.iter().enumerate() {
            let by_val = params[i].by_val;
            if !by_val {
                if let RExpr::Local(slot) = arg {
                    self.alloc_temp()?; // держим диапазон [base,base+argc) непрерывным
                    modes.push(ArgMode::ByRefLocal(*slot as u8));
                    continue;
                }
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
            RStmt::AssignIndex { obj, index, value } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let i = self.alloc_temp()?;
                self.compile_expr(index, i)?;
                let v = self.alloc_temp()?;
                self.compile_expr(value, v)?;
                self.emit(Instr::SetIndex { obj: o, idx: i, src: v });
                self.free_temp(3);
            }
            RStmt::AssignField { obj, name, value } => {
                let o = self.alloc_temp()?;
                self.compile_expr(obj, o)?;
                let v = self.alloc_temp()?;
                self.compile_expr(value, v)?;
                let name_id = self.names.intern(name);
                self.emit(Instr::SetProp {
                    obj: o,
                    name: name_id,
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

                let cond_pc = self.here();
                let cmp = self.alloc_temp()?;
                self.emit(Instr::Le {
                    dst: cmp,
                    a: slot,
                    b: bound,
                });
                self.free_temp(1);
                let jf = self.emit(Instr::JumpIfFalse { cond: cmp, target: 0 });

                self.loop_stack.push(LoopCtx {
                    break_patches: Vec::new(),
                    continue_patches: Vec::new(),
                });
                self.compile_block(body)?;

                let incr_pc = self.here();
                let one = self.alloc_temp()?;
                let k = self.add_const(BslValue::Number(bsl_number::BslNumber::from_i64(1)))?;
                self.emit(Instr::LoadConst { dst: one, k });
                self.emit(Instr::Add {
                    dst: slot,
                    a: slot,
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
                let zero_k = self.add_const(BslValue::Number(bsl_number::BslNumber::from_i64(0)))?;
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
                let jf = self.emit(Instr::JumpIfFalse { cond: cmp, target: 0 });

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
        BinaryOp::Eq => Instr::Eq { dst, a, b },
        BinaryOp::NotEq => Instr::NotEq { dst, a, b },
        BinaryOp::Lt => Instr::Lt { dst, a, b },
        BinaryOp::Gt => Instr::Gt { dst, a, b },
        BinaryOp::Le => Instr::Le { dst, a, b },
        BinaryOp::Ge => Instr::Ge { dst, a, b },
        BinaryOp::And => Instr::And { dst, a, b },
        BinaryOp::Or => Instr::Or { dst, a, b },
    }
}
