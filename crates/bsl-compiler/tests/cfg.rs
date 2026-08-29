//! Инварианты графа потока управления над разрешённым деревом
//! (`docs/research/performance/ssa-hotspot-analysis.md`, шаг 5).
//!
//! Проверка независима от построителя: она пересчитывает преемников из
//! терминаторов и требует, чтобы обратные ссылки, достижимость и
//! доминаторы сошлись. Построитель мог бы согласовать сам с собой любую
//! ошибку — потому проверка и написана отдельно.

use bsl_compiler::cfg;
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
