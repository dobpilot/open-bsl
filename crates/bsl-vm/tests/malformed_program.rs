//! `Program` — публичный тип с публичными полями, поэтому VM не вправе
//! считать её заведомо корректной.
//!
//! Проверка целей при разборе текстового байт-кода защищает только путь
//! `--run-bytecode`. Rust-клиент, собравший или подправивший `Program`
//! напрямую, до этой правки получал `Ok(Неопределено)` вместо ошибки: VM
//! переводила цель через `as usize`, а `pc` за концом чанка принимала за
//! нормальное завершение — и остаток программы молча пропадал.

use bsl_bytecode::{ArgMode, Instr, Program, parse_program, write_program};
use bsl_compiler::compile_program;
use bsl_rt::RtError;

fn compile(src: &str) -> Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved).expect("кодоген")
}
/// Все переходы программы: `(номер чанка, позиция инструкции)`.
fn jumps(program: &Program) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (c, chunk) in program.chunks.iter().enumerate() {
        for (pc, instr) in chunk.instrs.iter().enumerate() {
            if instr.jump_target().is_some() {
                out.push((c, pc));
            }
        }
    }
    out
}

/// Имя опкода — первое слово его отладочной записи.
fn opcode_name(instr: &Instr) -> String {
    let text = format!("{instr:?}");
    text.split([' ', '{', '(']).next().unwrap_or("").to_string()
}

/// Исходник, дающий ВСЕ шесть опкодов с целью: условие с коротким
/// замыканием (`Jump`, `JumpIfFalse`, `JumpIfTrue`), цикл с телом и
/// пустой цикл (`NumericForNext`, `NumericForNextI64`) и вызов с
/// пропущенным параметром по умолчанию (`JumpIfNotSkipped`).
const ALL_SIX: &str = concat!(
    "Функция Ф(а = 3, б = 7)\n",
    "Возврат а + б;\n",
    "КонецФункции\n",
    "Сумма = 0;\n",
    "Если Истина ИЛИ Ложь Тогда\n",
    "Для н = 1 По 5 Цикл\n",
    "Сумма = Сумма + н;\n",
    "КонецЦикла;\n",
    "Для п = 1 По 3 Цикл\n",
    "КонецЦикла;\n",
    "Сумма = Сумма + Ф(1,);\n",
    "Иначе\n",
    "Сообщить(1);\n",
    "КонецЕсли;\n",
);

#[test]
fn the_sample_really_contains_every_target_carrying_opcode() {
    // Иначе следующий тест мог бы «покрывать шесть опкодов», не встретив
    // половины из них: именно так и вышло в первой редакции.
    let program = compile(ALL_SIX);
    let mut kinds: Vec<String> = jumps(&program)
        .into_iter()
        .map(|(c, pc)| opcode_name(&program.chunks[c].instrs[pc]))
        .collect();
    kinds.sort();
    kinds.dedup();
    assert_eq!(
        kinds,
        vec![
            "Jump",
            "JumpIfFalse",
            "JumpIfNotSkipped",
            "JumpIfTrue",
            "NumericForNext",
            "NumericForNextI64",
        ],
        "образец обязан содержать все шесть опкодов с целью"
    );
}

#[test]
fn every_jump_of_every_opcode_is_checked_wherever_it_sits() {
    // Портится КАЖДЫЙ переход во ВСЕХ чанках: проверка, глядящая лишь на
    // первый или лишь на нулевой чанк, прошла бы мимо остальных.
    let sites = jumps(&compile(ALL_SIX));
    assert!(sites.len() >= 6, "переходов должно быть не меньше шести");
    for (chunk, pc) in sites {
        for target in [-1, 9999] {
            let mut program = compile(ALL_SIX);
            let name = opcode_name(&program.chunks[chunk].instrs[pc]);
            match &mut program.chunks[chunk].instrs[pc] {
                Instr::Jump { target: t }
                | Instr::JumpIfFalse { target: t, .. }
                | Instr::JumpIfTrue { target: t, .. }
                | Instr::JumpIfNotSkipped { target: t, .. }
                | Instr::NumericForNext { target: t, .. }
                | Instr::NumericForNextI64 { target: t, .. } => *t = target,
                other => panic!("не переход: {other:?}"),
            }
            assert!(
                matches!(
                    bsl_vm::run_program(&program),
                    Err(RtError::InvalidBytecode(_))
                ),
                "{name} в чанке {chunk} на {pc} с целью {target} обязан быть ошибкой"
            );
        }
    }
}

#[test]
fn a_handler_out_of_the_chunk_is_invalid_bytecode_too() {
    let mut program = compile(
        "Попытка\nВызватьИсключение \"бум\";\nИсключение\nСообщить(\"поймано\");\nКонецПопытки;\n",
    );
    program.chunks[0].exception_ranges[0].handler_pc = 9999;
    assert!(matches!(
        bsl_vm::run_program(&program),
        Err(RtError::InvalidBytecode(_))
    ));
}

#[test]
fn a_correct_program_still_runs() {
    let program = compile("Если Ложь Тогда\nСообщить(1);\nКонецЕсли;\n");
    assert!(bsl_vm::run_program(&program).is_ok());
}

#[test]
fn an_empty_handler_at_the_end_unwinds_into_ordinary_termination() {
    // `handler_pc == instrs.len()` — законная цель, и проверять её надо не
    // только разбором листинга, но и ИСПОЛНЕНИЕМ: возврат строгого `>=` в
    // VM тест на разбор не сломал бы. Исключение здесь настоящее, поэтому
    // разматывание действительно приходит на `pc == len`.
    let program = compile(concat!(
        "Попытка\n",
        "ВызватьИсключение \"бум\";\n",
        "Исключение\n",
        "КонецПопытки;\n",
    ));
    let handler = program.chunks[0].exception_ranges[0].handler_pc;
    assert_eq!(
        handler,
        program.chunks[0].instrs.len(),
        "обработчик обязан указывать ровно за конец чанка"
    );
    assert!(
        bsl_vm::run_program(&program).is_ok(),
        "пустой обработчик в конце обязан ловить исключение и завершать программу"
    );
}

/// Режим `ArgMode::Default` обязывает вызванную функцию вычислить в этот
/// слот значение по умолчанию. У функции без умолчаний прологa нет, и слот
/// остался бы с тем, что вызывающий успел положить в свой временный
/// регистр раньше, — то есть результат зависел бы от чужого выражения.
/// Поэтому VM обнуляет слот при построении кадра: испорченный извне режим
/// даёт `Неопределено`, одно и то же при любом соседнем коде.
#[test]
fn a_default_mode_without_a_default_prologue_yields_undefined_not_leftovers() {
    let src = "Функция Ф(а)\nВозврат а;\nКонецФункции\nВозврат Ф(22);\n";
    // Контроль: без правки вызов возвращает переданное значение — значит
    // временный регистр действительно занят числом 22, и «Неопределено»
    // ниже берётся из обнуления, а не из пустоты.
    assert_eq!(
        bsl_vm::run_program(&compile(src)).unwrap(),
        bsl_rt::BslValue::Number(bsl_number::BslNumber::from_i64(22))
    );

    let mut program = compile(src);
    let modes = program.chunks[0]
        .instrs
        .iter()
        .find_map(|i| match i {
            Instr::Call { arg_modes, .. } => Some(*arg_modes as usize),
            _ => None,
        })
        .expect("в чанке верхнего уровня обязан быть вызов");
    program.chunks[0].call_arg_modes[modes][0] = ArgMode::Default;

    assert_eq!(
        bsl_vm::run_program(&program).unwrap(),
        bsl_rt::BslValue::Undefined
    );
}

// --- Геометрия вызова в тексте --------------------------------------------
//
// Здесь листинг проходит ИМЕННО через текстовый разбор — тот путь, которым
// байт-код приходит от `--run-bytecode`. Разбор эти правки принимает: они
// синтаксически безупречны, и заводить в нём вторую копию проверки значило
// бы повторить ошибку, которую этот файл уже фиксирует выше. Отвергает их
// связывание, до первой инструкции, и потому одинаково для листинга и для
// `Program`, собранной Rust-клиентом.

/// Двухпараметровая функция с умолчанием, вызванная с пропуском позиции, и
/// переменная вызывающего рядом — та, которую испорченный вызов затирал.
const CALL_SAMPLE: &str = concat!(
    "Функция Ф(а, б = 100)\n",
    "Возврат б;\n",
    "КонецФункции\n",
    "х = 7;\n",
    "у = Ф(1, );\n",
    "Возврат Строка(х) + \"/\" + Строка(у);\n",
);

/// Переписывает в строке листинга операнд `ключ=значение`, не трогая
/// остального: номера регистров у соседей и хвостовой комментарий.
fn retoken(line: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}=");
    line.split(' ')
        .map(|token| {
            if token.starts_with(&prefix) {
                format!("{prefix}{value}")
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Печатает листинг, портит единственную строку с `anchor` и разбирает его
/// обратно. Разбор обязан пройти: проверка живёт не в нём.
fn tampered(anchor: &str, edit: impl Fn(&str) -> String) -> Program {
    let listing = write_program(&compile(CALL_SAMPLE), None).expect("печать листинга");
    let hits: Vec<&str> = listing.lines().filter(|l| l.contains(anchor)).collect();
    assert_eq!(
        hits.len(),
        1,
        "«{anchor}» обязан встречаться в листинге ровно раз:\n{listing}"
    );
    let broken = listing.replace(hits[0], &edit(hits[0]));
    parse_program(&broken).unwrap_or_else(|e| panic!("разбор обязан принять правку: {e}\n{broken}"))
}

fn invalid(program: &Program) -> &'static str {
    match bsl_vm::run_program(program) {
        Err(RtError::InvalidBytecode(text)) => text,
        other => panic!("ожидалась ошибка некорректного байт-кода, получено {other:?}"),
    }
}

/// Контроль: неиспорченный листинг проходит тот же путь и даёт исходные
/// значения. Без него три теста ниже прошли бы и на программе, которая
/// вообще не собирается.
#[test]
fn the_intact_listing_still_round_trips_and_runs() {
    let listing = write_program(&compile(CALL_SAMPLE), None).expect("печать листинга");
    let program = parse_program(&listing).expect("разбор");
    // «7/100» — то, что обязана дать программа: переменная вызывающего
    // цела, а пропущенный аргумент взял объявленное умолчание. Каждая из
    // трёх правок ниже ломает ровно одно из двух, и до проверок ломала
    // молча (измерено снятием проверок: «100/100», «100/100» и «7/»).
    assert_eq!(bsl_vm::run_program(&program).unwrap().to_string(), "7/100");
}

/// Номера регистров — `u8`. `base + i` заворачивалось: в отладочной сборке
/// это была паника «attempt to add with overflow», а в релизной — молчаливая
/// подмена. Пропущенный параметр становился алиасом регистра 0 вызывающего,
/// и пролог умолчаний записывал туда 100 поверх `х = 7`: «100/100».
#[test]
fn a_call_whose_argument_registers_leave_the_frame_is_invalid_bytecode() {
    let program = tampered(" Call func=", |line| retoken(line, "base", "255"));
    assert_eq!(
        invalid(&program),
        "регистры аргументов вызова выходят за кадр"
    );
}

/// Длина набора режимов — это и есть арность вызова, а вся остальная
/// геометрия кадра считается по `n_params` вызываемой функции. Лишний режим
/// превращал собственный регистр вызванной функции в алиас слота
/// вызывающего, и она затирала `х`: «100/100» с кодом успеха.
#[test]
fn argument_modes_that_outnumber_the_parameters_are_invalid_bytecode() {
    let program = tampered("[value default]", |line| {
        line.replace("[value default]", "[value default byref:0]")
    });
    assert_eq!(
        invalid(&program),
        "режимов аргументов не столько, сколько параметров у вызываемой функции"
    );
}

/// `src` пролога умолчаний — номер параметра. Слот с номером за их числом
/// VM не находила и считала аргумент переданным: умолчание не вычислялось, и
/// функция возвращала `Неопределено` вместо 100 — «7/».
#[test]
fn a_default_prologue_pointing_past_the_parameters_is_invalid_bytecode() {
    let program = tampered("JumpIfNotSkipped", |line| retoken(line, "src", "2"));
    assert_eq!(
        invalid(&program),
        "пролог умолчаний ссылается на несуществующий параметр"
    );
}

// --- Дыры периметра образа, достроенные этапом 4 --------------------------

/// Как `tampered`, но для произвольного исходника: форма NewStructure и
/// вызов по ссылке в CALL_SAMPLE не встречаются.
fn tampered_in(src: &str, anchor: &str, edit: impl Fn(&str) -> String) -> Program {
    let listing = write_program(&compile(src), None).expect("печать листинга");
    let hits: Vec<&str> = listing.lines().filter(|l| l.contains(anchor)).collect();
    assert_eq!(
        hits.len(),
        1,
        "«{anchor}» обязан встречаться в листинге ровно раз:\n{listing}"
    );
    let broken = listing.replace(hits[0], &edit(hits[0]));
    parse_program(&broken).unwrap_or_else(|e| panic!("разбор обязан принять правку: {e}\n{broken}"))
}

/// Параметр по ссылке — алиас ЛОКАЛИ вызывающего. `byref:slot` за числом
/// локалей кадра делал параметр алиасом чужой ячейки, и вызванная функция
/// писала мимо: код возврата 0. Граница — `n_locals`, не `n_regs`.
#[test]
fn a_byref_local_past_the_frame_locals_is_invalid_bytecode() {
    let src = "Функция Ф(а)\nа = 5;\nКонецФункции\nх = 1;\nФ(х);\nВозврат х;\n";
    // Контроль: `а` без `Знач` — ссылка, `а = 5` пишет в `х` вызывающего.
    assert_eq!(bsl_vm::run_program(&compile(src)).unwrap().to_string(), "5");

    let mut program = compile(src);
    let mut patched = false;
    for chunk in &mut program.chunks {
        let byref_sets: Vec<usize> = chunk
            .instrs
            .iter()
            .filter_map(|i| match i {
                Instr::Call { arg_modes, .. } => Some(*arg_modes as usize),
                _ => None,
            })
            .filter(|&idx| {
                matches!(
                    chunk.call_arg_modes[idx].first(),
                    Some(ArgMode::ByRefLocal(_))
                )
            })
            .collect();
        for idx in byref_sets {
            chunk.call_arg_modes[idx][0] = ArgMode::ByRefLocal(250);
            patched = true;
        }
    }
    assert!(
        patched,
        "в программе обязан быть вызов с параметром по ссылке"
    );
    assert_eq!(
        invalid(&program),
        "параметр по ссылке указывает за локали кадра"
    );
}

/// `Новый Структура("а, б", 1, 2)` — форма из двух полей, значения в
/// регистрах `base..base+count`. `count`, не равный длине формы, давал
/// верное первое поле и `index out of bounds` на втором; `base + count`
/// считается как `u8`, и заворот делал поле алиасом чужого регистра.
#[test]
fn a_new_structure_with_a_broken_geometry_is_invalid_bytecode() {
    const STRUCT_SAMPLE: &str = "х = Новый Структура(\"а, б\", 1, 2);\nВозврат х.а;\n";
    // Контроль: неиспорченный листинг проходит тот же путь и даёт 1.
    let listing = write_program(&compile(STRUCT_SAMPLE), None).expect("печать");
    assert_eq!(
        bsl_vm::run_program(&parse_program(&listing).expect("разбор"))
            .unwrap()
            .to_string(),
        "1"
    );

    // Число полей, не равное длине формы.
    let count_off = tampered_in(STRUCT_SAMPLE, "NewStructure", |line| {
        retoken(line, "count", "1")
    });
    assert_eq!(
        invalid(&count_off),
        "число полей структуры не равно длине её формы"
    );

    // Регистры полей за кадром: `base + count` заворачивается в `u8`.
    let base_off = tampered_in(STRUCT_SAMPLE, "NewStructure", |line| {
        retoken(line, "base", "255")
    });
    assert_eq!(
        invalid(&base_off),
        "регистры полей структуры выходят за кадр"
    );
}

/// Ноль — это `chunks[0]`, тело модуля, которого не вызывает никто. Без
/// отсева `chunks.get(0)` вернул бы `Some`, и вызов рекурсивно входил бы в
/// верхний уровень: ловимый `StackOverflow`, который окружающая `Попытка`
/// проглотила бы, обратив битый образ в неверный ответ с кодом успеха.
/// Текстовый формат отвергает такой `Call` при печати (`BadCallTarget`), но
/// разбор листинга его принимает.
#[test]
fn a_call_targeting_the_top_level_chunk_is_invalid_bytecode() {
    let program = tampered(" Call func=", |line| retoken(line, "func", "0"));
    assert_eq!(invalid(&program), "вызов ссылается на чанк верхнего уровня");
}

/// Режимы (`argmodes`) и умолчания (`defaults`) параметров — по одному на
/// параметр. Массив короче `n_params` фрагмент `Выполнить` резолвит по
/// неверному режиму: недостающий `param_by_val` кодоген считает `Знач`, и
/// переданный ПО ССЫЛКЕ параметр становится переданным ЗНАЧЕНИЕМ — образ
/// возвращает 1 вместо 42 с кодом успеха (проверено на `--run-bytecode`).
/// Проверка статическая, поэтому ловится связыванием и на этом статическом
/// листинге, где сам вызов исполняется по режимам ВЫЗЫВАЮЩЕГО и потому цел.
#[test]
fn param_mode_counts_that_disagree_with_the_arity_are_invalid_bytecode() {
    let src = "Процедура П(а)\nа = 5;\nКонецПроцедуры\nх = 1;\nП(х);\nВозврат х;\n";
    // Контроль: параметр по ссылке, `а = 5` пишет в `х` вызывающего.
    assert_eq!(bsl_vm::run_program(&compile(src)).unwrap().to_string(), "5");
    // Убираем режим единственного параметра: `param_by_val` короче `n_params`.
    let program = tampered_in(src, ".chunk 1 params", |line| {
        retoken(line, "argmodes", "[]")
    });
    assert_eq!(
        invalid(&program),
        "число режимов или умолчаний параметров не совпадает с числом параметров"
    );
}

/// `NumericForNextI64` на продолжении прыгает на `target`, а скрытое
/// состояние ищет по `state.pc == pc`. Цель в границах чанка, но не равная
/// собственной позиции, увела бы управление на чужую инструкцию — это
/// предусловие нигде не проверялось.
#[test]
fn a_numeric_for_target_that_is_not_its_own_pc_is_invalid_bytecode() {
    let mut program = compile(ALL_SIX);
    let mut patched = false;
    for chunk in &mut program.chunks {
        let limit = chunk.instrs.len();
        for (pc, instr) in chunk.instrs.iter_mut().enumerate() {
            if let Instr::NumericForNextI64 { target, .. } = instr {
                // Цель в границах `[0, limit]`, но не равная своей позиции.
                let wrong = if pc == 0 { 1 } else { 0 };
                debug_assert!(wrong <= limit);
                *target = wrong as i16;
                patched = true;
            }
        }
    }
    assert!(patched, "образец обязан содержать NumericForNextI64");
    assert_eq!(
        invalid(&program),
        "числовой цикл: цель не указывает на собственную инструкцию"
    );
}

/// Рантайм-`InvalidBytecode` (номер модульной переменной за таблицей)
/// внутри `Попытка` НЕ ловится: повреждённый образ обязан уйти наружу, а не
/// быть проглоченным обработчиком с признаком успеха.
#[test]
fn a_runtime_invalid_bytecode_is_not_caught_by_an_exception_handler() {
    // Модульную переменную читаем ИЗ ФУНКЦИИ: на верхнем уровне она —
    // локаль кадра, и `GetModuleVar` кодоген порождает только в теле
    // функции.
    let src = concat!(
        "Перем М;\n",
        "Функция Читать()\n",
        "Попытка\n",
        "Возврат М;\n",
        "Исключение\n",
        "Возврат 0;\n",
        "КонецПопытки;\n",
        "КонецФункции\n",
        "М = 5;\n",
        "Возврат Читать();\n",
    );
    // Контроль: годная программа возвращает 5 (обработчик не срабатывает).
    assert_eq!(bsl_vm::run_program(&compile(src)).unwrap().to_string(), "5");

    let mut program = compile(src);
    let mut patched = false;
    for chunk in &mut program.chunks {
        for instr in &mut chunk.instrs {
            if let Instr::GetModuleVar { slot, .. } = instr {
                *slot = 250;
                patched = true;
            }
        }
    }
    assert!(patched, "в программе обязан быть GetModuleVar");
    assert!(
        matches!(
            bsl_vm::run_program(&program),
            Err(RtError::InvalidBytecode(_))
        ),
        "повреждённый образ не должен ловиться Попыткой"
    );
}

/// Периметр образа проверяет арность встроенной функции и метода с
/// ФИКСИРОВАННОЙ арностью: `bsl-vm` не видит `bsl-sema`, а крафтнутый `count`
/// ронял бы `call_builtin_*` на `args[0]`. Арности берутся из `bsl-rt`.
#[test]
fn a_wrong_builtin_arg_count_is_invalid_bytecode() {
    // CallBuiltin: `Sqrt` берёт ровно один аргумент; `count = 0` — за арностью.
    let mut program = compile("x = Sqrt(4);\nВозврат x;\n");
    let mut patched = false;
    for chunk in &mut program.chunks {
        for instr in &mut chunk.instrs {
            if let Instr::CallBuiltin { count, .. } = instr {
                *count = 0;
                patched = true;
            }
        }
    }
    assert!(patched, "в программе обязан быть CallBuiltin");
    assert_eq!(
        invalid(&program),
        "число аргументов встроенной функции вне её арности"
    );
}

/// То же для метода с фиксированной арностью: `Количество()` берёт ноль
/// аргументов, а `count = 1` не совпадает с `static_arity`.
#[test]
fn a_wrong_fixed_arity_method_arg_count_is_invalid_bytecode() {
    let mut program = compile("С = Новый Соответствие;\nВозврат С.Количество();\n");
    let mut patched = false;
    for chunk in &mut program.chunks {
        for instr in &mut chunk.instrs {
            if let Instr::CallMethod { count, .. } = instr {
                *count = 1;
                patched = true;
            }
        }
    }
    assert!(patched, "в программе обязан быть CallMethod");
    assert_eq!(
        invalid(&program),
        "число аргументов метода не совпадает с его арностью"
    );
}
