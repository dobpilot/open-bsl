//! Замер ДО оптимизации — брифом явно требуется мерить, а не гадать:
//! "профилируй сначала арифметику, а не цикл диспетчеризации". Запуск:
//! `cargo run --release --example bench -p bsl-vm`.
//!
//! Три сценария, чтобы отделить стоимость диспетчеризации от стоимости
//! арифметики (нормализация мантиссы, промоушен в BigInt):
//! 1. `dispatch_only` — цикл, который почти не считает (инкремент целого
//!    счётчика), почти вся стоимость — переход между инструкциями.
//! 2. `exact_multiplication_growth` — умножение без границы масштаба (как
//!    в n-body: `x = x * x`), масштаб растёт на каждой итерации, мантисса
//!    рано или поздно перестаёт помещаться в `i128` и уходит в `BigInt`.
//! 3. `nbody` — сам бенчмарк из брифа, несколько шагов `Advance`.

use std::time::Instant;

use bsl_rt::BslValue;

fn run(src: &str) -> BslValue {
    let program = bsl_syntax::parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
    let resolved =
        bsl_sema::resolve_program(&program.items).unwrap_or_else(|e| panic!("sema error: {e:?}"));
    let compiled =
        bsl_bytecode::compile_program(&resolved).unwrap_or_else(|e| panic!("compile error: {e:?}"));
    bsl_vm::run_program(&compiled).unwrap_or_else(|e| panic!("runtime error: {e:?}"))
}

fn time_it(name: &str, src: &str) {
    let start = Instant::now();
    let result = run(src);
    let elapsed = start.elapsed();
    let digits = result.to_string().len();
    println!("{name:<32} {elapsed:>10.2?}   (итог: {digits} символов)");
}

fn main() {
    // 1. Почти чистая диспетчеризация: целочисленный счётчик, масштаб не растёт.
    time_it(
        "dispatch_only (2_000_000 итер.)",
        "x = 0;\nДля i = 1 По 2000000 Цикл\nx = x + 1;\nКонецЦикла\nВозврат x;",
    );

    // 2. Точное умножение без границы: масштаб растёт линейно (+10 знаков
    // за итерацию — умножение на константу, не возведение в квадрат: то
    // растёт экспоненциально и почти сразу упирается в MAX_SCALE) —
    // изолирует стоимость нормализации/промоушена в BigInt от диспетчеризации.
    time_it(
        "exact_multiplication_growth (5000 итер.)",
        "x = 1.0000000001;\nk = 1.0000000001;\nДля i = 1 По 5000 Цикл\nx = x * k;\nКонецЦикла\nВозврат x;",
    );

    // 3. Доступ к полям структуры — изолирует именно то, что чинит инлайн-
    // кэш (GetProp/SetProp), без затрат на рост масштаба: одна структура,
    // одна форма, много чтений/записей поля — лучший случай для кэша
    // (мономорфный сайт вызова).
    time_it(
        "structure_field_access (1_000_000 итер.)",
        "s = Новый Структура(\"x\", 0);\n\
         Для i = 1 По 1000000 Цикл\n\
         s.x = s.x + 1;\n\
         КонецЦикла\n\
         Возврат s.x;",
    );

    // Та же идея, но структура с 7 полями (как Planet в n-body:
    // x,y,z,vx,vy,vz,mass) и доступ к ПОСЛЕДНЕМУ — ближе к реальной форме
    // n-body, где выигрыш кэша от размера формы виднее.
    time_it(
        "structure_field_access_7fields (1_000_000 итер.)",
        "s = Новый Структура(\"x,y,z,vx,vy,vz,mass\", 0,0,0,0,0,0,0);\n\
         Для i = 1 По 1000000 Цикл\n\
         s.mass = s.mass + 1;\n\
         КонецЦикла\n\
         Возврат s.mass;",
    );

    // 4. n-body — реалистичная смесь (структуры, поля, sqrt, деление,
    // умножение) на нескольких шагах Advance.
    let nbody_src = include_str!("../tests/nbody_smoke.bsl");
    time_it("nbody (3 шага Advance)", nbody_src);

    // 5. n-body, 200 шагов Advance — see tests/conformance/fixtures/
    // n-body-perf.bsl для того, почему не брифовские 100000 (масштаб растёт
    // быстрее квадратично; 1000 шагов уже падает через ~98с на MAX_SCALE).
    // Только здесь, не в cargo test — растущий масштаб делает каждый шаг
    // медленнее предыдущего, так что абсолютное время мало что говорит без
    // сравнения с `nbody (3 шага Advance)` выше на той же машине.
    let nbody_perf_src =
        include_str!("../../../tests/conformance/fixtures/n-body-perf.bsl");
    time_it("nbody (200 шагов Advance)", nbody_perf_src);
}
