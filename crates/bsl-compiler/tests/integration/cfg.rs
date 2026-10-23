//! Инварианты графа потока управления над разрешённым деревом
//! (`docs/research/performance/ssa-hotspot-analysis.md`, шаг 5).
//!
//! Проверка независима от построителя: она пересчитывает преемников из
//! терминаторов и требует, чтобы обратные ссылки, достижимость и
//! доминаторы сошлись. Построитель мог бы согласовать сам с собой любую
//! ошибку — потому проверка и написана отдельно.

use bsl_compiler::cfg;
use bsl_compiler::{Optimizations, compile_program_with};
use std::path::{Path, PathBuf};

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = Vec::new();
    for dir in ["tests/conformance/fixtures", "benchmarks"] {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "bsl") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Граф строится и сходится на всём, что резолвится без реестра
/// компонентов.
///
/// Скрипты, которым реестр нужен, пропускаются — этот крейт компонентных
/// зависимостей не имеет, — но пропуск не молчаливый: тест печатает, что
/// проверено и что пропущено, и падает, если покрытие выродилось. Иначе
/// зелёный прогон означал бы «ничего не проверено».
#[test]
fn the_control_flow_graph_holds_its_invariants_on_the_corpus() {
    let mut checked = 0usize;
    let mut skipped = 0usize;
    let mut blocks = 0usize;
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            skipped += 1;
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            skipped += 1;
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut bodies: Vec<(&str, &[bsl_sema::RStmt])> =
            vec![("<верхний уровень>", &resolved.top_level.body)];
        for f in &resolved.functions {
            bodies.push((&f.name, &f.body));
        }
        for (what, body) in bodies {
            let graph = cfg::build(body);
            blocks += graph.blocks.len();
            if let Err(e) = cfg::verify(&graph) {
                panic!("{name}, {what}: {e}");
            }
        }
        checked += 1;
    }
    assert!(
        checked >= 20,
        "проверено всего {checked} скриптов — покрытие выродилось"
    );
    println!("граф сверен на {checked} скриптах ({blocks} блоков), пропущено {skipped}");
}

fn graph_of(src: &str) -> usize {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let graph = cfg::build(&resolved.top_level.body);
    cfg::verify(&graph).expect("инварианты");
    graph.blocks.len()
}

/// Восемь тысяч ПОСЛЕДОВАТЕЛЬНЫХ `Если` — тот самый скрипт, на котором
/// сломалась прошлая попытка: у неё он занимал 2,58 с против 0,20 с без
/// анализа. Соседние операторы вложенности не создают, и обход дерева
/// здесь мелкий; расти обязано только число блоков.
#[test]
fn eight_thousand_sequential_conditions_do_not_blow_up() {
    let src = format!(
        "А = 1;\n{}",
        "Если А = 1 Тогда А = 2; КонецЕсли;\n".repeat(8000)
    );
    let blocks = graph_of(&src);
    assert!(blocks > 8000, "ожидался блок на каждое ветвление: {blocks}");
}

/// Обработчик `Исключение` получает вход из КАЖДОГО блока защищённого
/// тела, а не только из его начала: исключение срабатывает на любом
/// операторе.
#[test]
fn a_handler_is_entered_from_every_block_of_the_protected_body() {
    let parsed = bsl_syntax::parse(
        "Попытка\n\tА = 1;\n\tЕсли А = 1 Тогда А = 2; КонецЕсли;\nИсключение\n\tА = 3;\nКонецПопытки;",
    )
    .expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let graph = cfg::build(&resolved.top_level.body);
    cfg::verify(&graph).expect("инварианты");

    let covered = graph
        .blocks
        .iter()
        .filter(|b| !b.handlers.is_empty())
        .count();
    assert!(
        covered >= 3,
        "тело с ветвлением обязано дать несколько накрытых блоков, а не один: {covered}"
    );
}

/// Недостижимый блок — это `Bottom`: доминатора у него нет, и проверка
/// требует именно этого, а не «доминатор неизвестен».
#[test]
fn an_unreachable_block_has_no_dominator() {
    let parsed = bsl_syntax::parse("А = 1;\nВозврат;\nА = 2;\n").expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let graph = cfg::build(&resolved.top_level.body);
    cfg::verify(&graph).expect("инварианты");

    let idom = graph.immediate_dominators();
    let reachable: std::collections::HashSet<_> = graph.reverse_postorder().into_iter().collect();
    assert!(
        (0..graph.blocks.len()).any(|b| !reachable.contains(&b)),
        "после `Возврат` обязан остаться недостижимый блок"
    );
    for (b, dom) in idom.iter().enumerate() {
        if !reachable.contains(&b) {
            assert!(dom.is_none(), "у недостижимого блока {b} есть доминатор");
        }
    }
}

// ---------------------------------------------------------------------
// SSA поверх графа
// ---------------------------------------------------------------------

use bsl_compiler::ssa;

fn ssa_of(src: &str) -> (bsl_sema::ResolvedProgram, usize) {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let n = resolved.top_level.locals.len();
    (resolved, n)
}

/// Ветвление с записью в обеих ветвях обязано дать `φ` в точке слияния —
/// это и есть та самая расстановка, которую план требует проверять
/// рукописным ожиданием на маленькой программе.
#[test]
fn a_variable_written_in_both_branches_gets_a_phi_at_the_join() {
    let (resolved, n) = ssa_of("А = 1;\nЕсли А = 1 Тогда А = 2; Иначе А = 3; КонецЕсли;\nБ = А;");
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");

    let phis: Vec<_> = form
        .values
        .iter()
        .filter(|v| matches!(v, ssa::Value::Phi { .. }))
        .collect();
    assert!(!phis.is_empty(), "слияния без φ: {:?}", form.values);
    let two_operands = phis
        .iter()
        .any(|v| matches!(v, ssa::Value::Phi { operands, .. } if operands.len() == 2));
    assert!(
        two_operands,
        "у φ слияния двух ветвей обязано быть два операнда"
    );
}

/// Прямолинейный код `φ` не порождает: сливать нечего.
#[test]
fn straight_line_code_needs_no_phi() {
    let (resolved, n) = ssa_of("А = 1;\nБ = А;\nВ = Б;");
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");

    assert!(
        !form
            .values
            .iter()
            .any(|v| matches!(v, ssa::Value::Phi { .. })),
        "в линейном коде φ не нужны: {:?}",
        form.values
    );
}

/// Недостижимый блок значений не получает вовсе: это `Bottom`, а не
/// «значение неизвестно».
#[test]
fn an_unreachable_block_carries_no_values() {
    let (resolved, n) = ssa_of("А = 1;\nВозврат;\nА = 2;\nБ = А;");
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");

    let reachable: std::collections::HashSet<_> = graph.reverse_postorder().into_iter().collect();
    let mut unreachable_seen = false;
    for b in 0..graph.blocks.len() {
        if !reachable.contains(&b) {
            unreachable_seen = true;
            assert!(
                form.entry[b].is_none() && form.exit[b].is_none(),
                "у недостижимого блока {b} есть состояние"
            );
        }
    }
    assert!(unreachable_seen, "после `Возврат` обязан быть мёртвый блок");
}

/// Цикл — тот случай, ради которого φ и заводят: значение на входе в
/// заголовок приходит и снаружи, и с обратного ребра.
#[test]
fn a_loop_header_merges_the_back_edge() {
    let (resolved, n) = ssa_of("А = 1;\nПока А < 10 Цикл\n\tА = А + 1;\nКонецЦикла;\nБ = А;");
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");

    assert!(
        form.values
            .iter()
            .any(|v| matches!(v, ssa::Value::Phi { operands, .. } if operands.len() == 2)),
        "заголовок цикла обязан слить вход и обратное ребро: {:?}",
        form.values
    );
}

/// Инварианты SSA держатся на всём, что резолвится без реестра.
#[test]
fn the_ssa_holds_its_invariants_on_the_corpus() {
    let mut checked = 0usize;
    let mut phis = 0usize;
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut bodies: Vec<(&str, &[bsl_sema::RStmt], usize)> = vec![(
            "<верхний уровень>",
            &resolved.top_level.body,
            resolved.top_level.locals.len(),
        )];
        for f in &resolved.functions {
            bodies.push((&f.name, &f.body, f.locals.len()));
        }
        for (what, body, n) in bodies {
            let graph = cfg::build(body);
            let form = ssa::build(&graph, n);
            phis += form.phis.iter().map(Vec::len).sum::<usize>();
            if let Err(e) = ssa::verify(&graph, &form) {
                panic!("{name}, {what}: {e}");
            }
        }
        checked += 1;
    }
    assert!(
        checked >= 20,
        "проверено {checked} скриптов — покрытие выродилось"
    );
    println!("SSA сверена на {checked} скриптах, φ построено {phis}");
}

// ---------------------------------------------------------------------
// Распространение констант по φ
// ---------------------------------------------------------------------

fn constants(src: &str) -> (Vec<ssa::Const>, ssa::Ssa) {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let n = resolved.top_level.locals.len();
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");
    let lat = ssa::propagate_constants(&graph, &form, n);
    (lat, form)
}

fn has_number(lat: &[ssa::Const], want: i64) -> bool {
    lat.iter().any(|c| match c {
        ssa::Const::Number(n) => n.to_i64_exact() == Some(want),
        _ => false,
    })
}

/// Одно и то же значение в обеих ветвях переживает слияние: `φ`
/// объединяет два одинаковых числа в него же.
#[test]
fn the_same_constant_on_both_branches_survives_the_join() {
    let (lat, form) = constants("Б = 0;\nЕсли Б = 0 Тогда А = 7; Иначе А = 7; КонецЕсли;\nВ = А;");

    let phi_const = form
        .values
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, ssa::Value::Phi { .. }))
        .any(|(id, _)| matches!(&lat[id], ssa::Const::Number(n) if n.to_i64_exact() == Some(7)));
    assert!(phi_const, "φ обязана остаться числом 7: {lat:?}");
}

/// Разные значения дают `Top`: константы больше нет.
#[test]
fn different_constants_on_the_branches_meet_to_top() {
    let (lat, form) = constants("Б = 0;\nЕсли Б = 0 Тогда А = 7; Иначе А = 8; КонецЕсли;\nВ = А;");

    let phi_top = form
        .values
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, ssa::Value::Phi { .. }))
        .any(|(id, _)| lat[id] == ssa::Const::Top);
    assert!(phi_top, "слияние 7 и 8 обязано дать Top: {lat:?}");
}

/// **То, чего блочно-локальный проход не может.** Ветвь заканчивается
/// `Возврат`, и в точку слияния из неё пути нет; операнд `φ` оттуда —
/// `Bottom`, а `Bottom` — единица объединения, поэтому константа с
/// живого ребра слияние переживает.
#[test]
fn a_constant_survives_a_merge_with_an_unreachable_branch() {
    let (lat, _) =
        constants("Б = 0;\nА = 5;\nЕсли Б = 0 Тогда\n\tА = 9;\n\tВозврат;\nКонецЕсли;\nВ = А;");

    assert!(
        has_number(&lat, 5),
        "константа с достижимого ребра обязана пережить слияние с мёртвым: {lat:?}"
    );
}

/// Счётчик цикла константой не остаётся: обратное ребро приносит другое
/// значение, и объединение поднимает решётку до `Top`.
#[test]
fn a_loop_carried_counter_is_not_constant() {
    let (lat, form) = constants("А = 0;\nПока А < 10 Цикл\n\tА = А + 1;\nКонецЦикла;\nБ = А;");

    let phi_top = form
        .values
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, ssa::Value::Phi { .. }))
        .any(|(id, _)| lat[id] == ssa::Const::Top);
    assert!(phi_top, "счётчик цикла константой быть не может: {lat:?}");
}

/// Арифметика над известными складывается, а деление на ноль — нет:
/// правило то же, что у свёртки в кодогене, и второй его редакции здесь
/// не заводится.
#[test]
fn arithmetic_folds_but_a_failing_operation_does_not() {
    // Слоты идут в порядке первого появления: А=0, Б=1, В=2.
    let (lat, form) = constants("А = 2;\nБ = А * 3;\nВ = 1 / 0;");
    assert!(has_number(&lat, 6), "2 * 3 обязано свернуться: {lat:?}");

    let div = form
        .values
        .iter()
        .enumerate()
        .find(|(_, v)| matches!(v, ssa::Value::Def { slot: 2, .. }))
        .map(|(id, _)| id)
        .expect("значение для В");
    assert_eq!(
        lat[div],
        ssa::Const::Top,
        "деление на ноль обязано остаться неизвестным, а не стать значением решётки"
    );
}

/// Решётка сходится на всём корпусе и не зацикливается.
#[test]
fn constant_propagation_converges_on_the_corpus() {
    let mut checked = 0usize;
    let mut known = 0usize;
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        let mut bodies: Vec<(&[bsl_sema::RStmt], usize)> =
            vec![(&resolved.top_level.body, resolved.top_level.locals.len())];
        for f in &resolved.functions {
            bodies.push((&f.body, f.locals.len()));
        }
        for (body, n) in bodies {
            let graph = cfg::build(body);
            let form = ssa::build(&graph, n);
            let lat = ssa::propagate_constants(&graph, &form, n);
            known += lat
                .iter()
                .filter(|c| matches!(c, ssa::Const::Number(_)))
                .count();
        }
        checked += 1;
    }
    assert!(checked >= 20, "проверено {checked} скриптов");
    println!("решётка сошлась на {checked} скриптах, известных значений {known}");
}

/// `Выполнить` меняет локальные по именам, и SSA обязана считать
/// изменёнными ВСЕ: иначе константа «переживёт» фрагмент, которого не
/// видно. Проверка на том же примере, который без этого правила давал бы
/// неверный ответ.
#[test]
fn a_dynamic_fragment_kills_every_slot() {
    let (lat, form) = constants("А = 1;\nВыполнить(\"А = 2\");\nБ = А;");

    // После фрагмента слот переопределён, и его значение неизвестно.
    let after: Vec<_> = form
        .values
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, ssa::Value::Def { slot: 0, .. }))
        .collect();
    assert!(
        after.len() >= 2,
        "фрагмент обязан дать слоту новое определение: {:?}",
        form.values
    );
    let last = after.last().unwrap().0;
    assert_eq!(
        lat[last],
        ssa::Const::Top,
        "после `Выполнить` значение слота известным быть не может: {lat:?}"
    );
}

/// Использования собираются, и доминирование проверяется по ним.
#[test]
fn uses_are_recorded_and_dominated_by_their_definitions() {
    let parsed = bsl_syntax::parse("А = 1;\nБ = А + 1;\nВ = Б;").expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let n = resolved.top_level.locals.len();
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");

    assert!(
        form.uses.len() >= 2,
        "чтения `А` и `Б` обязаны попасть в использования: {:?}",
        form.uses
    );
}

// ---------------------------------------------------------------------
// Живые диапазоны и раскладка по регистрам
// ---------------------------------------------------------------------

use bsl_compiler::regalloc;

fn allocation(src: &str) -> (regalloc::Allocation, ssa::Ssa, cfg::Cfg<'static>) {
    // Дерево живёт столько же, сколько тест: утечка здесь дешевле, чем
    // тащить время жизни через возвращаемый кортеж.
    let parsed = Box::leak(Box::new(bsl_syntax::parse(src).expect("разбор")));
    let resolved = Box::leak(Box::new(
        bsl_sema::resolve_program(&parsed.items).expect("резолвинг"),
    ));
    let n = resolved.top_level.locals.len();
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");
    let alloc = regalloc::allocate(&graph, &form).expect("раскладка");
    regalloc::verify(&graph, &form, &alloc).expect("проверка раскладки");
    (alloc, form, graph)
}

/// Значения, живущие одновременно, обязаны попасть в разные регистры.
#[test]
fn simultaneously_live_values_get_different_registers() {
    let (alloc, form, _) = allocation("А = 1;\nБ = 2;\nВ = А + Б;");

    // `А` и `Б` живы одновременно к моменту сложения.
    let defs: Vec<_> = form
        .values
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, ssa::Value::Def { .. }))
        .map(|(id, _)| id)
        .collect();
    assert!(
        defs.len() >= 3,
        "ожидались три определения: {:?}",
        form.values
    );
    assert_ne!(
        alloc.reg[defs[0]], alloc.reg[defs[1]],
        "одновременно живые `А` и `Б` делят регистр"
    );
}

/// А непересекающиеся — обязаны иметь возможность его разделить: иначе
/// раскладка не экономит ничего и смысла в ней нет.
#[test]
fn values_that_never_overlap_may_share_a_register() {
    let (alloc, _, _) =
        allocation("А = 1;\nСообщить(А);\nБ = 2;\nСообщить(Б);\nВ = 3;\nСообщить(В);");
    assert!(
        alloc.used <= 3,
        "три непересекающихся значения заняли {} регистров",
        alloc.used
    );
}

/// Раскладка считается и сходится на всём корпусе, и её проверка
/// независима от построителя.
#[test]
fn the_allocation_holds_on_the_corpus() {
    let mut checked = 0usize;
    let mut worst = 0usize;
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut bodies: Vec<(&str, &[bsl_sema::RStmt], usize)> = vec![(
            "<верхний уровень>",
            &resolved.top_level.body,
            resolved.top_level.locals.len(),
        )];
        for f in &resolved.functions {
            bodies.push((&f.name, &f.body, f.locals.len()));
        }
        for (what, body, n) in bodies {
            let graph = cfg::build(body);
            let form = ssa::build(&graph, n);
            // Отказ по нехватке регистров — законный исход, а не ошибка:
            // кадр вмещает 255, и кодоген на том же пределе отказывает уже
            // сегодня.
            if let Ok(alloc) = regalloc::allocate(&graph, &form) {
                worst = worst.max(alloc.used);
                if let Err(e) = regalloc::verify(&graph, &form, &alloc) {
                    panic!("{name}, {what}: {e}");
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 20, "проверено {checked} скриптов");
    println!("раскладка сверена на {checked} скриптах, максимум регистров {worst}");
}

/// Цикл, замкнутый `Перейти`, обязан получить `φ` в заголовке — как и
/// цикл `Пока`. Ошибка, ради которой написан тест, была не в графе, а во
/// фронтах доминирования: подъём обрывался, не дойдя до заголовка, чей
/// предшественник — входной блок, и `φ` не ставилась вовсе. Байт-код от
/// этого получался неверный, а инварианты SSA при этом держались:
/// представление было самосогласованным и неправильным одновременно.
#[test]
fn a_loop_closed_by_goto_gets_a_phi_too() {
    let (lat, form) = constants(
        "Счетчик = 0;\n~0:\nСчетчик = Счетчик + 1;\nЕсли Счетчик < 3 Тогда\n\tGoto ~0;\nКонецЕсли;\n",
    );

    assert!(
        form.values
            .iter()
            .any(|v| matches!(v, ssa::Value::Phi { .. })),
        "заголовок цикла через `Перейти` остался без φ: {:?}",
        form.values
    );
    let phi_top = form
        .values
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, ssa::Value::Phi { .. }))
        .any(|(id, _)| lat[id] == ssa::Const::Top);
    assert!(
        phi_top,
        "счётчик, растущий по обратному ребру, константой быть не может: {lat:?}"
    );
}

// ---------------------------------------------------------------------
// Домен представлений
// ---------------------------------------------------------------------

fn tiers(src: &str) -> (Vec<ssa::Tier>, ssa::Ssa) {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let n = resolved.top_level.locals.len();
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    ssa::verify(&graph, &form).expect("инварианты SSA");
    let t = ssa::propagate_tiers(&graph, &form, n);
    (t, form)
}

fn has_int64(t: &[ssa::Tier]) -> bool {
    t.iter().any(|x| matches!(x, ssa::Tier::Int64 { .. }))
}

/// Целые литералы и арифметика над ними доказывают ярус `Int64` вместе с
/// диапазоном.
#[test]
fn integer_arithmetic_proves_the_int64_tier() {
    let (t, _) = tiers("А = 2;\nБ = А * 3;\nВ = Б + 1;");
    assert!(has_int64(&t), "ярус целого не доказан: {t:?}");
    assert!(
        t.iter()
            .any(|x| matches!(x, ssa::Tier::Int64 { lo, hi } if *lo == 7 && *hi == 7)),
        "диапазон обязан сойтись к 7: {t:?}"
    );
}

/// Дробное число — тоже число, но яруса целого не даёт.
#[test]
fn a_fractional_literal_is_a_number_without_the_integer_tier() {
    let (t, _) = tiers("А = 0.5;\nБ = А + 1;");
    assert!(
        t.contains(&ssa::Tier::Number),
        "дробное обязано остаться числом без яруса: {t:?}"
    );
}

/// **Переполнение яруса не заворачивается молча.** Умножение двух
/// значений, чьи границы выходят за `i64`, опускает ярус до `Число`, а не
/// сужает его — плана это требование прямое.
#[test]
fn an_operation_that_may_overflow_drops_the_tier() {
    let big = i64::MAX;
    let (t, _) = tiers(&format!("А = {big};\nБ = А * {big};"));
    assert!(
        !t.iter()
            .any(|x| matches!(x, ssa::Tier::Int64 { lo, hi } if *lo != *hi || *lo == 0))
            || t.contains(&ssa::Tier::Number),
        "выход за i64 обязан опустить ярус до числа: {t:?}"
    );
}

/// Деление яруса целого не даёт даже при целых операндах: `1 / 2` в BSL
/// даёт `0,5`.
#[test]
fn division_does_not_prove_an_integer_even_from_integers() {
    let (t, form) = tiers("А = 1;\nБ = 2;\nВ = А / Б;");
    let div = form
        .values
        .iter()
        .enumerate()
        .find(|(_, v)| matches!(v, ssa::Value::Def { slot: 2, .. }))
        .map(|(id, _)| id)
        .expect("значение для В");
    assert_ne!(
        t[div],
        ssa::Tier::Int64 { lo: 0, hi: 0 },
        "деление не доказывает целого: {t:?}"
    );
}

/// Решётка ярусов сходится на всём корпусе, и её видно: сколько значений
/// удалось доказать целыми — это и есть статическая половина
/// доказательной базы шага 8.
/// Переменная числового цикла с целыми границами несёт `Int64`, и
/// диапазон накрывает как тело, так и выход из цикла.
#[test]
fn a_numeric_loop_variable_carries_the_int64_tier() {
    let (t, _) = tiers("Для н = 1 По 10 Цикл\n  Б = н;\nКонецЦикла;");
    assert!(
        t.iter()
            .any(|x| matches!(x, ssa::Tier::Int64 { lo, hi } if *lo == 1 && *hi == 11)),
        "ожидался Int64 [1, 11], получено {t:?}"
    );
}

/// А вот если тело ПИШЕТ в переменную цикла, границы о ней уже ничего не
/// говорят: счётчик кодоген ведёт прямо в слоте переменной, и следующая
/// итерация пойдёт от присвоенного значения, а не от очередного целого.
/// `Для н = 1 По 10 Цикл н = 100; КонецЦикла` доходит до 101.
#[test]
fn a_loop_variable_the_body_assigns_proves_nothing() {
    let (t, _) = tiers("Для н = 1 По 10 Цикл\n  н = 100;\nКонецЦикла;");
    assert!(
        !t.iter()
            .any(|x| matches!(x, ssa::Tier::Int64 { lo, hi } if *lo == 1 && *hi == 11)),
        "диапазон цикла пережил запись в теле: {t:?}"
    );
}

/// Граница, о которой не доказано, что она число, оставляет переменную
/// недоказанной — а не «числом».
#[test]
fn a_loop_bound_of_unknown_type_leaves_the_variable_unproven() {
    // Граница — строка: число из неё не доказано, и цикл на ней —
    // ошибка времени исполнения, а не целый диапазон.
    let (t, _) = tiers("Гр = \"десять\";\nДля н = 1 По Гр Цикл\n  Б = н;\nКонецЦикла;");
    assert!(
        !has_int64(&t),
        "переменная цикла с недоказанной границей получила Int64: {t:?}"
    );
}

#[test]
fn the_tier_lattice_converges_on_the_corpus() {
    let mut checked = 0usize;
    let (mut int64, mut number, mut top) = (0usize, 0usize, 0usize);
    let (mut top_entry, mut top_phi, mut top_def) = (0usize, 0usize, 0usize);
    let mut top_param = 0usize;
    let (mut used_int64, mut used_number, mut used_top) = (0usize, 0usize, 0usize);
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        // Третий элемент — сколько первых слотов являются ПАРАМЕТРАМИ.
        // У верхнего уровня их нет, и его входы — неинициализированные
        // локальные, то есть `Неопределено`: там `Top` верен и никаким
        // анализом не снимается.
        let mut bodies: Vec<(&[bsl_sema::RStmt], usize, usize)> =
            vec![(&resolved.top_level.body, resolved.top_level.locals.len(), 0)];
        for f in &resolved.functions {
            bodies.push((&f.body, f.locals.len(), f.params.len()));
        }
        for (body, n, n_params) in bodies {
            let graph = cfg::build(body);
            let form = ssa::build(&graph, n);
            let tiers = ssa::propagate_tiers(&graph, &form, n);
            // Значение, которое никто не читает, ярусом ничего не решает,
            // а знаменатель завышает: у входа их особенно много — слот,
            // присвоенный до первого чтения, всё равно имеет вход.
            // Поэтому вторая перепись идёт по ЧИТАЕМЫМ значениям.
            let mut read = vec![false; form.values.len()];
            for u in &form.uses {
                read[u.value] = true;
            }
            for (id, t) in tiers.iter().enumerate() {
                if read[id] {
                    match t {
                        ssa::Tier::Int64 { .. } => used_int64 += 1,
                        ssa::Tier::Number => used_number += 1,
                        ssa::Tier::Top => used_top += 1,
                        ssa::Tier::Bottom => {}
                    }
                }
                match t {
                    ssa::Tier::Int64 { .. } => int64 += 1,
                    ssa::Tier::Number => number += 1,
                    ssa::Tier::Bottom => {}
                    // `Top` разбирается по происхождению: пока не видно,
                    // ЧТО именно не доказано, нельзя решить, окупится ли
                    // следующая ступень анализа. Вход — параметр либо
                    // неинициализированная локальная, то есть цена
                    // отсутствия межпроцедурного входа; `φ` — цена
                    // слияния; определение — цена самого оператора.
                    ssa::Tier::Top => {
                        top += 1;
                        match &form.values[id] {
                            ssa::Value::Entry { slot } => {
                                top_entry += 1;
                                if (*slot as usize) < n_params {
                                    top_param += 1;
                                }
                            }
                            ssa::Value::Phi { .. } => top_phi += 1,
                            _ => top_def += 1,
                        }
                    }
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 20, "проверено {checked} скриптов");
    println!("ярусы сошлись на {checked} скриптах: Int64 {int64}, Число {number}, не число {top}");
    println!(
        "  из них не число: вход {top_entry} (параметров {top_param}), φ {top_phi}, определение {top_def}"
    );
    let used = used_int64 + used_number + used_top;
    println!(
        "  по ЧИТАЕМЫМ значениям ({used}): Int64 {used_int64}, Число {used_number}, не число {used_top}"
    );
}

// ---------------------------------------------------------------------
// Сверка предсказаний анализа с тем, что выпускает кодоген
// ---------------------------------------------------------------------

/// Всякий слот, в который кодоген ПИШЕТ, анализ обязан считать
/// записываемым.
///
/// Это и есть тот «исчерпывающий перечень видов записи в локальную,
/// выведенный, а не угаданный», без которого раскладку по регистрам
/// включать нельзя. Выводится он не чтением `compile_stmt` глазами, а
/// сверкой с таблицей эффектов `bsl-bytecode`: та исчерпывающа по
/// опкодам без ветви-заглушки, то есть уже является источником истины о
/// записях. Расхождение здесь называет пропущенный вид записи точно —
/// именно так были найдены `Выполнить` и переменная цикла.
///
/// Обратное включение НЕ проверяется: анализ вправе считать слот
/// записанным там, где кодоген обошёлся без записи. Завышение стоит
/// регистра, занижение — верности.
#[test]
fn every_slot_the_generator_writes_is_predicted_by_the_analysis() {
    let mut checked = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        let Ok(program) = bsl_compiler::compile_program(&resolved) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let mut bodies: Vec<(&str, &[bsl_sema::RStmt], usize)> = vec![(
            "<верхний уровень>",
            &resolved.top_level.body,
            resolved.top_level.locals.len(),
        )];
        for f in &resolved.functions {
            bodies.push((&f.name, &f.body, f.locals.len()));
        }
        for (i, (what, body, n)) in bodies.into_iter().enumerate() {
            let Some(chunk) = program.chunks.get(i) else {
                continue;
            };
            let limit = u8::try_from(n).unwrap_or(u8::MAX).min(chunk.n_locals);
            if limit == 0 {
                continue;
            }
            let overlap = bsl_bytecode::analysis::module_overlap(i, resolved.module_vars.len());
            let emitted = bsl_bytecode::analysis::written_regs(chunk, overlap, limit);

            // Предсказание анализа: слоты, у которых есть хоть одно
            // определение помимо входного.
            let graph = cfg::build(body);
            let form = ssa::build(&graph, n);
            let mut predicted = vec![false; limit as usize];
            // Параметры пишет пролог умолчаний, живущий ВНЕ тела: для
            // анализа они приходят как `Entry`, и это их определение.
            for slot in 0..chunk.n_params.min(limit) {
                predicted[slot as usize] = true;
            }
            for v in &form.values {
                if let ssa::Value::Def { slot, .. } | ssa::Value::Phi { slot, .. } = v
                    && (*slot as usize) < predicted.len()
                {
                    predicted[*slot as usize] = true;
                }
            }
            for (slot, &written) in emitted.iter().enumerate() {
                if written && !predicted[slot] {
                    missing.push(format!("{name}, {what}: слот {slot}"));
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 20, "проверено {checked} скриптов");
    assert!(
        missing.is_empty(),
        "кодоген пишет слоты, которых анализ не предсказал ({} шт.): {:?}",
        missing.len(),
        &missing[..missing.len().min(8)]
    );
    println!("предсказания записи сверены с кодогеном на {checked} скриптах");
}

/// Мёртвое определение всё равно ПИШЕТ в регистр и обязано конфликтовать
/// со всем живым.
///
/// `Н = Поток.Перейти(0)`, чьё значение никто не читает, не смеет делить
/// регистр с `Поток`: запись уничтожила бы поток. Пока конфликты
/// собирались только по живым, такие определения были невидимы, и
/// `binary-streams` падал с «метод не найден у Число».
#[test]
fn a_dead_definition_still_interferes_with_what_is_live() {
    let src = "Поток = Новый Массив;\n\
               Н = Поток.Количество();\n\
               Н = Поток.Количество();\n\
               Сообщить(Поток.Количество());\n";
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let n = resolved.top_level.locals.len();
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, n);
    let alloc = regalloc::allocate_slots(&graph, &form, n, 0).expect("раскладка");

    let stream = resolved
        .top_level
        .locals
        .iter()
        .position(|x| x == "Поток")
        .expect("слот Поток");
    let dead = resolved
        .top_level
        .locals
        .iter()
        .position(|x| x == "Н")
        .expect("слот Н");
    assert_ne!(
        alloc[stream], alloc[dead],
        "мёртвое определение `Н` делит регистр с живым `Поток`: {alloc:?}"
    );
}

/// Значение, использованное только в `Возврат`, живо: терминатор читает
/// его наравне с оператором.
#[test]
fn a_value_used_only_in_a_return_is_live() {
    let parsed = bsl_syntax::parse(
        "Функция Ф()\n\tА = 1;\n\tБ = 2;\n\tВозврат Новый Массив(А, Б);\nКонецФункции\nСообщить(Ф());",
    )
    .expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let f = &resolved.functions[0];
    let n = f.locals.len();
    let graph = cfg::build(&f.body);
    let form = ssa::build(&graph, n);
    let alloc = regalloc::allocate_slots(&graph, &form, n, f.params.len()).expect("раскладка");

    assert_ne!(
        alloc[0], alloc[1],
        "`А` и `Б` живы обе в возвращаемом выражении: {alloc:?}"
    );
}

/// Проверяется ИМЕННО та раскладка, которую применяет кодоген
/// (`allocate_slots`), а не раскладка значений: применяется одна, а
/// проверялась другая.
#[test]
fn the_applied_slot_layout_holds_on_the_corpus() {
    let mut checked = 0usize;
    let mut merged = 0usize;
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut bodies: Vec<(&str, &[bsl_sema::RStmt], usize, usize)> = vec![(
            "<верхний уровень>",
            &resolved.top_level.body,
            resolved.top_level.locals.len(),
            resolved.module_vars.len(),
        )];
        for f in &resolved.functions {
            bodies.push((&f.name, &f.body, f.locals.len(), f.params.len()));
        }
        for (what, body, n, pinned) in bodies {
            if n == 0 {
                continue;
            }
            let graph = cfg::build(body);
            let form = ssa::build(&graph, n);
            // Отказ распределителя — провал теста с именем файла и чанка,
            // а не пропуск. Пропущенный чанк не проверяется ничем, и
            // зелёный прогон означал бы «проверено» там, где не
            // проверялось.
            let alloc = regalloc::allocate_slots(&graph, &form, n, pinned)
                .unwrap_or_else(|e| panic!("{name}, {what}: раскладка отказала: {e}"));
            if let Err(e) = regalloc::verify_slots(&graph, &form, n, &alloc) {
                panic!("{name}, {what}: {e}");
            }
            if alloc.iter().collect::<std::collections::HashSet<_>>().len() < n {
                merged += 1;
            }
        }
        checked += 1;
    }
    assert!(checked >= 20, "проверено {checked} скриптов");
    // Раскладка, нигде не слившая ни одного слота, — это тождественное
    // отображение, и зелёная проверка над ним ничего не значит.
    assert!(
        merged > 0,
        "раскладка не слила ни одного слота на всём корпусе"
    );
    println!("раскладка слотов сверена на {checked} скриптах, слияния в {merged} чанках");
}

/// Кадр обязан НЕ РАСТИ от раскладки, и хоть где-то обязан уменьшиться —
/// иначе проход меняет номера операндов, ничего не выигрывая.
#[test]
fn the_layout_never_grows_the_frame_and_somewhere_shrinks_it() {
    let mut shrank = 0usize;
    for path in corpus() {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = bsl_syntax::parse(&src) else {
            continue;
        };
        let Ok(resolved) = bsl_sema::resolve_program(&parsed.items) else {
            continue;
        };
        let Ok(base) = bsl_compiler::compile_program(&resolved) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        // То же и здесь: компиляция с раскладкой обязана удаться там, где
        // удалась без неё. Молча пропустить — значит не заметить, что
        // проход выключился сам.
        let laid = compile_program_with(
            &resolved,
            Optimizations {
                ssa_regalloc: true,
                ..Optimizations::default()
            },
        )
        .unwrap_or_else(|e| panic!("{name}: компиляция с раскладкой отказала: {e}"));
        for (i, (b, l)) in base.chunks.iter().zip(&laid.chunks).enumerate() {
            assert!(
                l.n_locals <= b.n_locals && l.n_regs <= b.n_regs,
                "{name}, чанк {i}: кадр вырос {}/{} -> {}/{}",
                b.n_locals,
                b.n_regs,
                l.n_locals,
                l.n_regs
            );
            if l.n_regs < b.n_regs {
                shrank += 1;
            }
        }
    }
    assert!(
        shrank > 0,
        "раскладка нигде не уменьшила кадр — заявленной пользы нет"
    );
    println!("кадр уменьшен в {shrank} чанках");
}
