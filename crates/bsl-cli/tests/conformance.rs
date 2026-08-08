//! Раннер для tests/conformance/fixtures/ (задача 2 ревью): для каждого
//! `*.bsl` рядом с `*.expected` гоняет `bsl-cli <файл>` как отдельный
//! процесс и построчно сравнивает его stdout с оракулом.
//!
//! Подпроцесс, а не вызов пайплайна `bsl_syntax::parse -> ... ->
//! bsl_vm::run_program` в этом же процессе, потому что `Сообщить`/`Message`
//! пишет напрямую в stdout через `println!` (см. `bsl-vm/src/lib.rs`) — в
//! процессе теста это некуда перехватить без OS-уровневой подмены fd 1.
//! `CARGO_BIN_EXE_bsl-cli` cargo выставляет сам для интеграционных тестов
//! пакета, который собирает этот бинарник.
//!
//! Фикстура без пары `.expected` пропускается — это не ошибка, а способ
//! держать в репозитории заготовки, для которых пока нет платформенного
//! оракула. Раньше пропуск был невидим; теперь в конце прогона печатается
//! сводка «столько проверено, столько пропущено и каких именно» (видна при
//! `cargo test -p bsl-cli -- --nocapture`), чтобы отсутствие покрытия не
//! забывалось само собой.
//!
//! Скрипты замеров (`tests/conformance/measure/`) обходятся тем же
//! раннером: `.expected` у них тоже нет, но исполняться они обязаны — иначе
//! сеанс у платформы начнётся с падения на первой же строке (см.
//! `measure_script_runs_under_this_interpreter` ниже).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

/// Замок на ОБХОД КОРПУСА: четыре теста ниже исполняют одни и те же
/// фикстуры, а cargo пускает тесты одного бинарника параллельно.
///
/// Часть фикстур пишет во временные файлы по ФИКСИРОВАННЫМ путям
/// (`binary-data`, `binary-datarw` — `/tmp/open-bsl-datarw-fixture.bin` и
/// соседи), поэтому два обхода, попавшие на такую фикстуру одновременно,
/// затирают файл друг у друга, и тест падает на расхождении с оракулом —
/// не потому, что что-то сломано, а потому, что фикстура не выдерживает
/// запуска сама с собой. Опасность отмечена в `PROGRESS.md` ещё прошлой
/// итерацией (тогда — на `binary-data`); проявляется она от того, насколько
/// сойдутся по времени два обхода, поэтому добавление любой пары фикстур
/// способно её разбудить.
///
/// Замок лечит именно это: обход идёт по одному за раз, внутри теста всё
/// по-прежнему последовательно. Правильнее было бы развести сами фикстуры
/// по уникальным путям, но их `.expected` — платформенные эталоны, и такая
/// правка требует нового прогона на 1С; здесь это отдельная задача.
static CORPUS: Mutex<()> = Mutex::new(());

/// Взять замок корпуса, не заражаясь отравлением: если один тест уже упал с
/// паникой, остальные обязаны сообщить СВОЙ результат, а не панику про
/// `PoisonError`.
fn corpus_lock() -> MutexGuard<'static, ()> {
    CORPUS.lock().unwrap_or_else(|e| e.into_inner())
}

fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

fn fixtures_dir() -> PathBuf {
    conformance_dir().join("fixtures")
}

fn measure_dir() -> PathBuf {
    conformance_dir().join("measure")
}

/// Все `*.bsl` каталога, отсортированные по имени.
fn scripts_in(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("bsl"))
        .collect();
    entries.sort();
    entries
}

/// Построчный дифф, показывающий первую точку расхождения (плюс контекст),
/// а не всё содержимое обоих файлов сразу.
fn line_diff(expected: &str, actual: &str) -> String {
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let mut out = String::new();
    let max = exp_lines.len().max(act_lines.len());
    for i in 0..max {
        let e = exp_lines.get(i).copied();
        let a = act_lines.get(i).copied();
        if e != a {
            out.push_str(&format!(
                "  строка {}: ожидалось {:?}, получено {:?}\n",
                i + 1,
                e.unwrap_or("<конец файла>"),
                a.unwrap_or("<конец файла>"),
            ));
        }
    }
    if exp_lines.len() != act_lines.len() {
        out.push_str(&format!(
            "  разное число строк: ожидалось {}, получено {}\n",
            exp_lines.len(),
            act_lines.len()
        ));
    }
    out
}

#[test]
fn conformance_fixtures_match_oracle_output() {
    let _corpus = corpus_lock();
    let dir = fixtures_dir();
    let bsl_cli = env!("CARGO_BIN_EXE_bsl-cli");

    let mut checked = Vec::new();
    let mut skipped = Vec::new();
    let mut failures = Vec::new();

    let mut entries = scripts_in(&dir);
    entries.extend(scripts_in(&measure_dir()));

    for fixture in entries {
        let expected_path = fixture.with_extension("expected");
        let name = fixture.file_name().unwrap().to_string_lossy().into_owned();
        if !expected_path.exists() {
            skipped.push(fixture.file_stem().unwrap().to_string_lossy().into_owned());
            continue;
        }
        checked.push(name.clone());

        let expected = fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", expected_path.display()));

        let output = Command::new(bsl_cli)
            .arg(&fixture)
            .output()
            .unwrap_or_else(|e| panic!("не удалось запустить bsl-cli на {name}: {e}"));

        let actual = String::from_utf8_lossy(&output.stdout).into_owned();

        if actual.trim_end_matches('\n') != expected.trim_end_matches('\n') {
            let mut msg = format!("{name}: stdout разошёлся с {}\n", expected_path.display());
            msg.push_str(&line_diff(&expected, &actual));
            if !output.status.success() {
                msg.push_str(&format!(
                    "  процесс завершился с {}; stderr:\n{}\n",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            failures.push(msg);
        }
    }

    assert!(
        !checked.is_empty(),
        "ни одной фикстуры с парой .expected не найдено в {}",
        dir.display()
    );

    // Сводка в конце прогона: пропуск фикстуры — это ОТСУТСТВИЕ покрытия, и
    // видеть его нужно каждый раз, а не вспоминать о нём. Пропущенные — та
    // самая очередь на сеанс замеров.
    eprintln!(
        "conformance: {} проверено, {} пропущено (нет .expected)",
        checked.len(),
        skipped.len()
    );
    if !skipped.is_empty() {
        eprintln!("  пропущены: {}", skipped.join(", "));
    }

    assert!(
        failures.is_empty(),
        "{} фикстур(а) разошлись с оракулом:\n\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// `measure-all.bsl` — единственный файл, который человек прогоняет у
/// платформы, и падение на середине стоит целого сеанса. Поэтому он
/// проверяется не как «фикстура без оракула» (такие раннер выше просто
/// пропускает), а отдельно: исполняется целиком, и его вывод обязан быть
/// машинным — `ИД<таб>ЗНАЧЕНИЕ`, ровно по строке на каждый ИД из самого
/// скрипта.
///
/// Что этот тест НЕ проверяет и проверять не может: сами значения. Их
/// эталон приходит только с платформы (`bsl-cli --ingest-measurements`).
#[test]
fn measure_script_runs_under_this_interpreter() {
    let _corpus = corpus_lock();
    let script = measure_dir().join("measure-all.bsl");
    let output = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .arg(&script)
        .output()
        .unwrap_or_else(|e| panic!("не удалось запустить bsl-cli на measure-all.bsl: {e}"));
    assert!(
        output.status.success(),
        "measure-all.bsl не исполнился: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut printed = Vec::new();
    for line in stdout.lines() {
        let (id, _value) = line.split_once('\t').unwrap_or_else(|| {
            panic!("строка вывода без табуляции: {line:?} — печатать замер обязана процедура М()")
        });
        assert!(
            !id.is_empty() && !id.contains(' '),
            "неопрятный ИД в выводе: {id:?}"
        );
        printed.push(id.to_string());
    }

    let mut sorted = printed.clone();
    sorted.sort();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(before, sorted.len(), "ИД напечатан дважды: {printed:?}");

    // Сверка с реестром — на стороне bsl-rt
    // (`open_questions_registry.rs`); здесь достаточно того, что каждый ИД
    // из ИСХОДНИКА скрипта действительно дошёл до вывода: строка,
    // пропущенная из-за ветвления или упавшая молча, — ровно та ошибка,
    // которая обнаружилась бы уже у платформы.
    let src = fs::read_to_string(&script).expect("measure-all.bsl не читается");
    let in_source: Vec<String> = src
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("М(\""))
        .filter_map(|rest| rest.split('"').next().map(str::to_string))
        .collect();
    assert!(
        !in_source.is_empty(),
        "в measure-all.bsl не нашлось вызовов М()"
    );
    let missing: Vec<&String> = in_source
        .iter()
        .filter(|id| !printed.contains(id))
        .collect();
    assert!(
        missing.is_empty(),
        "ИД есть в скрипте, но не дошли до вывода: {missing:?}"
    );
}

/// Байт-код, напечатанный `--emit-bytecode`, обязан исполняться
/// `--run-bytecode` с тем же выводом, что и исходник. Проверяется на всех
/// фикстурах с оракулом: это единственное место, где путь «печать ->
/// разбор -> VM» проходит целиком, на настоящих программах, а не на
/// корпусе round-trip внутри bsl-bytecode.
#[test]
fn fixtures_produce_the_same_output_when_run_from_printed_bytecode() {
    let _corpus = corpus_lock();
    let bsl_cli = env!("CARGO_BIN_EXE_bsl-cli");
    let tmp = std::env::temp_dir().join(format!("bslc-{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("не создаётся временный каталог");

    let mut checked = 0;
    for fixture in scripts_in(&fixtures_dir()) {
        let expected_path = fixture.with_extension("expected");
        if !expected_path.exists() {
            continue;
        }
        let name = fixture.file_stem().unwrap().to_string_lossy().into_owned();
        let out = tmp.join(format!("{name}.bslc"));

        let emit = Command::new(bsl_cli)
            .arg("--emit-bytecode")
            .arg(&fixture)
            .arg(&out)
            .output()
            .unwrap_or_else(|e| panic!("не удалось напечатать байт-код {name}: {e}"));
        assert!(
            emit.status.success(),
            "{name}: --emit-bytecode завершился с {}\n{}",
            emit.status,
            String::from_utf8_lossy(&emit.stderr)
        );

        let run = Command::new(bsl_cli)
            .arg("--run-bytecode")
            .arg(&out)
            .output()
            .unwrap_or_else(|e| panic!("не удалось исполнить байт-код {name}: {e}"));
        assert!(
            run.status.success(),
            "{name}: --run-bytecode завершился с {}\n{}",
            run.status,
            String::from_utf8_lossy(&run.stderr)
        );

        let expected = fs::read_to_string(&expected_path).unwrap();
        let actual = String::from_utf8_lossy(&run.stdout).into_owned();
        assert_eq!(
            actual.trim_end_matches('\n'),
            expected.trim_end_matches('\n'),
            "{name}: вывод из байт-кода разошёлся с оракулом\n{}",
            line_diff(&expected, &actual)
        );
        checked += 1;
    }

    let _ = fs::remove_dir_all(&tmp);
    assert!(checked > 0, "не нашлось ни одной фикстуры с .expected");
}

/// Оба режима исполнения обязаны давать ОДИН И ТОТ ЖЕ вывод на всём
/// корпусе.
///
/// Это главная гарантия JIT-а. Он не пишет своей семантики — нативный код
/// зовёт те же функции, что и ветки интерпретатора, — но проверить это
/// утверждением в комментарии нельзя. Сверяются оба каталога, включая
/// фикстуры без `.expected`: здесь эталоном служит не платформа, а второй
/// режим, и «система сама с собой» тут уместна ровно потому, что вопрос
/// именно в их совпадении.
///
/// На платформах без JIT-а `--jit` ничего не меняет, и тест просто
/// сравнивает интерпретатор с самим собой — пусть лучше так, чем `cfg`,
/// из-за которого на этих платформах он молча не запускался бы.
#[test]
fn the_jit_agrees_with_the_interpreter_on_every_script() {
    let _corpus = corpus_lock();
    let mut checked = 0;
    for dir in [fixtures_dir(), measure_dir()] {
        for script in scripts_in(&dir) {
            let name = script.file_name().unwrap().to_string_lossy().to_string();
            if SLOW_TO_COMPARE.contains(&name.as_str()) {
                continue;
            }
            let plain = run_script(&script, &[]);
            let jitted = run_script(&script, &["--jit"]);
            assert_eq!(
                plain, jitted,
                "{name}: вывод под --jit отличается от вывода интерпретатора"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "не нашлось ни одного скрипта для сверки режимов"
    );
    println!("режимы сверены на {checked} скриптах");
}

/// Фикстуры, которые в этой сверке не участвуют — не потому, что режимы
/// на них расходятся, а потому, что каждая считается минутами (тест
/// собирается в debug, где всё в разы медленнее), и гонять их ДВАЖДЫ
/// незачем. Их численную семантику покрывают соседние фикстуры того же
/// семейства: `n-body-smoke` проходит те же ветки за 32 мс.
///
/// `n-body-pow-variant` не завершается и в release: это исследование
/// сходимости, а не тест.
const SLOW_TO_COMPARE: &[&str] = &[
    "n-body-perf.bsl",
    "n-body-precision.bsl",
    "n-body-pow-variant.bsl",
];

/// stdout скрипта плюс код возврата: расходиться режимы могут и падением.
fn run_script(script: &Path, flags: &[&str]) -> (String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_bsl-cli"))
        .args(flags)
        .arg(script)
        .output()
        .unwrap_or_else(|e| panic!("не удалось запустить {}: {e}", script.display()));
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}
