//! `bsl-cli --ingest-measurements <файл>` — приём результата сеанса у
//! платформы.
//!
//! На входе — вывод `tests/conformance/measure/measure-all.bsl`, снятый с
//! реальной 1С (строки `ИД<таб>ЗНАЧЕНИЕ`; посторонние строки, которые
//! платформа могла подмешать в окно сообщений, пропускаются).
//!
//! На выходе три вещи:
//!
//! 1. `tests/conformance/measure/platform.tsv` — тот же вывод, отсортирован
//!    по ИД и очищен от шума. Это ЕДИНСТВЕННЫЙ файл в репозитории, куда
//!    попадают платформенные значения, и попадают они только отсюда.
//! 2. Таблица расхождений с нашим выводом на том же скрипте, по одному
//!    блоку на ИД, с напоминанием, что именно у нас было выбрано.
//! 3. Списки «не замерено» (ИД реестра, которых в выводе нет) и
//!    «неизвестный ИД» (в выводе есть, в реестре нет).
//!
//! Чего эта команда НЕ делает и делать не должна: ничего не правит в коде и
//! не генерирует `.expected`. Решение, что чинить, а что было верно с
//! самого начала, принимает человек — автогенерация эталона из собственного
//! вывода превратила бы тесты в сравнение системы самой с собой.
//!
//! Код возврата: `1`, если разошёлся хотя бы один ЯКОРЬ (уже измеренное
//! значение) — тогда сеанс снят не с той конфигурации и остальным строкам
//! верить нельзя; иначе `0`, даже если расхождений по открытым вопросам
//! много (они-то и есть результат).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use bsl_rt::open_questions::{MEASURED_ANCHORS, OPEN_QUESTIONS};

/// Разбор вывода платформы. Ключ — ИД, значение — всё после первой
/// табуляции (в самом значении табуляций быть не должно, но если платформа
/// их добавила, лучше сохранить как есть, чем потерять).
fn parse_tsv(text: &str) -> (BTreeMap<String, String>, Vec<String>) {
    let mut out = BTreeMap::new();
    let mut noise = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        match line.split_once('\t') {
            Some((id, value)) if !id.trim().is_empty() => {
                out.insert(id.trim().to_string(), value.to_string());
            }
            _ => noise.push(line.to_string()),
        }
    }
    (out, noise)
}

/// Каталог замеров: ищется вверх от текущего каталога, чтобы командой можно
/// было пользоваться из любого места внутри репозитория.
fn find_measure_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("tests/conformance/measure");
        if candidate.join("measure-all.bsl").is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Наш собственный вывод на том же скрипте — подпроцессом того же
/// бинарника, ровно как в раннере conformance: `Сообщить` пишет прямо в
/// stdout, перехватить его внутри процесса нечем.
fn our_output(script: &Path) -> Result<BTreeMap<String, String>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("не найден собственный путь: {e}"))?;
    let out = Command::new(exe)
        .arg(script)
        .output()
        .map_err(|e| format!("не удалось запустить {}: {e}", script.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} не исполнился ({}):\n{}",
            script.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(parse_tsv(&String::from_utf8_lossy(&out.stdout)).0)
}

pub fn run(input: &str, script_override: Option<&str>) -> i32 {
    let text = match std::fs::read_to_string(input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("не удалось прочитать «{input}»: {e}");
            return 1;
        }
    };
    let (platform, noise) = parse_tsv(&text);
    if platform.is_empty() {
        eprintln!(
            "в «{input}» нет ни одной строки вида ИД<таб>ЗНАЧЕНИЕ — \
             похоже, это не вывод measure-all.bsl"
        );
        return 1;
    }

    let script = match script_override {
        Some(p) => PathBuf::from(p),
        None => match find_measure_dir() {
            Some(dir) => dir.join("measure-all.bsl"),
            None => {
                eprintln!(
                    "не найден tests/conformance/measure/measure-all.bsl ни в текущем \
                     каталоге, ни выше. Передайте путь третьим аргументом."
                );
                return 1;
            }
        },
    };

    // 1. Платформенный вывод на диск — отсортированным и без шума.
    let tsv_path = script.with_file_name("platform.tsv");
    let mut tsv = String::new();
    for (id, value) in &platform {
        tsv.push_str(id);
        tsv.push('\t');
        tsv.push_str(value);
        tsv.push('\n');
    }
    if let Err(e) = std::fs::write(&tsv_path, &tsv) {
        eprintln!("не удалось записать {}: {e}", tsv_path.display());
        return 1;
    }
    println!(
        "записано {} ({} строк{})",
        tsv_path.display(),
        platform.len(),
        if noise.is_empty() {
            String::new()
        } else {
            format!(", пропущено посторонних: {}", noise.len())
        }
    );

    let ours = match our_output(&script) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    // 2. Якоря — канарейка сеанса.
    let mut broken_anchors = Vec::new();
    for a in MEASURED_ANCHORS {
        if let Some(got) = platform.get(a.id)
            && got != a.expect
        {
            broken_anchors.push((a.id, a.expect, got.clone(), a.note));
        }
    }
    if !broken_anchors.is_empty() {
        println!(
            "\nЯКОРЯ РАЗОШЛИСЬ ({}) — сеанс под подозрением:",
            broken_anchors.len()
        );
        for (id, expect, got, note) in &broken_anchors {
            println!("  {id}");
            println!("    ожидалось  {expect}");
            println!("    платформа  {got}");
            println!("    почему     {note}");
        }
        println!(
            "  Якорь снят с реальной 1С раньше. Расхождение значит либо другую \n\
             \x20 конфигурацию платформы, либо ошибку переноса вывода — разбирать \n\
             \x20 остальные строки до выяснения бессмысленно."
        );
    }

    // 3. Расхождения по открытым вопросам.
    let mut diffs = Vec::new();
    let mut not_measured = Vec::new();
    for q in OPEN_QUESTIONS {
        let Some(theirs) = platform.get(q.id) else {
            not_measured.push(q.id);
            continue;
        };
        let ours_value = ours
            .get(q.id)
            .cloned()
            .unwrap_or_else(|| "<нет в нашем выводе>".into());
        if &ours_value != theirs {
            diffs.push((q, ours_value, theirs.clone()));
        }
    }

    println!("\nРАСХОЖДЕНИЯ ({}):", diffs.len());
    for (q, ours_value, theirs) in &diffs {
        println!("  {}", q.id);
        println!("    наше       {ours_value}");
        println!("    платформа  {theirs}");
        println!("    выбрано    {}", q.chosen);
        println!("    блокирует  {}", q.blocks);
    }
    if diffs.is_empty() {
        println!("  нет — по всем замеренным вопросам выбранное поведение совпало");
    }

    let matched = OPEN_QUESTIONS.len() - not_measured.len() - diffs.len();
    println!(
        "\nСОВПАЛО: {matched} из {} вопросов реестра",
        OPEN_QUESTIONS.len()
    );

    if !not_measured.is_empty() {
        println!(
            "\nНЕ ЗАМЕРЕНО ({}) — этих ИД в выводе платформы нет:",
            not_measured.len()
        );
        for id in &not_measured {
            println!("  {id}");
        }
    }

    let known: Vec<&str> = OPEN_QUESTIONS
        .iter()
        .map(|q| q.id)
        .chain(MEASURED_ANCHORS.iter().map(|a| a.id))
        .collect();
    let unknown: Vec<&String> = platform
        .keys()
        .filter(|id| {
            !known.contains(&id.as_str())
                && !known
                    .iter()
                    .any(|k| id.starts_with(k) && id.as_bytes().get(k.len()) == Some(&b'.'))
        })
        .collect();
    if !unknown.is_empty() {
        println!(
            "\nНЕИЗВЕСТНЫЕ ИД ({}) — есть в выводе, нет в реестре:",
            unknown.len()
        );
        for id in &unknown {
            println!("  {id}\t{}", platform[*id]);
        }
    }

    if !noise.is_empty() {
        println!("\nПОСТОРОННИЕ СТРОКИ ({}):", noise.len());
        for line in noise.iter().take(10) {
            println!("  {line}");
        }
        if noise.len() > 10 {
            println!("  ... и ещё {}", noise.len() - 10);
        }
    }

    println!(
        "\nВ КОДЕ НИЧЕГО НЕ ИЗМЕНЕНО. По каждому расхождению решение принимает \n\
         человек: чинить нашу реализацию или признать, что выбранное поведение \n\
         и было верным. Закрытый вопрос убирается из bsl_rt::open_questions, из \n\
         measure-all.bsl и из меток в коде — все три места разом, иначе упадёт \n\
         open_questions_registry_matches_source_markers."
    );

    i32::from(!broken_anchors.is_empty())
}
