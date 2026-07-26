//! Цикл диспетчеризации VM: `match` в `loop`, без computed goto (в Rust его
//! нет и явные хвостовые вызовы не стабилизированы — не воюем с этим здесь,
//! оптимизация диспетчеризации приходит после профилирования, не раньше).

use bsl_bytecode::{Chunk, Instr};
use bsl_rt::{BslValue, RtError};

/// Выполняет чанк верхнего уровня (без вызовов — кадры появятся в M4) и
/// возвращает финальное состояние регистров, чтобы можно было прочитать
/// значения локальных переменных после выполнения.
pub fn run(chunk: &Chunk) -> Result<Vec<BslValue>, RtError> {
    let mut regs = vec![BslValue::Undefined; chunk.n_regs as usize];
    let mut pc: usize = 0;

    while pc < chunk.instrs.len() {
        match &chunk.instrs[pc] {
            Instr::Move { dst, src } => {
                regs[*dst as usize] = regs[*src as usize].clone();
                pc += 1;
            }
            Instr::LoadConst { dst, k } => {
                regs[*dst as usize] = chunk.consts[*k as usize].clone();
                pc += 1;
            }
            Instr::LoadBool { dst, val } => {
                regs[*dst as usize] = BslValue::Boolean(*val);
                pc += 1;
            }
            Instr::LoadUndefined { dst } => {
                regs[*dst as usize] = BslValue::Undefined;
                pc += 1;
            }
            Instr::LoadNull { dst } => {
                regs[*dst as usize] = BslValue::Null;
                pc += 1;
            }
            Instr::Add { dst, a, b } => {
                regs[*dst as usize] = regs[*a as usize].add(&regs[*b as usize])?;
                pc += 1;
            }
            Instr::Sub { dst, a, b } => {
                regs[*dst as usize] = regs[*a as usize].sub(&regs[*b as usize])?;
                pc += 1;
            }
            Instr::Mul { dst, a, b } => {
                regs[*dst as usize] = regs[*a as usize].mul(&regs[*b as usize])?;
                pc += 1;
            }
            Instr::Div { dst, a, b } => {
                regs[*dst as usize] = regs[*a as usize].div(&regs[*b as usize])?;
                pc += 1;
            }
            Instr::Neg { dst, src } => {
                regs[*dst as usize] = regs[*src as usize].neg()?;
                pc += 1;
            }
            Instr::Not { dst, src } => {
                regs[*dst as usize] = regs[*src as usize].not()?;
                pc += 1;
            }
            Instr::And { dst, a, b } => {
                regs[*dst as usize] = regs[*a as usize].and(&regs[*b as usize])?;
                pc += 1;
            }
            Instr::Or { dst, a, b } => {
                regs[*dst as usize] = regs[*a as usize].or(&regs[*b as usize])?;
                pc += 1;
            }
            Instr::Eq { dst, a, b } => {
                regs[*dst as usize] = BslValue::Boolean(regs[*a as usize].eq_value(&regs[*b as usize]));
                pc += 1;
            }
            Instr::NotEq { dst, a, b } => {
                regs[*dst as usize] =
                    BslValue::Boolean(!regs[*a as usize].eq_value(&regs[*b as usize]));
                pc += 1;
            }
            Instr::Lt { dst, a, b } => {
                regs[*dst as usize] =
                    BslValue::Boolean(regs[*a as usize].compare(&regs[*b as usize], "<")?.is_lt());
                pc += 1;
            }
            Instr::Gt { dst, a, b } => {
                regs[*dst as usize] =
                    BslValue::Boolean(regs[*a as usize].compare(&regs[*b as usize], ">")?.is_gt());
                pc += 1;
            }
            Instr::Le { dst, a, b } => {
                regs[*dst as usize] =
                    BslValue::Boolean(regs[*a as usize].compare(&regs[*b as usize], "<=")?.is_le());
                pc += 1;
            }
            Instr::Ge { dst, a, b } => {
                regs[*dst as usize] =
                    BslValue::Boolean(regs[*a as usize].compare(&regs[*b as usize], ">=")?.is_ge());
                pc += 1;
            }
            Instr::Jump { target } => {
                pc = *target as usize;
            }
            Instr::JumpIfFalse { cond, target } => {
                // Строгая булевость: не-`Булево` в условии — ошибка типа,
                // а не приведение к истинности.
                if regs[*cond as usize].as_condition()? {
                    pc += 1;
                } else {
                    pc = *target as usize;
                }
            }
        }
    }

    Ok(regs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_bytecode::compile_script;
    use bsl_number::BslNumber;
    use bsl_sema::resolve_script;
    use bsl_syntax::{parse, Item, Stmt};

    fn run_script(src: &str) -> Vec<BslValue> {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        let stmts: Vec<Stmt> = prog
            .items
            .into_iter()
            .map(|item| match item {
                Item::Stmt(s) => s,
                Item::VarDecl(vd) => Stmt::VarDecl(vd),
                other => panic!("unexpected item in VM test script: {other:?}"),
            })
            .collect();
        let resolved = resolve_script(&stmts).unwrap_or_else(|e| panic!("sema error: {e:?}"));
        let chunk = compile_script(&resolved).unwrap_or_else(|e| panic!("compile error: {e:?}"));
        run(&chunk).unwrap_or_else(|e| panic!("runtime error: {e:?}"))
    }

    fn run_script_err(src: &str) -> RtError {
        let prog = parse(src).unwrap();
        let stmts: Vec<Stmt> = prog
            .items
            .into_iter()
            .map(|item| match item {
                Item::Stmt(s) => s,
                Item::VarDecl(vd) => Stmt::VarDecl(vd),
                other => panic!("unexpected item in VM test script: {other:?}"),
            })
            .collect();
        let resolved = resolve_script(&stmts).unwrap();
        let chunk = compile_script(&resolved).unwrap();
        run(&chunk).unwrap_err()
    }

    fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    #[test]
    fn division_matches_oracle_27_digits() {
        // Тот же замер, что в оракуле bsl-number: 1/3 -> 27 троек.
        let regs = run_script("x = 1 / 3;");
        assert_eq!(regs[0], num("0.333333333333333333333333333"));
    }

    #[test]
    fn multiplication_is_exact_through_the_whole_pipeline() {
        let regs = run_script("x = 1.10 * 1.00;");
        assert_eq!(regs[0], num("1.1"));
    }

    #[test]
    fn if_else_branching() {
        let regs = run_script("Если Ложь Тогда\nx = 1;\nИначе\nx = 2;\nКонецЕсли");
        assert_eq!(regs[0], num("2"));
    }

    #[test]
    fn elsif_chain_picks_the_right_branch() {
        let regs = run_script(
            "y = 2;\nЕсли y = 1 Тогда\nx = 10;\nИначеЕсли y = 2 Тогда\nx = 20;\nИначе\nx = 30;\nКонецЕсли",
        );
        assert_eq!(regs[1], num("20")); // x объявлен вторым -> слот 1
    }

    #[test]
    fn while_loop_with_break() {
        let regs = run_script(
            "i = 0;\nПока Истина Цикл\nЕсли i = 5 Тогда\nПрервать;\nКонецЕсли;\ni = i + 1;\nКонецЦикла",
        );
        assert_eq!(regs[0], num("5"));
    }

    #[test]
    fn for_loop_sums_and_variable_survives_after_end() {
        // Сумма 0..10 включительно = 55; переменная цикла `i` жива после
        // КонецЦикла и равна 11 (границы вычисляются один раз).
        let regs = run_script(
            "sum = 0;\nДля i = 0 По 10 Цикл\nsum = sum + i;\nКонецЦикла",
        );
        assert_eq!(regs[0], num("55"));
        assert_eq!(regs[1], num("11"));
    }

    #[test]
    fn for_loop_bounds_evaluated_once() {
        // `bound` меняется внутри тела — Для не должен пересчитывать предел
        // на каждой итерации, иначе цикл никогда не кончится (bound растёт
        // быстрее i).
        let regs = run_script(
            "bound = 3;\ncount = 0;\nДля i = 0 По bound Цикл\ncount = count + 1;\nbound = bound + 100;\nКонецЦикла",
        );
        assert_eq!(regs[1], num("4")); // count: границы 0..3 включительно -> 4 итерации
    }

    #[test]
    fn continue_skips_rest_of_for_body() {
        let regs = run_script(
            "sum = 0;\nДля i = 0 По 4 Цикл\nЕсли i = 2 Тогда\nПродолжить;\nКонецЕсли;\nsum = sum + i;\nКонецЦикла",
        );
        // 0+1+3+4 = 8 (2 пропущен через Продолжить); sum объявлен первым -> слот 0.
        assert_eq!(regs[0], num("8"));
    }

    #[test]
    fn strict_boolean_condition_is_a_runtime_error() {
        // Если 1 Тогда — ошибка, не приведение к истинности.
        let err = run_script_err("Если 1 Тогда\nx = 1;\nКонецЕсли");
        assert!(matches!(
            err,
            RtError::TypeError {
                expected: "Булево",
                ..
            }
        ));
    }

    #[test]
    fn division_by_zero_is_a_runtime_error() {
        let err = run_script_err("x = 1 / 0;");
        assert!(matches!(err, RtError::Num(bsl_number::NumError::DivideByZero)));
    }

    #[test]
    fn logical_and_or_do_not_short_circuit() {
        // Оба операнда обязаны быть Булево — если бы `И` был короткозамкнутым
        // и второй операнд не вычислялся при Ложь слева, эта ошибка типа не
        // возникла бы. Она возникает, значит короткого замыкания нет.
        let err = run_script_err("x = Ложь И 1;");
        assert!(matches!(
            err,
            RtError::TypeError {
                expected: "Булево",
                ..
            }
        ));
    }

    #[test]
    fn unary_minus_and_not() {
        let regs = run_script("x = -5;\ny = Не Истина;");
        assert_eq!(regs[0], num("-5"));
        assert_eq!(regs[1], BslValue::Boolean(false));
    }
}
