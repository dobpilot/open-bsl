//! Стоимость КОМПИЛЯЦИИ, без печати листинга.
//!
//! Замер через `bsl-cli --emit-bytecode` меряет не то, что называется:
//! листинг форматируется целиком даже при выводе в `/dev/null`, и на
//! реальной фикстуре это дороже самой компиляции, так что доля свёртки
//! в нём занижена. Здесь исходник компилируется тем же путём, что и в
//! обычном запуске (фасад `Engine`), нужное число раз и без единой
//! напечатанной строки; фиксированная цена старта размывается числом
//! итераций.
//!
//! ```text
//! cargo run --release -p open-bsl --example compile-cost -- <файл.bsl> <итераций> [const-fold]
//! ```
//!
//! Проход принимается ровно один — `const-fold`, и это не упрощение на
//! потом. Единственный источник имён проходов — таблица `PASS_NAMES` в
//! `bsl-cli`; второй разбор списка здесь был бы её копией, которая на
//! первом же новом проходе разъедется с оригиналом. Понадобится мерить
//! другой проход — расширять придётся осознанно, а не обнаружить, что
//! пример молча принял имя, которого `bsl-cli` не знает.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = || -> ! {
        eprintln!("нужно: compile-cost <файл.bsl> <итераций> [const-fold]");
        std::process::exit(2);
    };

    // Разбор по срезу целиком, а не по индексам: `&args[..3]` паникует
    // раньше, чем успевает сработать ветка отказа, если аргументов меньше.
    let (path, iters, opts) = match args.as_slice() {
        [_, path, iters] => (path, iters, bsl_compiler::Optimizations::default()),
        [_, path, iters, pass] if pass == "const-fold" => (
            path,
            iters,
            bsl_compiler::Optimizations {
                const_fold: true,
                ..bsl_compiler::Optimizations::default()
            },
        ),
        _ => usage(),
    };
    let Ok(iters) = iters.parse::<usize>() else {
        usage()
    };

    let source = std::fs::read_to_string(path).expect("чтение исходника");
    let engine = open_bsl::Engine::builder()
        .optimizations(opts)
        .build()
        .expect("сборка движка");

    for _ in 0..iters {
        // `black_box`, чтобы компиляция не была признана мёртвой: её
        // результат здесь никому не нужен, а измеряется именно она.
        let module = engine.compile(&source).expect("компиляция");
        std::hint::black_box(&module);
    }
}
