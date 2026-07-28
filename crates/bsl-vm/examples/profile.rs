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
    print_top_stacks(&report);
}

/// Текстовая сводка поверх SVG: пять самых тяжёлых СТЕКОВ с листом и
/// ближайшими родителями.
///
/// Флеймграф отвечает на вопрос «сколько», но не на вопрос «под кем»:
/// одна и та же функция на картинке размазана по нескольким местам, и
/// понять, из какого пути она вызвана, можно только наведя мышь. Для
/// разбора «почему здесь вообще BigInt» этого мало, а сводка отвечает
/// сразу и попадает в лог прогона.
fn print_top_stacks(report: &pprof::Report) {
    let total: isize = report.data.values().sum();
    let mut stacks: Vec<(&pprof::Frames, isize)> =
        report.data.iter().map(|(f, n)| (f, *n)).collect();
    stacks.sort_by_key(|(_, n)| -n);

    for (frames, count) in stacks.iter().take(5) {
        let pct = 100.0 * *count as f64 / total.max(1) as f64;
        // Символы идут от листа к корню; берём лист и трёх родителей —
        // дальше начинается общий для всех хвост drive/step/main.
        let names: Vec<String> = frames
            .frames
            .iter()
            .filter_map(|level| level.first())
            .map(|sym| short_name(&sym.name()))
            // Пустое имя — заинлайненный кадр, у которого символа не
            // осталось. Показывать «  <-    <-  » незачем: пропускаем и
            // берём следующий настоящий.
            .filter(|name| !name.is_empty())
            .take(4)
            .collect();
        println!("    {pct:>5.1}%  {}", names.join("  <-  "));
    }
}

/// Обрезает generic-хвосты: `core::slice::sort::stable::sort::<usize,
/// <[usize]>::sort_by<...>>` в одну строку сводки не влезает и читать его
/// незачем — важно имя функции.
fn short_name(name: &str) -> String {
    let head = name.split_once("::<").map(|(h, _)| h).unwrap_or(name);
    let head = head.split_once('<').map(|(h, _)| h).unwrap_or(head);
    head.trim_end_matches("::").to_string()
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

    // 4. Строки: поиск подстроки. Иголка в конце, стог не меняется —
    //    профиль показывает ЧИСТЫЙ `BslString::find` без затрат на сборку.
    //    Внешний двойник: benchmarks/str_find.{bsl,lua}.
    profile(
        "str_find",
        "к = \"абвгдеёжзийклмнопрстуфхцчшщъыьэюя0123456789\";\n\
         стог = \"\";\n\
         Для i = 1 По 5000 Цикл\nстог = стог + к;\nКонецЦикла\n\
         стог = стог + \"ИГОЛКА\";\n\
         п = 0;\n\
         Для i = 1 По 300 Цикл\nп = СтрНайти(стог, \"ИГОЛКА\");\nКонецЦикла\n\
         Возврат п;",
    );

    // 5. Строки: конкатенация в цикле. Здесь наоборот — вся стоимость в
    //    копировании `Rc<[u16]>`, поиска нет вовсе.
    profile(
        "str_concat",
        "к = \"абвгдеёжзийклмнопрстуфхцчшщъыьэюя0123456789\";\n\
         т = \"\";\n\
         Для i = 1 По 3000 Цикл\nт = т + к;\nКонецЦикла\n\
         Возврат СтрДлина(т);",
    );

    // 6. Таблица: заполнение плюс агрегат по колонке. Профиль отвечает на
    //    вопрос, что дороже — SetProp по строке таблицы при заполнении или
    //    сам проход по колонке в `Итог`.
    profile(
        "table_total",
        "т = Новый ТаблицаЗначений;\n\
         т.Колонки.Добавить(\"ном\");\n\
         т.Колонки.Добавить(\"имя\");\n\
         Для i = 1 По 100000 Цикл\n\
         с = т.Добавить();\nс.ном = i;\nс.имя = \"строка\";\n\
         КонецЦикла\n\
         итог = 0;\n\
         Для i = 1 По 30 Цикл\nитог = т.Итог(\"ном\");\nКонецЦикла\n\
         Возврат итог;",
    );

    // 7. Таблица: сортировка. Здесь ожидается `collate` — приближение
    //    коллации приводит ОБЕ строки к верхнему регистру на КАЖДОМ
    //    сравнении, то есть две аллокации на сравнение.
    profile(
        "table_sort",
        "т = Новый ТаблицаЗначений;\n\
         т.Колонки.Добавить(\"ключ\");\n\
         т.Колонки.Добавить(\"имя\");\n\
         Для i = 1 По 50000 Цикл\n\
         с = т.Добавить();\nс.ключ = 50000 - i;\nс.имя = \"имя\" + Строка(i);\n\
         КонецЦикла\n\
         т.Сортировать(\"ключ\");\n\
         т.Сортировать(\"имя\");\n\
         Возврат т.Количество();",
    );
}
