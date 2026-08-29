//! Инварианты графа потока управления над разрешённым деревом
//! (`docs/ssa-hotspot-analysis.md`, шаг 5).
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
