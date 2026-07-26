use bsl_rt::BslValue;
use bsl_sema::{RExpr, RStmt, Resolved};
use bsl_syntax::{BinaryOp, UnaryOp};

use crate::chunk::Chunk;
use crate::instr::Instr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    TooManyLocals,
    TooManyRegisters,
    TooManyConstants,
    BreakOutsideLoop,
    ContinueOutsideLoop,
}

pub fn compile_script(resolved: &Resolved) -> Result<Chunk, CompileError> {
    let n_locals: u8 = resolved
        .locals
        .len()
        .try_into()
        .map_err(|_| CompileError::TooManyLocals)?;
    let mut c = Compiler {
        instrs: Vec::new(),
        consts: Vec::new(),
        next_reg: n_locals,
        max_reg: n_locals,
        loop_stack: Vec::new(),
    };
    c.compile_block(&resolved.body)?;
    Ok(Chunk {
        instrs: c.instrs,
        consts: c.consts,
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

struct Compiler {
    instrs: Vec<Instr>,
    consts: Vec<BslValue>,
    /// Вершина свободных регистров: локалы занимают `0..n_locals`, дальше —
    /// стек временных регистров, растущий/сжимающийся вокруг компиляции
    /// каждого подвыражения (тот же приём, что в компиляторе Lua).
    next_reg: u8,
    max_reg: u8,
    loop_stack: Vec<LoopCtx>,
}

impl Compiler {
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
        }
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
            RStmt::Assign { slot, value } => {
                self.compile_expr(value, *slot as u8)?;
            }
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
