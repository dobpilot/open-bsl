//! Флейм граф без perf: pprof-rs сэмплирует внутри процесса через SIGPROF.
//! Запуск: cargo run --profile profiling --example profile -p bsl-vm
//!
//! КАЖДЫЙ сценарий профилируется ОТДЕЛЬНО и пишет свой SVG. Слить их в один
//! график нельзя: у dispatch_only и у роста масштаба принципиально разные
//! профили, и в общем графике они замаскируют друг друга.

use bsl_rt::BslValue;

fn run(src: &str) -> BslValue {
    let program = bsl_syntax::parse(src).expect("parse");
    let resolved = bsl_sema::resolve_program(&program.items).expect("sema");
    let compiled = bsl_bytecode::compile_program(&resolved).expect("compile");
    bsl_vm::run_program(&compiled).expect("runtime")
}

fn profile(name: &str, src: &str) {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(997) // простое число: не попадает в резонанс с циклами
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("profiler");

    let t = std::time::Instant::now();
    let result = run(src);
    let elapsed = t.elapsed();

    let report = guard.report().build().expect("report");
    let path = format!("flamegraph-{name}.svg");
    let file = std::fs::File::create(&path).expect("create svg");
    report.flamegraph(file).expect("flamegraph");

    println!(
        "{name:<28} {elapsed:>9.2?}  итог {:>6} симв.  -> {path}",
        result.to_string().len()
    );
}

fn main() {
    // 1. Почти чистая диспетчеризация: масштаб не растёт, арифметика в i64.
    profile(
        "dispatch",
        "x = 0;\nДля i = 1 По 3000000 Цикл\nx = x + 1;\nКонецЦикла\nВозврат x;",
    );

    // 2. Точное умножение: масштаб растёт линейно, мантисса уходит в BigInt.
    //    Здесь должна доминировать арифметика, а не VM.
    profile(
        "scale_growth",
        "x = 1.0000000001;\nДля i = 1 По 2000 Цикл\nx = x * 1.0000000001;\nКонецЦикла\nВозврат x;",
    );

    // 3. Доступ к полям структуры: проверка, что инлайн-кэш работает и
    //    GetProp не доминирует.
    profile(
        "struct_fields",
        "s = Новый Структура(\"a,b,c\", 1, 2, 3);\nt = 0;\nДля i = 1 По 1000000 Цикл\nt = t + s.a + s.b + s.c;\nКонецЦикла\nВозврат t;",
    );
}
