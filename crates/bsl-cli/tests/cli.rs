//! Проверки самого интерфейса командной строки: подсказка, неизвестная
//! команда, нехватка аргумента, обычный запуск скрипта.
//!
//! Подпроцессом, а не вызовом функций: разбор аргументов и коды возврата
//! — ровно то, что видит человек в терминале, и проверять их изнутри
//! процесса бессмысленно (`std::process::exit` не вернётся).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../../tests/support/http_system.rs"]
mod http_system_support;

fn bsl_cli() -> &'static str {
    env!("CARGO_BIN_EXE_bsl-cli")
}

fn run(args: &[&str]) -> Output {
    Command::new(bsl_cli())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("не удалось запустить bsl-cli {args:?}: {e}"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/fixtures")
        .join(name)
}

fn measure(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/measure")
        .join(name)
}

fn stdout_of(out: &Output) -> String {
    assert!(
        out.status.success(),
        "прогон завершился ошибкой: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn help_is_printed_to_stdout_with_a_zero_exit_code() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert!(out.status.success(), "{flag}: код возврата {}", out.status);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("ИСПОЛЬЗОВАНИЕ"), "{flag}: {text}");
        // Каждая команда названа — список собирается из той же таблицы,
        // по которой идёт разбор (см. `COMMANDS` в main.rs).
        for expected in [
            "--emit-bytecode",
            "--run-bytecode",
            "--ingest-measurements",
            "--help",
        ] {
            assert!(
                text.contains(expected),
                "{flag}: в подсказке нет {expected}"
            );
        }
        // Подсказка — это не ошибка: stderr пуст.
        assert!(out.stderr.is_empty(), "{flag}: {:?}", out.stderr);
    }
}

#[test]
fn an_unknown_command_is_an_error_pointing_at_help() {
    let out = run(&["--такой-команды-нет"]);
    assert!(!out.status.success(), "неизвестная команда прошла молча");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--такой-команды-нет"), "{err}");
    assert!(err.contains("--help"), "{err}");
    // Ошибка идёт в stderr, чтобы `bsl-cli ... | что-то` не получил её на вход.
    assert!(out.stdout.is_empty());
}

#[test]
fn a_command_without_its_argument_shows_that_commands_usage() {
    for (flag, expected) in [
        ("--emit-bytecode", "<файл.bsl>"),
        ("--run-bytecode", "<файл.bslc>"),
        ("--ingest-measurements", "<вывод-платформы>"),
    ] {
        let out = run(&[flag]);
        assert!(!out.status.success(), "{flag}: прошло без аргумента");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(flag), "{flag}: {err}");
        assert!(err.contains(expected), "{flag}: нет формы вызова: {err}");
    }
}

/// Перестановка базы вызова на источник копии не имеет права сделать
/// параметр `Знач` общим с переменной вызывающего. Форма байт-кода это
/// проверяет в `bsl-compiler/tests/copy_elim.rs`, но проверка формы не
/// заменяет прогона: здесь исполняется присваивание внутри функции, и
/// снаружи оно обязано быть невидимым.
#[test]
fn a_znach_parameter_stays_private_under_copy_elimination() {
    let path = std::env::temp_dir().join(format!("bsl-cli-test-znach-{}.bsl", std::process::id()));
    std::fs::write(
        &path,
        "Функция Испортить(Знач Х)\n\
         \tХ = 99;\n\
         \tВозврат Х;\n\
         КонецФункции\n\
         А = 1;\n\
         Сообщить(Испортить(А));\n\
         Сообщить(А);\n",
    )
    .unwrap();
    let script = path.to_str().unwrap();

    let plain = stdout_of(&run(&[script]));
    assert_eq!(plain, "99\n1\n", "без оптимизаций `Знач` уже сломан");
    assert_eq!(
        stdout_of(&run(&["--optimize=copy-elim", script])),
        plain,
        "устранение копий сделало параметр `Знач` общим с переменной вызывающего"
    );

    let _ = std::fs::remove_file(&path);
}

/// Программа, собранная из модулей через `//@используй`, компилируется
/// другим путём — через рецепт каталога, — и проходы туда доходить
/// обязаны. Молчаливая потеря ключа тут хуже отказа: замер на таком
/// скрипте показывал бы «копий не снято», а на деле проход не запускался.
#[test]
fn the_optimize_modifier_reaches_modules_linked_by_a_directive() {
    let dir = std::env::temp_dir().join(format!("bsl-cli-test-linked-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let module = dir.join("mod.bsl");
    let main = dir.join("main.bsl");
    std::fs::write(&module, "Процедура Пусто() Экспорт\nКонецПроцедуры\n").unwrap();
    std::fs::write(
        &main,
        "//@используй(mod.bsl как М)\n\
         Т = Новый Массив;\n\
         Х = 5;\n\
         Т.Добавить(Х);\n\
         Сообщить(Т.Количество());\n",
    )
    .unwrap();
    let script = main.to_str().unwrap();

    let plain = stdout_of(&run(&["--emit-bytecode", script]));
    let optimized = stdout_of(&run(&["--optimize=copy-elim", "--emit-bytecode", script]));
    assert!(
        plain.contains("Move"),
        "исходный байт-код обязан содержать копии: {plain}"
    );
    assert_ne!(
        plain, optimized,
        "--optimize не дошёл до программы, собранной через //@используй"
    );
    // И поведение при этом прежнее.
    assert_eq!(
        stdout_of(&run(&["--optimize=copy-elim", script])),
        stdout_of(&run(&[script])),
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `--optimize=список` — интерфейс замера, а не удобство: без него число
/// ворот принадлежит комбинации проходов. Поэтому его контракт закреплён
/// тестом целиком — приём списка, отказ на опечатке и то, что имена
/// проходов видны в `--help`.
#[test]
fn the_optimize_modifier_takes_a_list_of_passes() {
    let path = std::env::temp_dir().join(format!(
        "bsl-cli-test-optimize-list-{}.bsl",
        std::process::id()
    ));
    // `Б + 3` сворачивает только поздний проход, `2 + 3` — только ранняя
    // свёртка, поэтому по листингу видно, какой именно проход работал.
    std::fs::write(&path, "Б = 1;\nА = Б + 3;\nВ = 2 + 3;\n").unwrap();
    let script = path.to_str().unwrap();

    let listing = |flags: &[&str]| {
        let mut args = flags.to_vec();
        args.push("--emit-bytecode");
        args.push(script);
        stdout_of(&run(&args))
    };

    // Ранняя свёртка снимает `Add`, но обязана оставить `AddConst`.
    let fold = listing(&["--optimize=const-fold"]);
    assert!(fold.contains("AddConst"), "{fold}");
    assert!(!fold.contains(" Add "), "{fold}");
    // Поздний проход, наоборот, поглощает и `AddConst`.
    let prop = listing(&["--optimize=const-prop"]);
    assert!(!prop.contains("AddConst"), "{prop}");
    // Список из двух проходов принимается и делает работу обоих.
    let both = listing(&["--optimize=const-fold,const-prop"]);
    assert!(
        !both.contains("AddConst") && !both.contains(" Add "),
        "{both}"
    );
    // Без ключа не работает ни один.
    let plain = listing(&[]);
    assert!(
        plain.contains("AddConst") && plain.contains(" Add "),
        "{plain}"
    );

    let _ = std::fs::remove_file(&path);
}

/// Опечатка в имени прохода обязана быть отказом, а не молчаливым «ни один
/// не включён»: замер с проглоченной опечаткой прочитался бы как результат
/// прохода, который не работал.
#[test]
fn an_unknown_pass_name_is_refused_rather_than_ignored() {
    for spec in [
        "--optimize=fold",
        "--optimize=",
        "--optimize=const-fold,typo",
    ] {
        let out = run(&[spec, "--emit-bytecode", "benchmarks/empty_for.bsl"]);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{spec}: ожидался отказ с кодом 2, получено {:?}",
            out.status.code()
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("неизвестный проход") && err.contains("const-fold"),
            "{spec}: сообщение не называет ни ошибку, ни допустимые проходы: {err}"
        );
    }
}

/// Имена проходов идут из одной таблицы с разбором ключа, поэтому проход не
/// может быть выбираемым, но не описанным. Тест держит эту связь.
#[test]
fn the_help_lists_every_selectable_pass() {
    let help = stdout_of(&run(&["--help"]));

    assert!(help.contains("--optimize[=проход,...]"), "{help}");
    for pass in ["const-fold", "const-prop", "copy-elim"] {
        assert!(help.contains(pass), "в --help нет прохода «{pass}»: {help}");
    }
}

/// Свёртка констант не имеет права вычислить на компиляции то, что на
/// исполнении бросает: `1 / 0` внутри `Попытка` обязано по-прежнему
/// доходить до обработчика (`docs/ssa-hotspot-analysis.md`, раздел
/// «Константы»). Проверка подпроцессом и через `--optimize`, потому что
/// вопрос именно в поведении собранного бинарника с включёнными
/// проходами, а не в форме байт-кода — её проверяет
/// `bsl-compiler/tests/const_fold.rs`.
#[test]
fn a_folded_division_by_zero_is_still_caught_under_optimize() {
    let path = std::env::temp_dir().join(format!(
        "bsl-cli-test-fold-throw-{}.bsl",
        std::process::id()
    ));
    std::fs::write(
        &path,
        "Попытка\n\
         \tА = 1 / 0;\n\
         \tСообщить(\"не брошено\");\n\
         Исключение\n\
         \tСообщить(\"перехвачено\");\n\
         КонецПопытки;\n",
    )
    .unwrap();
    let script = path.to_str().unwrap();

    let plain = run(&[script]);
    let optimized = run(&["--optimize", script]);
    assert_eq!(
        stdout_of(&plain),
        "перехвачено\n",
        "деление на ноль не дошло до обработчика и без оптимизаций"
    );
    assert_eq!(
        stdout_of(&optimized),
        stdout_of(&plain),
        "--optimize изменил наблюдаемое поведение исключения"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn script_arguments_land_in_the_command_line_arguments_array() {
    let path = std::env::temp_dir().join("bsl-cli-test-args.bsl");
    std::fs::write(
        &path,
        "Сообщить(АргументыКоманднойСтроки.Количество());\n\
         Для Каждого Арг Из АргументыКоманднойСтроки Цикл\n\
         \tСообщить(Арг);\n\
         КонецЦикла;\n\
         Сообщить(CommandLineArguments.Количество());\n",
    )
    .unwrap();
    let script = path.to_str().unwrap();

    // Аргумент с пробелом обязан дойти одной строкой — за это отвечает
    // разбиение args[2..], а не повторный разбор по пробелам.
    let out = run(&[script, "раз", "два три"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "2\nраз\nдва три\n2\n");

    // Без аргументов — пустой массив, а не ошибка.
    let out = run(&[script]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "0\n0\n");

    let _ = std::fs::remove_file(&path);
}

/// Регрессия на разбор аргументов: путь к скрипту не должен спутаться с
/// командой, а команда — с путём.
#[test]
fn a_script_path_still_runs_the_script() {
    let path = fixture("arithmetic.bsl");
    let out = run(&[path.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "скрипт не исполнился: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let expected = std::fs::read_to_string(fixture("arithmetic.expected")).unwrap();
    let actual = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n')
    );
}

#[test]
fn cli_runs_sync_and_async_http_through_the_system_adapter() {
    let (port, observed, server) = http_system_support::start_server();
    let path = std::env::temp_dir().join(format!(
        "bsl-cli-test-http-system-{}.bsl",
        std::process::id()
    ));
    std::fs::write(&path, http_system_support::source(port))
        .expect("не удалось записать HTTP-скрипт");

    let out = run(&[path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(stdout_of(&out), "");
    server.join().unwrap();
    http_system_support::assert_requests(&observed);
}

#[test]
fn a_script_with_a_utf8_bom_runs() {
    let path =
        std::env::temp_dir().join(format!("bsl-cli-test-utf8-bom-{}.bsl", std::process::id()));
    let mut source = vec![0xef, 0xbb, 0xbf];
    source.extend_from_slice("Сообщить(1);\n".as_bytes());
    std::fs::write(&path, source).expect("не удалось записать BOM-скрипт");

    let out = run(&[path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "BOM-скрипт не исполнился: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "1\n");
}

/// Вид объявления едет от резолвера до исполнения через ЧЕТЫРЕ границы:
/// `ResolvedFunction` -> компилятор -> `Chunk::is_procedure` -> печать и
/// разбор листинга -> `SnippetSignature` в VM. Юнит-тесты по краям этой
/// цепочки зелены и при потере признака посередине: sema-тест собирает
/// сигнатуру руками, а круговой тест листинга печатает то же, что
/// разобрал, поэтому одинаково не увидит `kind=proc` с обеих сторон.
///
/// Здесь цепочка проходится целиком и сверяется с ПЛАТФОРМЕННЫМ ответом:
/// `measure-stmtcall.platform.txt` снят на 8.3.27 и содержит, среди
/// прочего, «своя процедура выражением<таб>отказ» — правило, которое
/// держится только если признак дошёл до фрагмента `Вычислить`.
///
/// В конформансный прогон этот скрипт не попадает: там берутся только
/// `fixtures/*.bsl`, у которых есть `.expected`, а у скриптов замеров
/// эталон лежит рядом с ними и называется иначе.
#[test]
fn the_declaration_kind_survives_the_whole_pipeline() {
    let script = measure("measure-stmtcall.bsl");
    let oracle = std::fs::read_to_string(measure("measure-stmtcall.platform.txt"))
        .expect("платформенный эталон должен лежать рядом со скриптом");
    assert!(
        oracle.contains("своя процедура выражением\tотказ"),
        "эталон потерял строку про процедуру в выражении:\n{oracle}"
    );

    // Прямой прогон: резолвер -> компилятор -> VM.
    let direct = stdout_of(&run(&[script.to_str().unwrap()]));
    assert_eq!(direct, oracle, "прямой прогон разошёлся с платформой");

    // Тот же путь через ЛИСТИНГ: печать и разбор — отдельные границы, и
    // признак обязан пережить обе.
    let listing = std::env::temp_dir().join("bsl-cli-test-stmtcall.bslc");
    let out = run(&[
        "--emit-bytecode",
        script.to_str().unwrap(),
        listing.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "листинг не записан: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&listing).unwrap();
    // Две процедуры скрипта — `П` и `СвояПроцедура`; у функций и у
    // верхнего уровня поля нет.
    assert_eq!(
        text.matches("kind=proc").count(),
        2,
        "в листинге не тот набор процедур:\n{text}"
    );

    let replayed = stdout_of(&run(&["--run-bytecode", listing.to_str().unwrap()]));
    assert_eq!(replayed, oracle, "прогон листинга разошёлся с платформой");

    let _ = std::fs::remove_file(&listing);
}
