use std::collections::{HashMap, HashSet};

use bsl_number::BslNumber;
use bsl_syntax::{Expr as AExpr, Item, Stmt as AStmt};

use crate::resolved::{RExpr, RStmt, Resolved, ResolvedFunction, ResolvedParam, ResolvedProgram};

#[derive(Debug, Clone, PartialEq)]
pub enum SemaError {
    /// Идентификатор читается раньше первого присваивания/объявления.
    UndefinedVariable(String),
    /// Вызов имени, для которого нет объявленной `Процедура`/`Функция` в
    /// этом модуле (методов объектов и встроенных функций пока нет).
    UndefinedFunction(String),
    DuplicateFunction(String),
    ArgumentCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// `Ф(1, , 3)` — позиция пропущена (`,,`), но у соответствующего
    /// параметра `Ф` нет значения по умолчанию: пропустить нечем.
    MissingRequiredArgument {
        name: String,
        position: usize,
    },
    /// Функция встроенного языка (`Строка`, `СтрДлина`, `Вычислить`, …)
    /// вызвана отдельным оператором. Платформа отказывается компилировать
    /// такой модуль — «Встроенная функция может быть использована только в
    /// выражении», — но правило касается лишь функций ЯЗЫКА: обычную
    /// функцию глобального контекста (`СтрНайти`, `ПрочитатьJSON`, …)
    /// оператором звать можно. Деление измерено полным перебором — см.
    /// [`bsl_rt::BuiltinFn::is_intrinsic`].
    BuiltinFunctionAsStatement(String),
    /// Глобальная процедура (`Сообщить`, `ЗаписатьJSON`,
    /// `ЗаполнитьЗначенияСвойств`) в позиции выражения: платформа отвечает
    /// «Обращение к процедуре как к функции».
    ProcedureAsFunction(String),
    /// `'20240230'` — литерал прошёл лексер (цифры, верная длина), но
    /// такой календарной даты не существует.
    BadDateLiteral(String),
    /// Конструкция языка, для которой ещё нет резолвинга (коллекции,
    /// `Выполнить`/`Вычислить`, значения по умолчанию/пропуски аргументов,
    /// ... — приходят в последующих milestone'ах).
    Unsupported(&'static str),
}

/// Разрешённый динамический фрагмент и полное замыкание его компонентов.
pub type ResolvedSnippetWithRequirements =
    (Vec<String>, Vec<RStmt>, Vec<bsl_rt::LibraryRequirement>);

/// Сигнатура функции/процедуры, собранная до резолвинга тел — нужна, чтобы
/// вызовы разрешались независимо от порядка объявления (в том числе
/// рекурсия и взаимные вызовы).
struct FuncSig {
    index: u32,
    /// Есть ли у параметра в этой позиции значение по умолчанию — длина
    /// заодно и есть арность (по-настоящему нужен только режим передачи
    /// каждого параметра, а он читается из `ResolvedFunction::params` на
    /// этапе кодогена, не отсюда). Пропущенный аргумент (`Ф(1, , 3)`)
    /// допустим только там, где здесь `true` — иначе это ошибка резолвинга
    /// вызывающего кода, а не рантайма вызываемой функции.
    has_default: Vec<bool>,
}

/// Резолвит весь модуль: собирает сигнатуры всех `Процедура`/`Функция` за
/// один проход (чтобы вызовы работали независимо от порядка объявления и
/// поддерживали рекурсию), затем резолвит каждое тело и операторы верхнего
/// уровня.
/// `Перем` на уровне модуля образует ОБЛАСТЬ МОДУЛЯ: процедуры и функции
/// видят такие переменные и пишут в них, и запись видна снаружи —
/// ИЗМЕРЕНО на 8.3.27. Хранилище — первые слоты кадра верхнего уровня: он
/// стоит в самом низу стека значений и живёт всё исполнение, поэтому
/// доступ из любого кадра — это прямая индексация (`Instr::GetModuleVar`).
///
/// # Errors
///
/// Возвращает [`SemaError`] при повторном объявлении функции, неизвестном имени, неверном
/// числе аргументов, недопустимом литерале даты или ещё не поддерживаемой конструкции.
pub fn resolve_program(items: &[Item]) -> Result<ResolvedProgram, SemaError> {
    resolve_program_impl(items, None)
}

/// Разрешает модуль с каталогом функций и конструкторов собранного
/// runtime. Старый [`resolve_program`] на время миграции сохраняет закрытые
/// встроенные таблицы, а этот вход открывает компонентные функции.
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же правилам, что и [`resolve_program`].
pub fn resolve_program_with_registry(
    items: &[Item],
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<ResolvedProgram, SemaError> {
    resolve_program_impl(items, Some(registry))
}

fn resolve_program_impl(
    items: &[Item],
    registry: Option<&bsl_rt::RuntimeRegistry>,
) -> Result<ResolvedProgram, SemaError> {
    let mut sigs: HashMap<String, FuncSig> = HashMap::new();
    let mut func_items: Vec<&Item> = Vec::new();
    let mut top_stmts: Vec<AStmt> = Vec::new();
    // `Перем` на уровне модуля — это ОБЛАСТЬ МОДУЛЯ, а не просто первые
    // локальные тела: измерено, что процедуры их видят.
    let mut module_vars: Vec<String> = Vec::new();

    for item in items {
        match item {
            Item::Function(f) => {
                declare_sig(
                    &mut sigs,
                    &f.name,
                    f.params.iter().map(|p| p.default.is_some()).collect(),
                )?;
                func_items.push(item);
            }
            Item::Procedure(p) => {
                declare_sig(
                    &mut sigs,
                    &p.name,
                    p.params.iter().map(|p| p.default.is_some()).collect(),
                )?;
                func_items.push(item);
            }
            Item::VarDecl(vd) => {
                for name in &vd.names {
                    if !module_vars.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                        module_vars.push(name.clone());
                    }
                }
                top_stmts.push(AStmt::VarDecl(vd.clone()));
            }
            Item::Stmt(s) => top_stmts.push(s.clone()),
        }
    }

    // Номера слотов модульных переменных: они же — первые слоты кадра
    // верхнего уровня (см. затравку резолвера тела ниже).
    let module_index: HashMap<String, u32> = module_vars
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    let empty_module_index: HashMap<String, u32> = HashMap::new();

    let mut functions = Vec::with_capacity(func_items.len());
    let mut used_libraries = HashSet::new();
    for item in &func_items {
        let (name, params, body) = match item {
            Item::Function(f) => (&f.name, &f.params, &f.body),
            Item::Procedure(p) => (&p.name, &p.params, &p.body),
            _ => unreachable!(),
        };
        let mut r = Resolver {
            locals: Vec::new(),
            index: HashMap::new(),
            funcs: &sigs,
            module_index: &module_index,
            registry,
            used_libraries: HashSet::new(),
            strict_stmt_calls: true,
            stmt_call: false,
        };
        for p in params {
            r.declare(&p.name);
        }
        let resolved_body = r.resolve_block(body)?;
        // Значения по умолчанию резолвятся ПОСЛЕ тела, но той же `r` — её
        // `locals`/`index` к этому моменту уже содержат слоты параметров
        // (объявлены выше, до `resolve_block`), так что дефолт вида
        // `Ф(б = а + 1)`, ссылающийся на предыдущий параметр, резолвится
        // корректно.
        let mut resolved_params = Vec::with_capacity(params.len());
        for p in params {
            let default = match &p.default {
                Some(e) => Some(r.resolve_expr(e)?),
                None => None,
            };
            resolved_params.push(ResolvedParam {
                by_val: p.by_val,
                default,
            });
        }
        used_libraries.extend(r.used_libraries.iter().cloned());
        functions.push(ResolvedFunction {
            name: name.clone(),
            uses_dynamic: crate::resolved::block_uses_dynamic(&resolved_body),
            params: resolved_params,
            locals: r.locals,
            body: resolved_body,
        });
    }

    // Тело модуля видит те же переменные как ОБЫЧНЫЕ локальные: его кадр и
    // есть их хранилище, и номер слота совпадает с номером в `module_index`
    // — на этом совпадении держится доступ из функций.
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &sigs,
        module_index: &empty_module_index,
        registry,
        used_libraries: HashSet::new(),
        strict_stmt_calls: true,
        stmt_call: false,
    };
    for name in &module_vars {
        r.declare(name);
    }
    let top_body = r.resolve_block(&top_stmts)?;
    used_libraries.extend(r.used_libraries.iter().cloned());
    let top_level = Resolved {
        uses_dynamic: crate::resolved::block_uses_dynamic(&top_body),
        locals: r.locals,
        body: top_body,
    };

    Ok(ResolvedProgram {
        requirements: match registry {
            Some(registry) => registry.requirements_for(used_libraries),
            None => vec![bsl_rt::LibraryRequirement::bsl_rt()],
        },
        functions,
        top_level,
        module_vars,
    })
}

fn declare_sig(
    sigs: &mut HashMap<String, FuncSig>,
    name: &str,
    has_default: Vec<bool>,
) -> Result<(), SemaError> {
    let key = name.to_uppercase();
    if sigs.contains_key(&key) {
        return Err(SemaError::DuplicateFunction(name.to_string()));
    }
    let index = sigs.len() as u32;
    sigs.insert(key, FuncSig { index, has_default });
    Ok(())
}

/// Разрешает имена в плоском скрипте верхнего уровня без объявлений функций —
/// удобно для тестов. Функции разрешаются только через [`resolve_program`].
///
/// # Errors
///
/// Возвращает [`SemaError`] при неизвестном имени, неверном числе аргументов, недопустимом
/// литерале даты или ещё не поддерживаемой конструкции.
pub fn resolve_script(stmts: &[AStmt]) -> Result<Resolved, SemaError> {
    let empty_funcs = HashMap::new();
    let empty_module = HashMap::new();
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &empty_funcs,
        module_index: &empty_module,
        registry: None,
        used_libraries: HashSet::new(),
        strict_stmt_calls: true,
        stmt_call: false,
    };
    let body = r.resolve_block(stmts)?;
    Ok(Resolved {
        uses_dynamic: crate::resolved::block_uses_dynamic(&body),
        locals: r.locals,
        body,
    })
}

/// Разрешает имена во фрагменте кода для `Выполнить`/`Вычислить`: `existing_locals` —
/// уже объявленные переменные окружающего скрипта, которые первыми добавляются в таблицу имён
/// первыми, поэтому ссылки на них в фрагменте попадают на ТЕ ЖЕ слоты, а
/// не заводят копию. Новые имена, объявленные внутри фрагмента, получают
/// слоты ПОСЛЕ существующих — полный список возвращается вызывающему,
/// который сам решает, сохранять ли их (VM для `Выполнить`/`Вычислить`
/// внутри уже скомпилированного кода их не сохраняет: она не может расширять
/// статически размеченный кадр; REPL сохраняет, поскольку его
/// кадр и так растёт от строки к строке).
///
/// ИЗМЕРЕНО на 8.3.27: фрагмент ВИДИТ процедуры и функции модуля —
/// `Вычислить("Удвоить(21)")` возвращает 42. Поэтому `signatures` тянется
/// сюда из уже скомпилированной программы: пара «имя -> (номер, арность)»
/// в том же порядке, в каком функции лежат в `Program::chunks[1..]`.
/// Пустой список означает «функций нет» (так зовёт REPL до первого
/// объявления), а не «вызывать нельзя».
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что и [`resolve_script`], а также если фрагмент
/// вызывает функцию, которой нет в `signatures`.
pub fn resolve_snippet_stmts(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[(String, usize)],
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    resolve_snippet_stmts_mode(existing_locals, module_vars, stmts, signatures, true)
}

/// То же, что [`resolve_snippet_stmts`], но для строки REPL: голый вызов
/// функции встроенного языка (`СтрДлина("аб")`) не отвергается — REPL
/// печатает его значение. Платформенное правило про оператор относится к
/// компиляции модуля, а строка REPL модулем не является; фрагменты
/// `Выполнить` остаются строгими, как на платформе (ИЗМЕРЕНО, якорь
/// `CALL.STMT.INTRINSIC`).
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что и
/// [`resolve_snippet_stmts`], кроме [`SemaError::BuiltinFunctionAsStatement`].
pub fn resolve_repl_stmts(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[(String, usize)],
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    resolve_snippet_stmts_mode(existing_locals, module_vars, stmts, signatures, false)
}

/// Разрешает динамический фрагмент с тем же каталогом компонентов, что и
/// основную программу, и возвращает его собственное замыкание требований.
///
/// # Errors
///
/// Возвращает [`SemaError`] по тем же причинам, что
/// [`resolve_snippet_stmts`].
pub fn resolve_snippet_stmts_with_registry(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[(String, usize)],
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<ResolvedSnippetWithRequirements, SemaError> {
    resolve_snippet_stmts_mode_registry(
        existing_locals,
        module_vars,
        stmts,
        signatures,
        true,
        Some(registry),
    )
}

fn resolve_snippet_stmts_mode(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[(String, usize)],
    strict_stmt_calls: bool,
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    let (locals, body, _) = resolve_snippet_stmts_mode_registry(
        existing_locals,
        module_vars,
        stmts,
        signatures,
        strict_stmt_calls,
        None,
    )?;
    Ok((locals, body))
}

fn resolve_snippet_stmts_mode_registry(
    existing_locals: &[String],
    module_vars: &[String],
    stmts: &[AStmt],
    signatures: &[(String, usize)],
    strict_stmt_calls: bool,
    registry: Option<&bsl_rt::RuntimeRegistry>,
) -> Result<ResolvedSnippetWithRequirements, SemaError> {
    let empty_funcs: HashMap<String, FuncSig> = signatures
        .iter()
        .enumerate()
        .map(|(index, (name, arity))| {
            (
                name.to_uppercase(),
                FuncSig {
                    index: index as u32,
                    has_default: vec![false; *arity],
                },
            )
        })
        .collect();
    let index = existing_locals
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    // Фрагмент ВИДИТ переменные уровня модуля и пишет в них — ИЗМЕРЕНО на
    // 8.3.27. Свой стек этому не мешает: модульные значения едут во
    // фрагмент отдельным блоком, а `Program::module_base` говорит VM, с
    // какого места он там лежит (см. `run_dynamic_snippet`).
    //
    // Имена из `existing_locals` по-прежнему выигрывают: `index` заполнен
    // до `module_index`, а резолвер смотрит сначала в него. На верхнем
    // уровне это важно — там модульные переменные И ЕСТЬ локальные кадра,
    // и разрешать их как модульные значило бы писать в блок-копию.
    let module_index: HashMap<String, u32> = module_vars
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    let mut r = Resolver {
        locals: existing_locals.to_vec(),
        index,
        funcs: &empty_funcs,
        module_index: &module_index,
        registry,
        used_libraries: HashSet::new(),
        strict_stmt_calls,
        stmt_call: false,
    };
    let body = r.resolve_block(stmts)?;
    let requirements = match registry {
        Some(registry) => registry.requirements_for(r.used_libraries.iter().cloned()),
        None => vec![bsl_rt::LibraryRequirement::bsl_rt()],
    };
    Ok((r.locals, body, requirements))
}

/// Типы, которые умеет строить `Новый` — в каноническом написании, оба
/// языка. Список нужен снаружи (автодополнение REPL предлагает их после
/// `Новый`), а `resolve_new` разбирает каждый по-своему: у них разная
/// арность и разный смысл аргументов, одной таблицей не обойтись. Что
/// список не разъедется с `match`, проверяет
/// `every_new_type_is_recognised_by_resolve_new`.
pub const NEW_TYPES: &[&str] = &[
    "Массив",
    "Array",
    "Структура",
    "Structure",
    "Соответствие",
    "Map",
    "ТаблицаЗначений",
    "ValueTable",
    "ОписаниеТипов",
    "TypeDescription",
    "СравнениеЗначений",
    "ValueComparison",
    "ЗаписьТекста",
    "TextWriter",
    "ЧтениеJSON",
    "JSONReader",
    "ЗаписьJSON",
    "JSONWriter",
    "ПараметрыЗаписиJSON",
    "JSONWriterSettings",
    "НастройкиСериализацииJSON",
    "JSONSerializerSettings",
    "ЧтениеXML",
    "XMLReader",
    "ЗаписьXML",
    "XMLWriter",
    // Английское написание ИЗМЕРЕНО: `Новый DOMBuilder` платформа
    // принимает и отдаёт тот же тип, что и `Новый ПостроительDOM`.
    "ПостроительDOM",
    "DOMBuilder",
    // `Новый ДокументDOM` и `Новый ЗаписьDOM` — ИЗМЕРЕНО, что платформа
    // строит оба (справка называет писателя иначе, см. `dom.rs`), и
    // английские написания `DOMDocument`/`DOMWriter` тоже принимает.
    "ДокументDOM",
    "DOMDocument",
    "ЗаписьDOM",
    "DOMWriter",
    // `Новый РазыменовательПространствИменDOM(Узел)` — ИЗМЕРЕНО, что
    // платформа строит разыменователь и конструктором тоже, но РОВНО с
    // одним аргументом: без узла (в отличие от метода документа
    // `СоздатьРазыменовательПИ`) он не создаётся. Английское написание
    // `DOMNamespaceResolver` тоже измерено.
    "РазыменовательПространствИменDOM",
    "DOMNamespaceResolver",
    // Объектная модель XML-схемы. Все написания ИЗМЕРЕНЫ через `Тип(...)`
    // и `Новый`: английские имена у трёх первых есть, а `РасширенноеИмяXML`
    // строится РОВНО двумя аргументами (URI и локальное имя) —
    // одноаргументную форму платформа отвергает.
    "ПостроительСхемXML",
    "XMLSchemaBuilder",
    "СхемаXML",
    "XMLSchema",
    "НаборСхемXML",
    "XMLSchemaSet",
    // `Новый ФабрикаXDTO` строит фабрику по НАБОРУ СХЕМ (или, без
    // аргумента, только по встроенным типам XML Schema) — измерено, что
    // ни путь к файлу, ни схема, ни текст схемы сюда не годятся: файл
    // берёт глобальная `СоздатьФабрикуXDTO`. Английское написание
    // `Новый XDTOFactory` тоже измерено.
    "ФабрикаXDTO",
    "XDTOFactory",
    // `Новый СериализаторXDTO(Фабрика)` — фабрика ОБЯЗАТЕЛЬНА: измерено,
    // что без аргумента платформа отвечает «Конструктор не найден», а
    // два аргумента, строку, число и тип XDTO отвергает. Английское
    // написание `Новый XDTOSerializer` тоже измерено.
    "СериализаторXDTO",
    "XDTOSerializer",
    "РасширенноеИмяXML",
    "XMLExpandedName",
    "ПараметрыЗаписиXML",
    "XMLWriterSettings",
    "ТекстовыйДокумент",
    "TextDocument",
    "ТабличныйДокумент",
    "SpreadsheetDocument",
    // Английское написание ИЗМЕРЕНО: `Новый PDFDocument` платформа
    // принимает и отдаёт тот же тип «Документ PDF».
    "ДокументPDF",
    "PDFDocument",
    // Коллекция вложений строится и сама по себе — ИЗМЕРЕНО, как и
    // английское написание `PDFAttachmentCollection`. У самого
    // `ВложениеPDF` конструктора НЕТ («Конструктор не найден»), поэтому
    // его в этой таблице нет.
    "КоллекцияВложенийPDF",
    "PDFAttachmentCollection",
    // Английское написание проверено пробой `BIN.NEW.EN` на платформе:
    // `Новый BinaryData(Путь)` принимается.
    "ДвоичныеДанные",
    "BinaryData",
    // Английское написание ИЗМЕРЕНО: `Новый BinaryDataBuffer(4)` платформа
    // принимает.
    "БуферДвоичныхДанных",
    "BinaryDataBuffer",
    "УникальныйИдентификатор",
    "UUID",
    // Английские написания обоих потоков ИЗМЕРЕНЫ: `Тип("MemoryStream")` и
    // `Тип("FileStream")` платформа разрешает и считает равными русским.
    "ПотокВПамяти",
    "MemoryStream",
    "ФайловыйПоток",
    "FileStream",
    // Английские написания ИЗМЕРЕНЫ: `Тип("DataReader")` и `Тип("DataWriter")`
    // платформа разрешает и считает равными русским.
    "ЧтениеДанных",
    "DataReader",
    "ЗаписьДанных",
    "DataWriter",
    // Читателей архива на 8.3.27 ДВА, и оба английских написания измерены:
    // `Тип("ZipFileReader")` даёт «Чтение ZIP файла», а
    // `Тип("ArchiveFileReader")` — «Чтение файла архива», то есть это
    // разные типы, а не два имени одного.
    "ЧтениеZipФайла",
    "ZipFileReader",
    "ЧтениеФайлаАрхива",
    "ArchiveFileReader",
    // Писателей тоже два, и оба английских написания измерены:
    // `Тип("ZipFileWriter")` — «Запись ZIP файла», `Тип("ArchiveFileWriter")`
    // — «Запись файла архива».
    "ЗаписьZipФайла",
    "ZipFileWriter",
    "ЗаписьФайлаАрхива",
    "ArchiveFileWriter",
];

struct Resolver<'a> {
    locals: Vec<String>,
    /// Ключ — имя в верхнем регистре: доступ к переменным регистронезависим.
    index: HashMap<String, u32>,
    funcs: &'a HashMap<String, FuncSig>,
    /// Переменные уровня модуля: имя -> номер слота в кадре верхнего
    /// уровня. У резолвера ТЕЛА модуля пуста (там те же переменные уже
    /// локальные), у резолвера каждой функции — заполнена.
    ///
    /// Локальное имя ЗАТЕНЯЕТ модульное — ИЗМЕРЕНО на 8.3.27: функция с
    /// явной `Перем` того же имени работает со своей копией, а модульная
    /// после вызова цела.
    module_index: &'a HashMap<String, u32>,
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    used_libraries: HashSet<bsl_rt::LibraryKey>,
    /// Отвергать ли функции встроенного языка в позиции оператора. В
    /// модулях и фрагментах `Выполнить` — да, как на платформе; в REPL —
    /// нет: голый вызов `СтрДлина("аб")` там печатает значение, и лишать
    /// REPL этого ради правила о компиляции модулей незачем.
    strict_stmt_calls: bool,
    /// Взводится веткой `ExprStmt` на время разрешения верхнего выражения
    /// оператора-вызова и потребляется входом в `resolve_expr`: только в
    /// этой позиции легальна глобальная процедура.
    stmt_call: bool,
}

impl<'a> Resolver<'a> {
    fn declare(&mut self, name: &str) -> u32 {
        let key = name.to_uppercase();
        if let Some(&slot) = self.index.get(&key) {
            return slot;
        }
        let slot = self.locals.len() as u32;
        self.locals.push(name.to_string());
        self.index.insert(key, slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<u32> {
        self.index.get(&name.to_uppercase()).copied()
    }

    fn resolve_block(&mut self, stmts: &[AStmt]) -> Result<Vec<RStmt>, SemaError> {
        let mut out = Vec::new();
        for s in stmts {
            if let Some(rs) = self.resolve_stmt(s)? {
                out.push(rs);
            }
        }
        Ok(out)
    }

    fn resolve_stmt(&mut self, s: &AStmt) -> Result<Option<RStmt>, SemaError> {
        match s {
            AStmt::Assign { target, value } => match target {
                AExpr::Ident(name) => {
                    // Уже известное локальное имя — обычная запись. Иначе
                    // ищем модульное: присваивание модульной переменной
                    // ОБЯЗАНО писать в неё, а не заводить локальную копию,
                    // иначе результат не увидит ни тело модуля, ни соседняя
                    // функция.
                    if let Some(slot) = self.lookup(name) {
                        let value = self.resolve_expr(value)?;
                        return Ok(Some(RStmt::AssignLocal { slot, value }));
                    }
                    if let Some(&slot) = self.module_index.get(&name.to_uppercase()) {
                        let value = self.resolve_expr(value)?;
                        return Ok(Some(RStmt::AssignModuleVar { slot, value }));
                    }
                    let slot = self.declare(name);
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignLocal { slot, value }))
                }
                AExpr::Index { obj, index } => {
                    let obj = self.resolve_expr(obj)?;
                    let index = self.resolve_expr(index)?;
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignIndex { obj, index, value }))
                }
                AExpr::Field { obj, name } => {
                    let obj = self.resolve_expr(obj)?;
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignField {
                        obj,
                        name: name.clone(),
                        open: self.registry.is_some(),
                        value,
                    }))
                }
                _ => Err(SemaError::Unsupported(
                    "присваивание поддержано только в переменную, индекс или поле",
                )),
            },
            AStmt::ExprStmt(e) => {
                // Позиция оператора: единственное место, где легальна
                // глобальная процедура, — и запретное для функций
                // встроенного языка. Проверка второго — по РЕЗУЛЬТАТУ
                // разрешения, а не по имени: пользовательская функция,
                // затеняющая встроенное имя, разрешится в `RExpr::Call` и
                // под правило не попадёт.
                self.stmt_call = true;
                let r = self.resolve_expr(e)?;
                let forbidden = match &r {
                    RExpr::CallBuiltinFn { builtin, .. } => builtin.is_intrinsic(),
                    RExpr::CallComponent { kind, .. } => *kind == bsl_rt::FunctionKind::Intrinsic,
                    RExpr::DynEval(_) => true,
                    _ => false,
                };
                if self.strict_stmt_calls && forbidden {
                    // Обе запретные формы — вызовы по имени-идентификатору;
                    // другие выражения сюда не приводят.
                    let shown = match e {
                        AExpr::Call { callee, .. } => match callee.as_ref() {
                            AExpr::Ident(n) => n.clone(),
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    return Err(SemaError::BuiltinFunctionAsStatement(shown));
                }
                Ok(Some(RStmt::ExprStmt(r)))
            }
            AStmt::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let cond = self.resolve_expr(cond)?;
                let then_branch = self.resolve_block(then_branch)?;
                let mut elsifs = Vec::new();
                for (c, b) in elsif_branches {
                    elsifs.push((self.resolve_expr(c)?, self.resolve_block(b)?));
                }
                let else_branch = match else_branch {
                    Some(b) => Some(self.resolve_block(b)?),
                    None => None,
                };
                Ok(Some(RStmt::If {
                    cond,
                    then_branch,
                    elsif_branches: elsifs,
                    else_branch,
                }))
            }
            AStmt::While { cond, body } => {
                let cond = self.resolve_expr(cond)?;
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::While { cond, body }))
            }
            AStmt::ForNumeric {
                var,
                from,
                to,
                body,
            } => {
                let from = self.resolve_expr(from)?;
                let to = self.resolve_expr(to)?;
                // Переменная цикла объявляется до тела: тело может ссылаться
                // на неё, а сама она остаётся живой и после `КонецЦикла`.
                let slot = self.declare(var);
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::ForNumeric {
                    slot,
                    from,
                    to,
                    body,
                }))
            }
            AStmt::ForEach { var, iter, body } => {
                let iter = self.resolve_expr(iter)?;
                // Как и в ForNumeric: переменная объявляется до тела, живёт
                // и после КонецЦикла.
                let slot = self.declare(var);
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::ForEach { slot, iter, body }))
            }
            AStmt::Break => Ok(Some(RStmt::Break)),
            AStmt::Continue => Ok(Some(RStmt::Continue)),
            AStmt::Return(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Return(r)))
            }
            AStmt::Try { body, except_body } => {
                let body = self.resolve_block(body)?;
                let except_body = self.resolve_block(except_body)?;
                Ok(Some(RStmt::Try { body, except_body }))
            }
            AStmt::Raise(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Raise(r)))
            }
            AStmt::VarDecl(vd) => {
                // Только регистрирует слоты; значение по умолчанию —
                // Неопределено, для этого не нужна ни одна инструкция VM
                // (регистры кадра и так инициализируются Неопределено).
                for name in &vd.names {
                    self.declare(name);
                }
                Ok(None)
            }
            AStmt::Execute(e) => Ok(Some(RStmt::Execute(self.resolve_expr(e)?))),
        }
    }

    fn resolve_expr(&mut self, e: &AExpr) -> Result<RExpr, SemaError> {
        // Потребляем флаг позиции оператора: он относится ровно к ЭТОМУ
        // выражению. Вложенные выражения — аргументы, индексы, объекты
        // цепочек — заходят сюда уже с погашенным флагом, поэтому
        // процедура в аргументе (`Сообщить(Сообщить(1))`) не проскочит.
        let stmt_call = std::mem::take(&mut self.stmt_call);
        match e {
            AExpr::Number(text) => {
                let n = BslNumber::parse_canonical(text).unwrap_or_else(|err| {
                    panic!("лексер пропустил некорректный числовой литерал {text:?}: {err}")
                });
                Ok(RExpr::Number(n))
            }
            AExpr::Bool(b) => Ok(RExpr::Bool(*b)),
            AExpr::Undefined => Ok(RExpr::Undefined),
            AExpr::Null => Ok(RExpr::Null),
            AExpr::Ident(name) => match self.lookup(name) {
                Some(slot) => Ok(RExpr::Local(slot)),
                None => match self.module_index.get(&name.to_uppercase()) {
                    Some(&slot) => Ok(RExpr::ModuleVar(slot)),
                    // `АргументыКоманднойСтроки` пишется и без скобок — как
                    // свойство глобального контекста в OneScript, откуда
                    // это расширение и взято. Переменная с тем же именем
                    // побеждает: проверки выше.
                    None if bsl_rt::BuiltinFn::lookup(name)
                        == Some(bsl_rt::BuiltinFn::CommandLineArguments) =>
                    {
                        Ok(RExpr::CallBuiltinFn {
                            builtin: bsl_rt::BuiltinFn::CommandLineArguments,
                            args: Vec::new(),
                        })
                    }
                    // `ФабрикаXDTO` у платформы — свойство глобального
                    // контекста (фабрика КОНФИГУРАЦИИ), поэтому пишется без
                    // скобок и разрешается тем же приёмом, что и
                    // `АргументыКоманднойСтроки`: голое имя становится
                    // вызовом встроенной функции с нулём аргументов.
                    // Функция всегда отвечает ловимой ошибкой — метаданных
                    // конфигурации здесь нет (см. `bsl_rt::BuiltinFn`).
                    // Переменная с тем же именем побеждает: проверки выше.
                    None if bsl_rt::BuiltinFn::lookup(name)
                        == Some(bsl_rt::BuiltinFn::XdtoConfigurationFactory) =>
                    {
                        Ok(RExpr::CallBuiltinFn {
                            builtin: bsl_rt::BuiltinFn::XdtoConfigurationFactory,
                            args: Vec::new(),
                        })
                    }
                    // Голое имя СИСТЕМНОГО перечисления (без `.Член`) — тоже
                    // валидное выражение, а не обращение к неопределённой
                    // переменной. ИЗМЕРЕНО на `ВариантЗаписиДатыJSON`
                    // (`JSON.DATE_VARIANT_EN_NAMES`, «Т+»:
                    // `Вычислить("JSONDateWritingVariant")` платформа
                    // вычисляет, а не отвергает). Переменная/модульная с тем
                    // же именем побеждает — проверки выше. Что именно
                    // возвращают `Строка()`/`ТипЗнч()` такого значения, не
                    // измерено (см. `bsl_rt::BslValue::EnumType`,
                    // `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`).
                    None if bsl_rt::lookup_enum(name).is_some() => Ok(RExpr::EnumTypeRef(
                        bsl_rt::lookup_enum(name).expect("проверено guard'ом выше"),
                    )),
                    None if self.registry.is_some()
                        && matches!(
                            name.to_uppercase().as_str(),
                            "ФАЙЛОВЫЕПОТОКИ" | "FILESTREAMS"
                        ) =>
                    {
                        let registry = self.registry.expect("проверено guard'ом");
                        let Some((library_index, constructor)) = registry.lookup_constructor(name)
                        else {
                            return Err(SemaError::Unsupported(
                                "ФайловыеПотоки требует зарегистрированный компонент bsl-stream",
                            ));
                        };
                        let package = registry
                            .library(library_index)
                            .expect("индекс получен из таблицы имён этого реестра")
                            .package;
                        let library = bsl_rt::LibraryKey::new(package);
                        self.used_libraries.insert(library.clone());
                        Ok(RExpr::CreateObject {
                            library,
                            constructor,
                            args: Vec::new(),
                        })
                    }
                    // Голое имя менеджера `ФайловыеПотоки` разрешается так
                    // же, как голое имя перечисления, — но НЕ в константу:
                    // измерено, что `ФайловыеПотоки = ФайловыеПотоки` —
                    // «Нет», значит каждое обращение строит новый объект, и
                    // за этим стоит отдельная инструкция.
                    None if matches!(
                        name.to_uppercase().as_str(),
                        "ФАЙЛОВЫЕПОТОКИ" | "FILESTREAMS"
                    ) =>
                    {
                        Ok(RExpr::NewFileStreamsManager)
                    }
                    None => Err(SemaError::UndefinedVariable(name.clone())),
                },
            },
            AExpr::Unary { op, expr } => Ok(RExpr::Unary {
                op: *op,
                expr: Box::new(self.resolve_expr(expr)?),
            }),
            AExpr::Binary { op, lhs, rhs } => Ok(RExpr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_expr(lhs)?),
                rhs: Box::new(self.resolve_expr(rhs)?),
            }),
            AExpr::Call { callee, args } => self.resolve_call(callee, args, stmt_call),
            AExpr::Str(s) => Ok(RExpr::Str(s.clone())),
            // Литерал `'ГГГГММДД'`/`'ГГГГММДДЧЧММСС'`. Лексер уже проверил,
            // что внутри только цифры и их 8 или 14, — но НЕ проверил, что
            // получившаяся дата существует (`'20240230'` пройдёт лексер),
            // поэтому разбор может провалиться и здесь, и это ошибка
            // резолвинга, а не рантайма: литерал известен на этапе
            // компиляции, падать на нём во время исполнения незачем.
            AExpr::Date(digits) => bsl_rt::BslDate::parse_digits(digits)
                .map(RExpr::Date)
                .ok_or(SemaError::BadDateLiteral(digits.clone())),
            AExpr::Index { obj, index } => Ok(RExpr::Index {
                obj: Box::new(self.resolve_expr(obj)?),
                index: Box::new(self.resolve_expr(index)?),
            }),
            AExpr::Field { obj, name } => {
                // `ТипЗначенияJSON.ИмяСвойства` — не чтение поля объекта, а
                // КОНСТАНТА: у платформы перечисление тоже не объект, и
                // несуществующий член она ловит на компиляции, а не в
                // рантайме (проверено `Вычислить`). Проверка идёт до
                // резолвинга левой части — иначе имя перечисления сначала
                // не нашлось бы среди переменных.
                if let AExpr::Ident(base) = obj.as_ref() {
                    if let Some(kind) = bsl_rt::lookup_enum(base) {
                        let member = bsl_rt::lookup_member(kind, name).ok_or_else(|| {
                            SemaError::UndefinedVariable(format!("{base}.{name}"))
                        })?;
                        return Ok(RExpr::EnumMember(member));
                    }
                }
                Ok(RExpr::Field {
                    obj: Box::new(self.resolve_expr(obj)?),
                    name: name.clone(),
                    open: self.registry.is_some(),
                })
            }
            AExpr::New { type_name, args } => self.resolve_new(type_name, args),
            AExpr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => Ok(RExpr::Ternary {
                cond: Box::new(self.resolve_expr(cond)?),
                then_expr: Box::new(self.resolve_expr(then_expr)?),
                else_expr: Box::new(self.resolve_expr(else_expr)?),
            }),
        }
    }

    /// Разбирает известные платформенные типы из [`NEW_TYPES`]. Общие
    /// пользовательские типы пока не поддержаны.
    fn resolve_new(&mut self, type_name: &str, args: &[AExpr]) -> Result<RExpr, SemaError> {
        if let Some(registry) = self.registry {
            if let Some((library_index, constructor)) = registry.lookup_constructor(type_name) {
                let descriptor = registry
                    .constructor(library_index, constructor)
                    .expect("индекс получен из таблицы имён этого реестра");
                let found: u8 =
                    args.len()
                        .try_into()
                        .map_err(|_| SemaError::ArgumentCountMismatch {
                            name: format!("Новый {type_name}"),
                            expected: descriptor.arity.max() as usize,
                            found: args.len(),
                        })?;
                if !descriptor.arity.accepts(found) {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: format!("Новый {type_name}"),
                        expected: descriptor.arity.max() as usize,
                        found: args.len(),
                    });
                }
                let mut resolved_args = Vec::with_capacity(args.len());
                for argument in args {
                    resolved_args.push(self.resolve_expr(argument)?);
                }
                let package = registry
                    .library(library_index)
                    .expect("индекс получен из таблицы имён этого реестра")
                    .package;
                let library = bsl_rt::LibraryKey::new(package);
                self.used_libraries.insert(library.clone());
                return Ok(RExpr::CreateObject {
                    library,
                    constructor,
                    args: resolved_args,
                });
            }
        }
        if self.registry.is_some() {
            let upper = type_name.to_uppercase();
            if matches!(upper.as_str(), "ТЕКСТОВЫЙДОКУМЕНТ" | "TEXTDOCUMENT") {
                return Err(SemaError::Unsupported(
                    "ТекстовыйДокумент требует зарегистрированный компонент bsl-textdoc",
                ));
            }
            if matches!(
                upper.as_str(),
                "ПОТОКВПАМЯТИ"
                    | "MEMORYSTREAM"
                    | "ФАЙЛОВЫЙПОТОК"
                    | "FILESTREAM"
                    | "ЧТЕНИЕДАННЫХ"
                    | "DATAREADER"
                    | "ЗАПИСЬДАННЫХ"
                    | "DATAWRITER"
            ) {
                return Err(SemaError::Unsupported(
                    "потоковый тип требует зарегистрированный компонент bsl-stream",
                ));
            }
            if matches!(
                upper.as_str(),
                "ЧТЕНИЕXML"
                    | "XMLREADER"
                    | "ЗАПИСЬXML"
                    | "XMLWRITER"
                    | "ПАРАМЕТРЫЗАПИСИXML"
                    | "XMLWRITERSETTINGS"
            ) {
                return Err(SemaError::Unsupported(
                    "тип XML требует зарегистрированный компонент bsl-xml",
                ));
            }
            if matches!(
                upper.as_str(),
                "ПОСТРОИТЕЛЬDOM"
                    | "DOMBUILDER"
                    | "ДОКУМЕНТDOM"
                    | "DOMDOCUMENT"
                    | "ЗАПИСЬDOM"
                    | "DOMWRITER"
                    | "РАЗЫМЕНОВАТЕЛЬПРОСТРАНСТВИМЕНDOM"
                    | "DOMNAMESPACERESOLVER"
            ) {
                return Err(SemaError::Unsupported(
                    "тип DOM требует зарегистрированный компонент bsl-xml",
                ));
            }
            if matches!(
                upper.as_str(),
                "ПОСТРОИТЕЛЬСХЕМXML"
                    | "XMLSCHEMABUILDER"
                    | "СХЕМАXML"
                    | "XMLSCHEMA"
                    | "НАБОРСХЕМXML"
                    | "XMLSCHEMASET"
                    | "РАСШИРЕННОЕИМЯXML"
            ) {
                return Err(SemaError::Unsupported(
                    "тип модели схемы требует зарегистрированный компонент bsl-xml",
                ));
            }
            if matches!(
                upper.as_str(),
                "ФАБРИКАXDTO" | "XDTOFACTORY" | "СЕРИАЛИЗАТОРXDTO" | "XDTOSERIALIZER"
            ) {
                return Err(SemaError::Unsupported(
                    "тип XDTO требует зарегистрированный компонент bsl-xml",
                ));
            }
            if matches!(
                upper.as_str(),
                "ДОКУМЕНТPDF" | "PDFDOCUMENT" | "КОЛЛЕКЦИЯВЛОЖЕНИЙPDF" | "PDFATTACHMENTCOLLECTION"
            ) {
                return Err(SemaError::Unsupported(
                    "тип PDF требует зарегистрированный компонент bsl-pdf",
                ));
            }
            if matches!(
                upper.as_str(),
                "ЧТЕНИЕZIPФАЙЛА"
                    | "ZIPFILEREADER"
                    | "ЧТЕНИЕФАЙЛААРХИВА"
                    | "ARCHIVEFILEREADER"
                    | "ЗАПИСЬZIPФАЙЛА"
                    | "ZIPFILEWRITER"
                    | "ЗАПИСЬФАЙЛААРХИВА"
                    | "ARCHIVEFILEWRITER"
            ) {
                return Err(SemaError::Unsupported(
                    "тип архива требует зарегистрированный компонент bsl-zip",
                ));
            }
            if matches!(
                upper.as_str(),
                "ЧТЕНИЕJSON"
                    | "JSONREADER"
                    | "ЗАПИСЬJSON"
                    | "JSONWRITER"
                    | "ПАРАМЕТРЫЗАПИСИJSON"
                    | "JSONWRITERSETTINGS"
                    | "НАСТРОЙКИСЕРИАЛИЗАЦИИJSON"
                    | "JSONSERIALIZERSETTINGS"
            ) {
                return Err(SemaError::Unsupported(
                    "JSON-тип требует зарегистрированный компонент bsl-json",
                ));
            }
        }
        match type_name.to_uppercase().as_str() {
            "МАССИВ" | "ARRAY" => {
                let mut dims = Vec::with_capacity(args.len());
                for a in args {
                    dims.push(self.resolve_expr(a)?);
                }
                Ok(RExpr::NewArray { dims })
            }
            "СТРУКТУРА" | "STRUCTURE" => {
                if args.is_empty() {
                    return Ok(RExpr::NewStructure {
                        keys: Vec::new(),
                        values: Vec::new(),
                    });
                }
                // ВНИМАНИЕ: список ключей обязан быть строковым ЛИТЕРАЛОМ —
                // именно это делает число форм структур конечным и
                // известным на этапе компиляции (`ShapeTable::intern`
                // заводит их с `depth = 0`, то есть с полным запасом
                // переходов). Когда сюда добавят вычисляемую строку
                // (`Новый Структура(КлючиИзПеременной)`), интернировать её
                // ключи в таблицу форм НЕЛЬЗЯ: цикл с разными ключами
                // заводил бы бессмертную форму на каждой итерации, а
                // деградация по `MAX_SHAPE_TRANSITIONS` тут не спасает —
                // каждая такая форма создаётся с нулевой глубиной. Такой
                // конструктор должен сразу строить структуру в словарном
                // режиме (`bsl_rt::StructureStorage::Dictionary`), минуя
                // интернирование.
                let key_text = match &args[0] {
                    AExpr::Str(s) => s,
                    _ => {
                        return Err(SemaError::Unsupported(
                            "Новый Структура(...) со списком полей не строковым литералом появится позже",
                        ))
                    }
                };
                let keys: Vec<String> = key_text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let rest = &args[1..];
                let values = if rest.is_empty() {
                    keys.iter().map(|_| RExpr::Undefined).collect()
                } else {
                    if rest.len() != keys.len() {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: "Новый Структура".to_string(),
                            expected: keys.len(),
                            found: rest.len(),
                        });
                    }
                    let mut vs = Vec::with_capacity(rest.len());
                    for a in rest {
                        vs.push(self.resolve_expr(a)?);
                    }
                    vs
                };
                Ok(RExpr::NewStructure { keys, values })
            }
            "ТАБЛИЦАЗНАЧЕНИЙ" | "VALUETABLE" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ТаблицаЗначений".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTable)
            }
            "ОПИСАНИЕТИПОВ" | "TYPEDESCRIPTION" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ОписаниеТипов".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTypeDescription(Box::new(
                    self.resolve_expr(&args[0])?,
                )))
            }
            "СРАВНЕНИЕЗНАЧЕНИЙ" | "VALUECOMPARISON" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый СравнениеЗначений".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewValueComparison)
            }
            "СООТВЕТСТВИЕ" | "MAP" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый Соответствие".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewMap)
            }
            "ЗАПИСЬТЕКСТА" | "TEXTWRITER" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЗаписьТекста".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTextWriter {
                    path: Box::new(self.resolve_expr(&args[0])?),
                })
            }
            // Ровно один аргумент — имя файла. Ни пустой конструктор, ни
            // два аргумента платформа не принимает (пробы `BIN.NEW.NOARG`,
            // `BIN.NEW.TWOARGS`), поэтому арность проверяется здесь, а не
            // в рантайме.
            "ДВОИЧНЫЕДАННЫЕ" | "BINARYDATA" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ДвоичныеДанные".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewBinaryData {
                    path: Box::new(self.resolve_expr(&args[0])?),
                })
            }
            // Размер обязателен, порядок байтов необязателен, третьего
            // аргумента нет: всё измерено — платформа отвергает и пустой
            // конструктор, и вызов с тремя аргументами.
            "БУФЕРДВОИЧНЫХДАННЫХ" | "BINARYDATABUFFER" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый БуферДвоичныхДанных".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let size = self.resolve_expr(&args[0])?;
                let order = match args.get(1) {
                    Some(a) => self.resolve_expr(a)?,
                    None => RExpr::Undefined,
                };
                Ok(RExpr::NewBinaryBuffer {
                    size: Box::new(size),
                    order: Box::new(order),
                })
            }
            // Аргумент необязателен: без него — случайный идентификатор,
            // со строкой — разбор канонической формы.
            "УНИКАЛЬНЫЙИДЕНТИФИКАТОР" | "UUID" => {
                if args.len() > 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый УникальныйИдентификатор".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let arg = match args.first() {
                    Some(a) => self.resolve_expr(a)?,
                    None => RExpr::Undefined,
                };
                Ok(RExpr::NewUuid { arg: Box::new(arg) })
            }
            // Единственный аргумент необязателен: пустой конструктор
            // платформа принимает, а второй аргумент отвергает (измерено).
            "ПОТОКВПАМЯТИ" | "MEMORYSTREAM" => {
                if args.len() > 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ПотокВПамяти".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let arg = match args.first() {
                    Some(a) => self.resolve_expr(a)?,
                    None => RExpr::Undefined,
                };
                Ok(RExpr::NewMemoryStream { arg: Box::new(arg) })
            }
            // Имя и режим обязательны, доступ необязателен и по умолчанию
            // `ЧтениеИЗапись` — измерено сравнением «без доступа» с явной
            // `ЧтениеИЗапись` во всей таблице режимов.
            "ФАЙЛОВЫЙПОТОК" | "FILESTREAM" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ФайловыйПоток".to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let path = self.resolve_expr(&args[0])?;
                let mode = self.resolve_expr(&args[1])?;
                let access = match args.get(2) {
                    Some(a) => self.resolve_expr(a)?,
                    None => RExpr::Undefined,
                };
                Ok(RExpr::NewFileStream {
                    path: Box::new(path),
                    mode: Box::new(mode),
                    access: Box::new(access),
                })
            }
            // Источник обязателен (пустой конструктор платформа не находит
            // вовсе), хвостовые три — нет. Пятый аргумент у платформы есть,
            // но его тип не измерен, поэтому больше четырёх здесь ошибка.
            "ЧТЕНИЕДАННЫХ" | "DATAREADER" | "ЗАПИСЬДАННЫХ" | "DATAWRITER" =>
            {
                let reader = matches!(
                    type_name.to_uppercase().as_str(),
                    "ЧТЕНИЕДАННЫХ" | "DATAREADER"
                );
                let display = if reader {
                    "Новый ЧтениеДанных"
                } else {
                    "Новый ЗаписьДанных"
                };
                if args.is_empty() || args.len() > 4 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: display.to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let source = Box::new(self.resolve_expr(&args[0])?);
                let mut tail = Vec::new();
                for i in 1..4 {
                    tail.push(Box::new(match args.get(i) {
                        Some(a) => self.resolve_expr(a)?,
                        None => RExpr::Undefined,
                    }));
                }
                let mut tail = tail.into_iter();
                let encoding = tail.next().unwrap_or_else(|| Box::new(RExpr::Undefined));
                let order = tail.next().unwrap_or_else(|| Box::new(RExpr::Undefined));
                let separator = tail.next().unwrap_or_else(|| Box::new(RExpr::Undefined));
                Ok(if reader {
                    RExpr::NewDataReader {
                        source,
                        encoding,
                        order,
                        separator,
                    }
                } else {
                    RExpr::NewDataWriter {
                        source,
                        encoding,
                        order,
                        separator,
                    }
                })
            }
            // Оба читателя архива: все аргументы необязательны, но у
            // zip-варианта их не больше двух — ИЗМЕРЕНО, что
            // `Новый ЧтениеZipФайла(файл, пароль, тип)` платформа встречает
            // «Конструктор не найден», тогда как
            // `Новый ЧтениеФайлаАрхива(файл, пароль, ТипФайлаАрхива.Zip)`
            // принимает.
            "ЧТЕНИЕZIPФАЙЛА" | "ZIPFILEREADER" | "ЧТЕНИЕФАЙЛААРХИВА" | "ARCHIVEFILEREADER" =>
            {
                let zip = matches!(
                    type_name.to_uppercase().as_str(),
                    "ЧТЕНИЕZIPФАЙЛА" | "ZIPFILEREADER"
                );
                let limit = if zip { 2 } else { 3 };
                if args.len() > limit {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: if zip {
                            "Новый ЧтениеZipФайла".to_string()
                        } else {
                            "Новый ЧтениеФайлаАрхива".to_string()
                        },
                        expected: limit,
                        found: args.len(),
                    });
                }
                let mut arg = |i: usize| -> Result<Box<RExpr>, SemaError> {
                    Ok(Box::new(match args.get(i) {
                        Some(a) => self.resolve_expr(a)?,
                        None => RExpr::Undefined,
                    }))
                };
                let source = arg(0)?;
                let password = arg(1)?;
                let archive_type = arg(2)?;
                Ok(RExpr::NewArchiveReader {
                    zip,
                    source,
                    password,
                    archive_type,
                })
            }
            // Мест у писателей РАЗНОЕ число, и это измерено с настоящим
            // путём первым аргументом: у zip-варианта их семь (восьмой —
            // «Конструктор не найден»), у архивного восемь (девятый —
            // «Конструктор не найден»), ровно на вставленный третьим
            // `ТипФайлаАрхива` больше. Прежний общий предел 7 держался на
            // пробе с `Неопределено` первым: там платформа отвечает
            // «Конструктор не найден» уже потому, что подходящей перегрузки
            // с таким первым аргументом нет вовсе. Значат места разное, и
            // разбирает их рантайм.
            "ЗАПИСЬZIPФАЙЛА" | "ZIPFILEWRITER" | "ЗАПИСЬФАЙЛААРХИВА" | "ARCHIVEFILEWRITER" =>
            {
                let zip = matches!(
                    type_name.to_uppercase().as_str(),
                    "ЗАПИСЬZIPФАЙЛА" | "ZIPFILEWRITER"
                );
                let limit = if zip { 7 } else { 8 };
                if args.len() > limit {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: if zip {
                            "Новый ЗаписьZipФайла".to_string()
                        } else {
                            "Новый ЗаписьФайлаАрхива".to_string()
                        },
                        expected: limit,
                        found: args.len(),
                    });
                }
                let mut resolved = Vec::with_capacity(args.len());
                for a in args {
                    resolved.push(self.resolve_expr(a)?);
                }
                Ok(RExpr::NewArchiveWriter {
                    zip,
                    args: resolved,
                })
            }
            "ЧТЕНИЕJSON" | "JSONREADER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЧтениеJSON".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewJsonReader)
            }
            "ЗАПИСЬJSON" | "JSONWRITER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЗаписьJSON".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewJsonWriter)
            }
            // Оба аргумента необязательны, недостающие — `Неопределено`,
            // как и хвостовые аргументы встроенных функций.
            "ПАРАМЕТРЫЗАПИСИJSON" | "JSONWRITERSETTINGS" => {
                if args.len() > 2 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ПараметрыЗаписиJSON".to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                let mut parts = Vec::with_capacity(2);
                for a in args {
                    parts.push(self.resolve_expr(a)?);
                }
                while parts.len() < 2 {
                    parts.push(RExpr::Undefined);
                }
                let indent = parts.pop().expect("две позиции только что заполнены");
                let line_break = parts.pop().expect("две позиции только что заполнены");
                Ok(RExpr::NewJsonWriterSettings {
                    line_break: Box::new(line_break),
                    indent: Box::new(indent),
                })
            }
            // Без аргументов — все свойства читаются и пишутся отдельно
            // через точку (см. `bsl_rt::BslValue::new_json_serializer_settings`).
            "НАСТРОЙКИСЕРИАЛИЗАЦИИJSON" | "JSONSERIALIZERSETTINGS" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый НастройкиСериализацииJSON".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewJsonSerializerSettings)
            }
            "ТАБЛИЧНЫЙДОКУМЕНТ" | "SPREADSHEETDOCUMENT" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ТабличныйДокумент".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewSpreadDocument)
            }
            // Аргументов у конструктора НЕТ: измерено, что и путь, и
            // `ДвоичныеДанные` платформа отвергает — источник назначается
            // отдельным `Прочитать`.
            "ДОКУМЕНТPDF" | "PDFDOCUMENT" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ДокументPDF".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewPdfDocument)
            }
            // Коллекцию вложений платформа строит и отдельно от документа
            // (измерено), а вот `Новый ВложениеPDF` не знает вовсе —
            // «Конструктор не найден».
            //
            // Аргументы конструктор МОЛЧА ИГНОРИРУЕТ: измерен ровно один
            // случай — `Новый КоллекцияВложенийPDF(1)` платформа принимает
            // и отдаёт пустую коллекцию («новый коллекция с аргументом:
            // КоллекцияВложенийPDF»). Любое другое число аргументов здесь
            // проходит по аналогии с этим замером, а не потому, что его
            // мерили. Проверки арности нет — как нет и вычисления
            // аргументов: смысла у них никакого, а побочный эффект в
            // конструкторе коллекции вложений выдумывать незачем.
            "КОЛЛЕКЦИЯВЛОЖЕНИЙPDF" | "PDFATTACHMENTCOLLECTION" => {
                Ok(RExpr::NewPdfAttachments)
            }
            "ТЕКСТОВЫЙДОКУМЕНТ" | "TEXTDOCUMENT" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ТекстовыйДокумент".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTextDocument)
            }
            "ЧТЕНИЕXML" | "XMLREADER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЧтениеXML".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXmlReader)
            }
            // Аргументов у построителя нет: `Новый ПостроительDOM("x")`
            // платформа отвергает (измерено).
            "ПОСТРОИТЕЛЬDOM" | "DOMBUILDER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ПостроительDOM".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewDomBuilder)
            }
            // Ни у документа, ни у писателя аргументов нет: измерено, что
            // `Новый ДокументDOM("а")` и `Новый ЗаписьDOM("а")` платформа
            // отвергает.
            "ДОКУМЕНТDOM" | "DOMDOCUMENT" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ДокументDOM".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewDomDocument)
            }
            "РАЗЫМЕНОВАТЕЛЬПРОСТРАНСТВИМЕНDOM" | "DOMNAMESPACERESOLVER" =>
            {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый РазыменовательПространствИменDOM".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewDomNsResolver(Box::new(
                    self.resolve_expr(&args[0])?,
                )))
            }
            "ЗАПИСЬDOM" | "DOMWRITER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЗаписьDOM".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewDomWriter)
            }
            // Аргументов нет ни у построителя схем, ни у пустой схемы,
            // ни у набора: `Новый ПостроительСхемXML("x")`, `Новый
            // СхемаXML("urn:test")` и `Новый НаборСхемXML("x")` платформа
            // отвергает (измерено все три).
            "ПОСТРОИТЕЛЬСХЕМXML" | "XMLSCHEMABUILDER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ПостроительСхемXML".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXsBuilder)
            }
            "СХЕМАXML" | "XMLSCHEMA" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый СхемаXML".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXmlSchema)
            }
            "НАБОРСХЕМXML" | "XMLSCHEMASET" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый НаборСхемXML".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXmlSchemaSet)
            }
            // Набор схем необязателен: без него получается фабрика с
            // одними встроенными типами XML Schema (измерено), и
            // пропущенная позиция уходит вниз как `Неопределено` — так же,
            // как хвостовые аргументы `Новый ПараметрыЗаписиJSON`. Двух
            // аргументов платформа не берёт (измерено).
            "ФАБРИКАXDTO" | "XDTOFACTORY" => {
                if args.len() > 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ФабрикаXDTO".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                let schemas = match args.first() {
                    Some(a) => self.resolve_expr(a)?,
                    None => RExpr::Undefined,
                };
                Ok(RExpr::NewXdtoFactory {
                    schemas: Box::new(schemas),
                })
            }
            // Фабрика здесь, в отличие от `Новый ФабрикаXDTO`,
            // ОБЯЗАТЕЛЬНА: измерено, что вызов без аргумента платформа
            // отвергает на компиляции («Конструктор не найден»), а двух
            // аргументов не берёт.
            "СЕРИАЛИЗАТОРXDTO" | "XDTOSERIALIZER" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый СериализаторXDTO".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXdtoSerializer(Box::new(
                    self.resolve_expr(&args[0])?,
                )))
            }
            // Ровно два аргумента: URI и локальное имя (измерено, что
            // одноаргументная форма и форма без аргументов отвергаются).
            "РАСШИРЕННОЕИМЯXML" | "XMLEXPANDEDNAME" => {
                if args.len() != 2 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый РасширенноеИмяXML".to_string(),
                        expected: 2,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXmlExpandedName {
                    uri: Box::new(self.resolve_expr(&args[0])?),
                    local: Box::new(self.resolve_expr(&args[1])?),
                })
            }
            "ЗАПИСЬXML" | "XMLWRITER" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЗаписьXML".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewXmlWriter)
            }
            // Три необязательных параметра — кодировка, версия и признак
            // отступа; недостающие уходят `Неопределено`, как и у
            // `ПараметрыЗаписиJSON`.
            "ПАРАМЕТРЫЗАПИСИXML" | "XMLWRITERSETTINGS" => {
                if args.len() > 3 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ПараметрыЗаписиXML".to_string(),
                        expected: 3,
                        found: args.len(),
                    });
                }
                let mut parts = Vec::with_capacity(3);
                for a in args {
                    parts.push(self.resolve_expr(a)?);
                }
                while parts.len() < 3 {
                    parts.push(RExpr::Undefined);
                }
                let indent = parts.pop().expect("три позиции только что заполнены");
                let version = parts.pop().expect("три позиции только что заполнены");
                let encoding = parts.pop().expect("три позиции только что заполнены");
                Ok(RExpr::NewXmlWriterSettings {
                    encoding: Box::new(encoding),
                    version: Box::new(version),
                    indent: Box::new(indent),
                })
            }
            _ => Err(SemaError::Unsupported(
                "этот тип в выражении Новый пока не поддержан",
            )),
        }
    }

    fn resolve_call(
        &mut self,
        callee: &AExpr,
        args: &[Option<AExpr>],
        stmt_call: bool,
    ) -> Result<RExpr, SemaError> {
        match callee {
            AExpr::Ident(name) => {
                if let Some((index, has_default)) = self
                    .funcs
                    .get(&name.to_uppercase())
                    .map(|s| (s.index, s.has_default.clone()))
                {
                    let arity = has_default.len();
                    if args.len() != arity {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: arity,
                            found: args.len(),
                        });
                    }
                    // В отличие от `resolve_required_args` (используется
                    // ниже для builtin'ов, у которых нет объявленных
                    // умолчаний) — пропуск позиции здесь допустим, если у
                    // ЭТОГО параметра есть значение по умолчанию: тогда
                    // компилируется маркер `RExpr::Skipped`, а не ошибка.
                    let mut rargs = Vec::with_capacity(args.len());
                    for (i, a) in args.iter().enumerate() {
                        match a {
                            Some(e) => rargs.push(self.resolve_expr(e)?),
                            None if has_default[i] => rargs.push(RExpr::Skipped),
                            None => {
                                return Err(SemaError::MissingRequiredArgument {
                                    name: name.clone(),
                                    position: i,
                                })
                            }
                        }
                    }
                    return Ok(RExpr::Call {
                        func: index,
                        args: rargs,
                    });
                }
                if let Some(registry) = self.registry {
                    if let Some((library_index, function)) = registry.lookup_function(name) {
                        let descriptor = registry
                            .function(library_index, function)
                            .expect("индекс получен из таблицы имён этого реестра");
                        if descriptor.kind == bsl_rt::FunctionKind::Procedure && !stmt_call {
                            return Err(SemaError::ProcedureAsFunction(name.clone()));
                        }
                        let found: u8 = args.len().try_into().map_err(|_| {
                            SemaError::ArgumentCountMismatch {
                                name: name.clone(),
                                expected: descriptor.arity.max() as usize,
                                found: args.len(),
                            }
                        })?;
                        if !descriptor.arity.accepts(found) {
                            return Err(SemaError::ArgumentCountMismatch {
                                name: name.clone(),
                                expected: descriptor.arity.max() as usize,
                                found: args.len(),
                            });
                        }
                        // Глобальные функции компонентов имеют ту же
                        // BSL-семантику пропущенных позиций, что и builtin:
                        // `ПрочитатьJSON(Ч, , Имена)` передаёт `Неопределено`.
                        let rargs = self.resolve_builtin_args(args)?;
                        let package = registry
                            .library(library_index)
                            .expect("индекс получен из таблицы имён этого реестра")
                            .package;
                        let library = bsl_rt::LibraryKey::new(package);
                        self.used_libraries.insert(library.clone());
                        return Ok(RExpr::CallComponent {
                            library,
                            function,
                            kind: descriptor.kind,
                            args: rargs,
                        });
                    }
                }
                // `Окр(x[, ЧислоРазрядов[, Режим]])` — единственный
                // builtin с необязательными аргументами, до генерального
                // механизма умолчаний builtin'ов (которого нет — см.
                // `bsl_rt::BuiltinFn::arity`, всегда фиксированная арность).
                // Подставляем недостающие `0` литералами здесь же, а не
                // заводим вариативную арность ради одной функции:
                // `BuiltinFn::Round` в рантайме всегда видит ровно 3
                // аргумента. `0` для режима означает "умолчание" (см.
                // `BslValue::round`).
                if name.eq_ignore_ascii_case("Окр") || name.eq_ignore_ascii_case("Round") {
                    const ROUND_ARITY: usize = 3;
                    if args.is_empty() || args.len() > ROUND_ARITY {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: ROUND_ARITY,
                            found: args.len(),
                        });
                    }
                    let mut rargs = self.resolve_builtin_args(args)?;
                    while rargs.len() < ROUND_ARITY {
                        // ИМЕННО `Неопределено`, а не `0`: измерено, что
                        // умолчание платформы совпадает с режимом 1, а не с
                        // режимом 0, — подстановка нуля меняла бы семантику.
                        rargs.push(RExpr::Undefined);
                    }
                    return Ok(RExpr::CallBuiltinFn {
                        builtin: bsl_rt::BuiltinFn::Round,
                        args: rargs,
                    });
                }
                if let Some(builtin) = bsl_rt::BuiltinFn::lookup(name) {
                    // При компиляции с реестром вынесенные функции не
                    // должны проваливаться в legacy-`CallBuiltin`: иначе модуль
                    // не попадёт в `.requires`. Путь без реестра нужен только
                    // переходному `bsl-cli`.
                    if self.registry.is_some()
                        && matches!(
                            builtin,
                            bsl_rt::BuiltinFn::StrFindByRegex
                                | bsl_rt::BuiltinFn::StrFindAllByRegex
                                | bsl_rt::BuiltinFn::StrReplaceByRegex
                                | bsl_rt::BuiltinFn::StrLikeByRegex
                                | bsl_rt::BuiltinFn::BitwiseAnd
                                | bsl_rt::BuiltinFn::BitwiseOr
                                | bsl_rt::BuiltinFn::BitwiseNot
                                | bsl_rt::BuiltinFn::BitwiseAndNot
                                | bsl_rt::BuiltinFn::BitwiseXor
                                | bsl_rt::BuiltinFn::BitwiseShiftLeft
                                | bsl_rt::BuiltinFn::BitwiseShiftRight
                                | bsl_rt::BuiltinFn::CheckBit
                                | bsl_rt::BuiltinFn::CheckByBitMask
                                | bsl_rt::BuiltinFn::SetBit
                                | bsl_rt::BuiltinFn::NumberFromHexString
                                | bsl_rt::BuiltinFn::NumberFromBinaryString
                                | bsl_rt::BuiltinFn::ReadJson
                                | bsl_rt::BuiltinFn::WriteJson
                                | bsl_rt::BuiltinFn::WriteJsonDate
                                | bsl_rt::BuiltinFn::ReadJsonDate
                                | bsl_rt::BuiltinFn::WriteJsonValue
                                | bsl_rt::BuiltinFn::ReadJsonValue
                        )
                    {
                        return Err(SemaError::UndefinedFunction(name.clone()));
                    }
                    // Глобальная процедура легальна только оператором:
                    // `Х = Сообщить(1)` платформа отвергает («Обращение к
                    // процедуре как к функции») — ИЗМЕРЕНО, якорь
                    // `CALL.EXPR.PROCEDURE`.
                    if builtin.is_procedure() && !stmt_call {
                        return Err(SemaError::ProcedureAsFunction(name.clone()));
                    }
                    let (min, max) = builtin.arity_range();
                    if args.len() < min || args.len() > max {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: max,
                            found: args.len(),
                        });
                    }
                    // Недостающие необязательные позиции добиваются
                    // `Неопределено` ЗДЕСЬ, а не вариативной арностью в
                    // рантайме: `call_builtin_fn` тогда всегда индексирует
                    // фиксированный набор аргументов, а «аргумент опущен»
                    // становится обычным значением, которое сама функция и
                    // трактует (`Сред` — до конца строки, `КодСимвола` —
                    // позиция 1, `СтрШаблон` — пустая подстановка).
                    let mut rargs = self.resolve_builtin_args(args)?;
                    while rargs.len() < max {
                        rargs.push(RExpr::Undefined);
                    }
                    return Ok(RExpr::CallBuiltinFn {
                        builtin,
                        args: rargs,
                    });
                }
                if name.eq_ignore_ascii_case("Вычислить") || name.eq_ignore_ascii_case("Eval")
                {
                    if args.len() != 1 {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            found: args.len(),
                        });
                    }
                    let mut rargs = self.resolve_required_args(args)?;
                    return Ok(RExpr::DynEval(Box::new(rargs.remove(0))));
                }
                Err(SemaError::UndefinedFunction(name.clone()))
            }
            AExpr::Field { obj, name } => {
                let method = bsl_rt::BuiltinMethod::lookup(name);
                // `Добавить` полиморфен по типу получателя (0 аргументов —
                // новая строка таблицы, 1 — элемент массива/колонка), а тип
                // получателя в динамическом BSL здесь ещё не известен:
                // финальную проверку арности для него делает рантайм (см.
                // `bsl_rt::call_builtin_method`). Для остальных методов
                // арность фиксирована и проверяется сразу.
                let expected: Option<usize> = method.and_then(|method| match method {
                    // DOM. Признаки — строго без аргументов, поиск
                    // элемента по идентификатору — строго с одним, а у
                    // четырёх «атрибутных» методов форм две (имя либо
                    // URI и локальное имя), поэтому их арность решает
                    // рантайм. Всё измерено: платформа отвергает и
                    // `ЕстьАтрибут()`, и `ЕстьАтрибут("а", "б", "в")`.
                    bsl_rt::BuiltinMethod::DomHasChildNodes
                    | bsl_rt::BuiltinMethod::DomHasAttributes => Some(0),
                    bsl_rt::BuiltinMethod::DomGetElementById => Some(1),
                    // Фабрики узлов и мутация. Строго один аргумент — у
                    // текстоподобных фабрик и у операций с одним узлом,
                    // строго два — у инструкции обработки, вставки перед,
                    // замены; у форм с пространством имён (`СоздатьЭлемент`,
                    // `СоздатьАтрибут`, `УстановитьАтрибут`,
                    // `УдалитьАтрибут`) арность решает рантайм.
                    bsl_rt::BuiltinMethod::DomCreateTextNode
                    | bsl_rt::BuiltinMethod::DomCreateCdataSection
                    | bsl_rt::BuiltinMethod::DomCreateComment
                    | bsl_rt::BuiltinMethod::DomAppendChild
                    | bsl_rt::BuiltinMethod::DomRemoveChild
                    | bsl_rt::BuiltinMethod::DomSetAttributeNode
                    | bsl_rt::BuiltinMethod::DomRemoveAttributeNode => Some(1),
                    bsl_rt::BuiltinMethod::DomCreateProcessingInstruction
                    | bsl_rt::BuiltinMethod::DomInsertBefore
                    | bsl_rt::BuiltinMethod::DomReplaceChild => Some(2),
                    bsl_rt::BuiltinMethod::DomCreateElement
                    | bsl_rt::BuiltinMethod::DomCreateAttribute
                    | bsl_rt::BuiltinMethod::DomSetAttribute
                    | bsl_rt::BuiltinMethod::DomRemoveAttribute => None,
                    bsl_rt::BuiltinMethod::DomGetAttribute
                    | bsl_rt::BuiltinMethod::DomHasAttribute
                    | bsl_rt::BuiltinMethod::DomGetAttributeNode
                    | bsl_rt::BuiltinMethod::DomGetElementsByName => None,
                    // XPath. Строго фиксированы три имени: создание
                    // выражения — ровно два аргумента, поиск URI — один,
                    // обход — ноль (измерено, что платформа отвергает и
                    // `СоздатьВыражениеXPath(в)`, и
                    // `НайтиURIПространстваИмен()`). У вычисления форм две
                    // (три аргумента и четыре), у создания разыменователя
                    // — три (ноль, один и два), у `Вычислить` — две (узел
                    // и узел с видом), у `ЭлементСнимка` имя совпало бы с
                    // чужим, будь оно у кого-то ещё; всё это решает
                    // рантайм.
                    bsl_rt::BuiltinMethod::XPathCreateExpression => Some(2),
                    bsl_rt::BuiltinMethod::XPathLookupNamespaceUri
                    | bsl_rt::BuiltinMethod::XPathSnapshotItem => Some(1),
                    bsl_rt::BuiltinMethod::XPathNext => Some(0),
                    // `ПолучитьГруппы()` аргументов не берёт: измерено, что
                    // синтаксис статьи 16.5.4 — ровно пустые скобки.
                    bsl_rt::BuiltinMethod::RegexGetGroups => Some(0),
                    bsl_rt::BuiltinMethod::XPathEvaluate
                    | bsl_rt::BuiltinMethod::XPathCreateNsResolver
                    | bsl_rt::BuiltinMethod::XPathEvaluateExpression => None,
                    bsl_rt::BuiltinMethod::Count
                    | bsl_rt::BuiltinMethod::Clear
                    | bsl_rt::BuiltinMethod::Close => Some(0),
                    // `Размер` есть только у двоичных данных, и лишний
                    // аргумент платформа отвергает (проба
                    // `BIN.SIZE.EXTRAARG`) — арность фиксированная.
                    bsl_rt::BuiltinMethod::Size => Some(0),
                    bsl_rt::BuiltinMethod::Delete => Some(1),
                    // `Получить` полиморфен, как и `Добавить`: у
                    // `Соответствие` и буфера это один аргумент, а у
                    // именованной коллекции компонент схемы — ещё и пара
                    // (URI, имя) (измерено). Арность решает рантайм.
                    bsl_rt::BuiltinMethod::Get => None,
                    // `СоздатьСхемуXML` — строго один аргумент: ни без
                    // аргументов, ни с двумя платформа его не берёт
                    // (измерено).
                    bsl_rt::BuiltinMethod::CreateXmlSchema => Some(1),
                    // Методы буфера. `Установить` и побитовые — строго два
                    // аргумента, `Разделить`/`Соединить` — один (измерено:
                    // ни без аргументов, ни с двумя платформа их не берёт).
                    bsl_rt::BuiltinMethod::BufSet
                    | bsl_rt::BuiltinMethod::WriteBitwiseAnd
                    | bsl_rt::BuiltinMethod::WriteBitwiseOr
                    | bsl_rt::BuiltinMethod::WriteBitwiseXor
                    | bsl_rt::BuiltinMethod::WriteBitwiseAndNot => Some(2),
                    bsl_rt::BuiltinMethod::BufSplit | bsl_rt::BuiltinMethod::BufConcat => Some(1),
                    // `ПолучитьСрез` — 1 или 2 (количество необязательно),
                    // арность решает рантайм.
                    bsl_rt::BuiltinMethod::BufSlice => None,
                    // У чтения целого 1..2 аргумента, у записи 2..3, у
                    // `Инвертировать` 0..2 — арность решает рантайм.
                    bsl_rt::BuiltinMethod::ReadInt16
                    | bsl_rt::BuiltinMethod::ReadInt32
                    | bsl_rt::BuiltinMethod::ReadInt64
                    | bsl_rt::BuiltinMethod::WriteInt16
                    | bsl_rt::BuiltinMethod::WriteInt32
                    | bsl_rt::BuiltinMethod::WriteInt64
                    | bsl_rt::BuiltinMethod::Invert => None,
                    bsl_rt::BuiltinMethod::Insert => Some(2),
                    bsl_rt::BuiltinMethod::FindRows | bsl_rt::BuiltinMethod::Total => Some(1),
                    // `Записать` — 1 у `ЗаписьТекста` (кусок текста) и 1..2 у
                    // `ТекстовыйДокумент` (путь и кодировка). Как и у
                    // `Прочитать`, арность решает рантайм.
                    bsl_rt::BuiltinMethod::Write => None,
                    bsl_rt::BuiltinMethod::UnloadColumn | bsl_rt::BuiltinMethod::IndexOf => Some(1),
                    bsl_rt::BuiltinMethod::LoadColumn | bsl_rt::BuiltinMethod::Move => Some(2),
                    // `Свойство` — 1 или 2 (см. `BslValue::structure_property`),
                    // `Найти` — 1 или 2 (список колонок необязателен), как и
                    // у `Добавить` арность решает рантайм. Из волны 3 так же
                    // устроены `Скопировать` (0..2), `СкопироватьКолонки`
                    // (0..1) и `Свернуть` (1..2).
                    bsl_rt::BuiltinMethod::Add
                    | bsl_rt::BuiltinMethod::Property
                    | bsl_rt::BuiltinMethod::Find
                    | bsl_rt::BuiltinMethod::Sort
                    | bsl_rt::BuiltinMethod::FillValues
                    | bsl_rt::BuiltinMethod::Copy
                    | bsl_rt::BuiltinMethod::CopyColumns
                    | bsl_rt::BuiltinMethod::Collapse => None,
                    // JSON. Без аргументов — обход читателя и открытие/
                    // закрытие контейнеров записи.
                    bsl_rt::BuiltinMethod::WriteStartObject
                    | bsl_rt::BuiltinMethod::WriteEndObject
                    | bsl_rt::BuiltinMethod::WriteStartArray
                    | bsl_rt::BuiltinMethod::WriteEndArray => Some(0),
                    // `Пропустить` — 0 у читателей JSON/XML (шаг через узел) и
                    // 1 у `ЧтениеДанных` (сколько байтов перешагнуть). Тип
                    // получателя здесь ещё не известен, поэтому арность решает
                    // рантайм.
                    bsl_rt::BuiltinMethod::SkipNode => None,
                    // Методы `ЧтениеДанных`/`ЗаписьДанных`. Необязательные
                    // хвостовые аргументы (количество, кодировка, разделитель)
                    // проверяет рантайм; фиксированы только те, у кого форма
                    // ровно одна.
                    bsl_rt::BuiltinMethod::DataReadByte
                    | bsl_rt::BuiltinMethod::GetBinaryData
                    | bsl_rt::BuiltinMethod::GetBinaryDataBuffer => Some(0),
                    bsl_rt::BuiltinMethod::DataWriteByte => Some(1),
                    bsl_rt::BuiltinMethod::DataReadIntoBuffer
                    | bsl_rt::BuiltinMethod::DataReadChars
                    | bsl_rt::BuiltinMethod::DataReadLine
                    | bsl_rt::BuiltinMethod::DataWriteChars
                    | bsl_rt::BuiltinMethod::DataWriteLine => None,
                    bsl_rt::BuiltinMethod::WritePropertyName
                    | bsl_rt::BuiltinMethod::WriteJsonValue => Some(1),
                    // `УстановитьСтроку` — 1 у читателя (текст) и 0..1 у
                    // писателя (параметры), `ОткрытьФайл` — 1..2. Тип
                    // получателя здесь ещё не известен, поэтому арность,
                    // как у `Добавить`, решает рантайм.
                    // `Прочитать` — 0 у читателей JSON/XML (шаг по потоку) и 1 у
                    // `ТекстовыйДокумент` (путь к файлу). Тип получателя здесь
                    // ещё не известен, поэтому арность решает рантайм.
                    bsl_rt::BuiltinMethod::SetString
                    | bsl_rt::BuiltinMethod::OpenFile
                    | bsl_rt::BuiltinMethod::ReadNext => None,
                    // XML. Обход читателя и закрытие элемента — без
                    // аргументов, остальное — по числу измеренных
                    // параметров.
                    bsl_rt::BuiltinMethod::GetText | bsl_rt::BuiltinMethod::LineCount => Some(0),
                    bsl_rt::BuiltinMethod::SetText
                    | bsl_rt::BuiltinMethod::GetLine
                    | bsl_rt::BuiltinMethod::AddLine
                    | bsl_rt::BuiltinMethod::DeleteLine
                    | bsl_rt::BuiltinMethod::OutputArea => Some(1),
                    // `ПолучитьОбласть` у платформы проверяет число
                    // аргументов в РАНТАЙМЕ: `ПолучитьОбласть(2, 3)` — не
                    // ошибка компиляции, а ловимое исключение.
                    bsl_rt::BuiltinMethod::GetArea => None,
                    // `Область` — 1 аргумент (адрес строкой) либо 4
                    // (координаты), поэтому арность решает рантайм.
                    bsl_rt::BuiltinMethod::Region => None,
                    bsl_rt::BuiltinMethod::MergeCells
                    | bsl_rt::BuiltinMethod::UnmergeCells
                    | bsl_rt::BuiltinMethod::EndRowGroup => Some(0),
                    // `НачатьГруппуСтрок` — от нуля до двух аргументов.
                    bsl_rt::BuiltinMethod::BeginRowGroup => None,
                    bsl_rt::BuiltinMethod::InsertLine | bsl_rt::BuiltinMethod::ReplaceLine => {
                        Some(2)
                    }
                    bsl_rt::BuiltinMethod::XmlReadAttribute
                    | bsl_rt::BuiltinMethod::XmlAttributeCount
                    | bsl_rt::BuiltinMethod::XmlMoveToContent
                    | bsl_rt::BuiltinMethod::WriteXmlDeclaration
                    | bsl_rt::BuiltinMethod::WriteEndElement => Some(0),
                    bsl_rt::BuiltinMethod::XmlAttributeName
                    | bsl_rt::BuiltinMethod::XmlAttributeValue
                    | bsl_rt::BuiltinMethod::WriteStartElement
                    | bsl_rt::BuiltinMethod::WriteXmlText
                    | bsl_rt::BuiltinMethod::WriteXmlComment
                    | bsl_rt::BuiltinMethod::WriteCdataSection
                    | bsl_rt::BuiltinMethod::WriteXmlRaw => Some(1),
                    bsl_rt::BuiltinMethod::WriteXmlAttribute
                    | bsl_rt::BuiltinMethod::WriteXmlProcessingInstruction => Some(2),
                    // Потоки. `ТекущаяПозиция` — без аргументов, `Перейти`
                    // — строго со смещением и точкой отсчёта: `Перейти(0)`
                    // платформа отвергает, и это ошибка КОМПИЛЯЦИИ, а не
                    // ловимое исключение (измерено).
                    bsl_rt::BuiltinMethod::CurrentPosition => Some(0),
                    bsl_rt::BuiltinMethod::Seek => Some(2),
                    // У `Открыть` доступ необязателен (2..3), поэтому
                    // арность решает рантайм; остальные три метода
                    // менеджера берут ровно имя файла.
                    bsl_rt::BuiltinMethod::StreamOpen => None,
                    bsl_rt::BuiltinMethod::StreamOpenForRead
                    | bsl_rt::BuiltinMethod::StreamOpenForWrite
                    | bsl_rt::BuiltinMethod::StreamOpenForAppend => Some(1),
                    // `Создать` и `Тип` полиморфны по получателю, как
                    // `Получить` и `Добавить`: у менеджера потоков
                    // `Создать` — один аргумент, у фабрики XDTO — от одного
                    // до трёх; `Тип` у фабрики — пара (URI, имя) либо
                    // расширенное имя, а у экземпляра `ОбъектXDTO` —
                    // вообще без аргументов. Всё измерено, и решает рантайм.
                    bsl_rt::BuiltinMethod::Create | bsl_rt::BuiltinMethod::XdtoType => None,
                    // Экземпляр XDTO. Арности измерены поимённо: имя
                    // свойства — один аргумент, `Установить` — два (оно
                    // делит вариант с `БуферДвоичныхДанных`, см. выше), а
                    // `Проверить`, `Свойства`, `Владелец` и
                    // `Последовательность` берут ровно ноль: лишний
                    // аргумент — ошибка на всех четырёх (пробы «объект …
                    // с аргументом» в `measure-xdto.bsl`; какая именно —
                    // компиляции или исполнения — не различима, они сняты
                    // через `Выполнить` внутри `Попытка`, здесь это
                    // ошибка компиляции).
                    bsl_rt::BuiltinMethod::XdtoGetList
                    | bsl_rt::BuiltinMethod::XdtoIsSet
                    | bsl_rt::BuiltinMethod::XdtoUnset
                    | bsl_rt::BuiltinMethod::XdtoSequenceValue
                    | bsl_rt::BuiltinMethod::XdtoSequenceProperty => Some(1),
                    bsl_rt::BuiltinMethod::XdtoValidate
                    | bsl_rt::BuiltinMethod::XdtoObjectProperties
                    | bsl_rt::BuiltinMethod::XdtoOwner
                    | bsl_rt::BuiltinMethod::XdtoSequenceOf => Some(0),
                    // Ввод-вывод фабрики: у `ПрочитатьXML` форм две
                    // (читатель и читатель с типом), у `ЗаписатьXML` три
                    // (писатель со значением, плюс имя, плюс URI) —
                    // измерено, что третий аргумент чтения и пятый записи
                    // платформа отвергает. Арность решает рантайм.
                    bsl_rt::BuiltinMethod::XdtoReadXml | bsl_rt::BuiltinMethod::XdtoWriteXml => {
                        None
                    }
                    // Три члена сериализатора, которых здесь нет: они
                    // не поддержаны ни при какой арности, и проверять её
                    // значило бы отвечать на `Сер.XMLТип()` рассказом про
                    // число аргументов вместо главного — что метода нет.
                    // Отказ даёт рантайм, и он перехватывается `Попытка`,
                    // как на платформе.
                    bsl_rt::BuiltinMethod::XdtoXmlTypeOfType
                    | bsl_rt::BuiltinMethod::XdtoXmlTypeOfValue
                    | bsl_rt::BuiltinMethod::XdtoCanReadXml => None,
                    // Распаковка. У `Извлечь` форм три (элемент с
                    // каталогом, с режимом и с паролем), у `ИзвлечьВсе` —
                    // две; всё измерено, и обе арности решает рантайм.
                    bsl_rt::BuiltinMethod::ArchiveExtract
                    | bsl_rt::BuiltinMethod::ArchiveExtractAll => None,
                });
                if let Some(expected) = expected {
                    if args.len() != expected {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected,
                            found: args.len(),
                        });
                    }
                }
                let rargs = self.resolve_required_args(args)?;
                let obj = self.resolve_expr(obj)?;
                Ok(RExpr::CallMethod {
                    obj: Box::new(obj),
                    method: name.clone(),
                    open: self.registry.is_some(),
                    args: rargs,
                })
            }
            _ => Err(SemaError::Unsupported(
                "вызов не по простому имени/методу появится позже",
            )),
        }
    }

    /// Аргументы ВСТРОЕННОЙ функции: пропущенная позиция (`Ф(1, , 3)`) —
    /// то же `Неопределено`, которым добиваются недостающие хвостовые
    /// позиции (см. вызывающие ветки). Без этого
    /// `ЗаполнитьЗначенияСвойств(П, И, , "Б")` — обычная для этой функции
    /// форма записи, когда нужен только последний параметр, — не
    /// компилировалась бы вовсе.
    ///
    /// У ПОЛЬЗОВАТЕЛЬСКИХ функций правило другое (`RExpr::Skipped` и
    /// пролог умолчаний, см. `resolve_call`): там пропуск значит «взять
    /// объявленное значение по умолчанию», а объявить его встроенной
    /// функции негде. Пропуск ОБЯЗАТЕЛЬНОЙ позиции здесь не ловится: как и
    /// в 1С, это ошибка времени исполнения у той функции, которой достался
    /// `Неопределено`, а не ошибка компиляции.
    fn resolve_builtin_args(&mut self, args: &[Option<AExpr>]) -> Result<Vec<RExpr>, SemaError> {
        let mut rargs = Vec::with_capacity(args.len());
        for a in args {
            rargs.push(match a {
                Some(e) => self.resolve_expr(e)?,
                None => RExpr::Undefined,
            });
        }
        Ok(rargs)
    }

    /// Аргументы там, где пропуск позиции подставить нечем: методы
    /// объектов и `Вычислить`. У метода нет ни объявленных умолчаний (он
    /// не пользовательский), ни фиксированной арности, по которой можно
    /// было бы отличить пропущенный необязательный аргумент от
    /// пропущенного обязательного, — тип получателя в BSL известен только
    /// в рантайме. Поэтому здесь `Ф(1, , 3)` остаётся ошибкой резолвинга,
    /// в отличие от [`resolve_builtin_args`](Self::resolve_builtin_args).
    fn resolve_required_args(&mut self, args: &[Option<AExpr>]) -> Result<Vec<RExpr>, SemaError> {
        let mut rargs = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Some(e) => rargs.push(self.resolve_expr(e)?),
                None => {
                    return Err(SemaError::Unsupported(
                        "пропущенные аргументы Ф(1, , 3) появятся вместе со значениями по умолчанию",
                    ));
                }
            }
        }
        Ok(rargs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_syntax::parse;

    fn resolve_src(src: &str) -> Resolved {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        let stmts = items_to_stmts(prog.items);
        resolve_script(&stmts).unwrap_or_else(|e| panic!("sema error: {e:?}"))
    }

    /// В тестовых скриптах верхнего уровня допускаем только `Перем` и
    /// обычные операторы — объявления процедур/функций сюда не проверяем
    /// (для них нужны кадры, это M4).
    fn items_to_stmts(items: Vec<bsl_syntax::Item>) -> Vec<AStmt> {
        items
            .into_iter()
            .map(|item| match item {
                bsl_syntax::Item::Stmt(s) => s,
                bsl_syntax::Item::VarDecl(vd) => AStmt::VarDecl(vd),
                other => panic!("expected only statements/Перем in test script, got {other:?}"),
            })
            .collect()
    }

    fn resolve_program_src(src: &str) -> ResolvedProgram {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        resolve_program(&prog.items).unwrap_or_else(|e| panic!("sema error: {e:?}"))
    }

    /// `NEW_TYPES` — список для автодополнения, а разбор `Новый` живёт в
    /// `match` по имени типа. Разъехаться им нельзя: имя из списка,
    /// которого `resolve_new` не знает, REPL предлагал бы вхолостую.
    #[test]
    fn every_new_type_is_recognised_by_resolve_new() {
        for type_name in NEW_TYPES {
            let prog = parse(&format!("x = Новый {type_name}();")).unwrap();
            let stmts = items_to_stmts(prog.items);
            // Арность у типов разная (`ЗаписьТекста` требует путь), поэтому
            // ошибка числа аргументов здесь допустима — недопустимо
            // «такой тип не поддержан».
            match resolve_script(&stmts) {
                Ok(_) | Err(SemaError::ArgumentCountMismatch { .. }) => {}
                Err(other) => panic!("Новый {type_name}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_type_outside_the_list_is_not_constructible() {
        let prog = parse("x = Новый СписокЗначений();").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts),
            Err(SemaError::Unsupported(_))
        ));
    }

    #[test]
    fn implicit_declaration_on_first_assignment() {
        let r = resolve_src("PI = 3.14;");
        assert_eq!(r.locals, vec!["PI".to_string()]);
        assert_eq!(
            r.body,
            vec![RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Number(BslNumber::parse_canonical("3.14").unwrap()),
            }]
        );
    }

    #[test]
    fn reading_undefined_variable_is_an_error() {
        let prog = parse("y = x;").unwrap();
        let stmts = items_to_stmts(prog.items);
        let err = resolve_script(&stmts).unwrap_err();
        assert_eq!(err, SemaError::UndefinedVariable("x".to_string()));
    }

    #[test]
    fn case_insensitive_identifier_is_the_same_variable() {
        let r = resolve_src("x = 1;\nX = x + 1;");
        assert_eq!(r.locals, vec!["x".to_string()]);
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Binary {
                    op: bsl_syntax::BinaryOp::Add,
                    lhs: Box::new(RExpr::Local(0)),
                    rhs: Box::new(RExpr::Number(BslNumber::from_i64(1))),
                },
            }
        );
    }

    #[test]
    fn for_loop_variable_is_declared_and_alive_in_body() {
        let r = resolve_src("Для i = 0 По 10 Цикл\ny = i;\nКонецЦикла");
        assert_eq!(r.locals, vec!["i".to_string(), "y".to_string()]);
    }

    #[test]
    fn var_decl_registers_slot_without_runtime_effect() {
        let r = resolve_src("Перем a, b;\na = 1;");
        assert_eq!(r.locals, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(r.body.len(), 1);
    }

    #[test]
    fn calling_undeclared_function_is_an_error() {
        let prog = parse("Ф();").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::UndefinedFunction("Ф".to_string())
        );
    }

    #[test]
    fn builtin_function_call_resolves_without_user_declaration() {
        let r = resolve_src("x = sqrt(4);");
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::CallBuiltinFn {
                    builtin: bsl_rt::BuiltinFn::Sqrt,
                    args: vec![RExpr::Number(BslNumber::from_i64(4))],
                },
            }
        );
    }

    /// Мест у писателей архива разное число, и обе границы ИЗМЕРЕНЫ с
    /// настоящим путём первым аргументом: zip-вариант принимает семь и
    /// отвергает восьмой, архивный принимает восемь и отвергает девятый.
    /// Разница ровно в одно место — на вставленный третьим
    /// `ТипФайлаАрхива`.
    #[test]
    fn the_two_archive_writers_have_different_argument_limits() {
        let empty = |n: usize| -> String { ", Неопределено".repeat(n) };
        let resolve = |src: &str| {
            let prog = parse(src).unwrap();
            resolve_script(&items_to_stmts(prog.items))
        };

        assert!(resolve(&format!(
            "x = Новый ЗаписьZipФайла(\"/tmp/а.zip\"{});",
            empty(6)
        ))
        .is_ok());
        assert_eq!(
            resolve(&format!(
                "x = Новый ЗаписьZipФайла(\"/tmp/а.zip\"{});",
                empty(7)
            ))
            .unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Новый ЗаписьZipФайла".to_string(),
                expected: 7,
                found: 8,
            }
        );

        assert!(resolve(&format!(
            "x = Новый ЗаписьФайлаАрхива(\"/tmp/а.zip\"{});",
            empty(7)
        ))
        .is_ok());
        assert_eq!(
            resolve(&format!(
                "x = Новый ЗаписьФайлаАрхива(\"/tmp/а.zip\"{});",
                empty(8)
            ))
            .unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Новый ЗаписьФайлаАрхива".to_string(),
                expected: 8,
                found: 9,
            }
        );
    }

    #[test]
    fn builtin_function_arity_mismatch_is_an_error() {
        let prog = parse("x = Pow(2);").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert_eq!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Pow".to_string(),
                expected: 2,
                found: 1,
            }
        );
    }

    #[test]
    fn count_method_call_resolves_on_array() {
        let r = resolve_src("a = Новый Массив(3);\nn = a.Count();");
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::CallMethod {
                    obj: Box::new(RExpr::Local(0)),
                    method: "Count".to_string(),
                    open: false,
                    args: vec![],
                },
            }
        );
    }

    #[test]
    fn an_unknown_method_is_left_for_the_runtime_receiver() {
        let prog = parse("a = Новый Массив(3);\nn = a.НетТакогоМетода();").unwrap();
        let stmts = items_to_stmts(prog.items);
        let resolved = resolve_script(&stmts).unwrap();
        assert!(matches!(
            &resolved.body[1],
            RStmt::AssignLocal {
                value: RExpr::CallMethod { method, .. },
                ..
            } if method == "НетТакогоМетода"
        ));
    }

    #[test]
    fn functions_can_be_called_before_their_declaration() {
        // Main вызывает Helper, объявленную ниже по тексту — должно работать,
        // сигнатуры собираются за отдельный проход до резолвинга тел.
        let rp = resolve_program_src(
            "Функция Main()\nВозврат Helper();\nКонецФункции\n\nФункция Helper()\nВозврат 1;\nКонецФункции",
        );
        assert_eq!(rp.functions.len(), 2);
        assert_eq!(rp.functions[0].name, "Main");
        assert_eq!(
            rp.functions[0].body,
            vec![RStmt::Return(Some(RExpr::Call {
                func: 1,
                args: vec![],
            }))]
        );
    }

    #[test]
    fn params_occupy_the_first_slots_by_val_flag_recorded() {
        let rp = resolve_program_src("Процедура П(Знач а, б)\nКонецПроцедуры");
        let f = &rp.functions[0];
        assert_eq!(f.locals[0], "а");
        assert_eq!(f.locals[1], "б");
        assert!(f.params[0].by_val);
        assert!(!f.params[1].by_val);
    }

    #[test]
    fn argument_count_mismatch_is_an_error() {
        let prog = parse("Функция Ф(а)\nВозврат а;\nКонецФункции\nx = Ф(1, 2);").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Ф".to_string(),
                expected: 1,
                found: 2,
            }
        );
    }

    #[test]
    fn skipping_a_parameter_without_a_default_is_an_error() {
        let prog = parse("Функция Ф(а, б)\nВозврат а;\nКонецФункции\nx = Ф(1, );").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                position: 1,
            }
        );
    }

    #[test]
    fn skipping_a_defaulted_parameter_resolves_to_skipped_marker() {
        let prog = parse("Функция Ф(а, б = 100)\nВозврат а;\nКонецФункции\nx = Ф(1, );").unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        match &resolved.top_level.body[0] {
            RStmt::AssignLocal {
                value: RExpr::Call { args, .. },
                ..
            } => {
                assert_eq!(args[1], RExpr::Skipped);
            }
            other => panic!("expected AssignLocal(Call), got {other:?}"),
        }
    }

    /// У ВСТРОЕННОЙ функции объявленных умолчаний нет, поэтому пропуск —
    /// не `RExpr::Skipped`, а `Неопределено`: ровно то же, чем добиваются
    /// недостающие хвостовые позиции. Ради этой формы записи
    /// (`ЗаполнитьЗначенияСвойств(П, И, , "Б")`) правило и заведено.
    #[test]
    fn skipping_a_builtin_argument_resolves_to_undefined() {
        let resolved =
            resolve_src("П = Новый Структура(\"А\", 1);\nЗаполнитьЗначенияСвойств(П, П, , \"Б\");");
        let RStmt::ExprStmt(RExpr::CallBuiltinFn { builtin, args }) = &resolved.body[1] else {
            panic!(
                "ожидался вызов встроенной функции, получено {:?}",
                resolved.body[1]
            );
        };
        assert_eq!(*builtin, bsl_rt::BuiltinFn::FillPropertyValues);
        assert_eq!(args.len(), 4);
        assert_eq!(args[2], RExpr::Undefined, "пропущенная позиция");
        assert!(
            matches!(args[3], RExpr::Str(_)),
            "последняя позиция на месте"
        );
    }

    /// Правило платформы о позиции вызова трёхчастное — снято полным
    /// перебором таблицы имён, деление хранит `BuiltinFn::is_intrinsic`.
    /// Функция встроенного языка оператором — отказ компиляции.
    #[test]
    fn an_intrinsic_call_as_a_statement_is_rejected() {
        for src in [
            "Строка(1);",
            "СтрЗаменить(\"а\", \"а\", \"б\");",
            "ТекущаяДата();",
            "Вычислить(\"1\");",
            "StrLen(\"аб\");",
        ] {
            let prog = parse(src).unwrap();
            let err = resolve_script(&items_to_stmts(prog.items)).unwrap_err();
            assert!(
                matches!(err, SemaError::BuiltinFunctionAsStatement(_)),
                "{src}: {err:?}"
            );
        }
    }

    /// Функция глобального контекста оператором законна — платформа зовёт
    /// её и отбрасывает результат, а нуль-арная просто выполняется.
    /// Процедуры оператором — тем более.
    #[test]
    fn a_context_function_call_as_a_statement_is_allowed() {
        resolve_src("СтрНайти(\"а\", \"б\");");
        resolve_src("ТекущаяУниверсальнаяДатаВМиллисекундах();");
        resolve_src("Сообщить(1);");
        resolve_src("ЗаписатьJSON(1, 2);");
    }

    /// Глобальная процедура в позиции выражения — «Обращение к процедуре
    /// как к функции», в том числе аргументом другого вызова.
    #[test]
    fn a_procedure_in_an_expression_is_rejected() {
        for src in [
            "Х = Сообщить(1);",
            "Сообщить(Сообщить(1));",
            "Х = ЗаполнитьЗначенияСвойств(1, 2);",
        ] {
            let prog = parse(src).unwrap();
            let err = resolve_script(&items_to_stmts(prog.items)).unwrap_err();
            assert!(
                matches!(err, SemaError::ProcedureAsFunction(_)),
                "{src}: {err:?}"
            );
        }
    }

    /// REPL мягче модуля: голый вызов функции языка там печатает значение,
    /// а строгий фрагмент `Выполнить` идёт через `resolve_snippet_stmts` и
    /// отвергает его, как платформа.
    #[test]
    fn the_repl_keeps_bare_intrinsic_calls() {
        let prog = parse("СтрДлина(\"аб\");").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(resolve_repl_stmts(&[], &[], &stmts, &[]).is_ok());
        assert!(matches!(
            resolve_snippet_stmts(&[], &[], &stmts, &[]),
            Err(SemaError::BuiltinFunctionAsStatement(_))
        ));
    }

    #[test]
    fn duplicate_function_name_is_an_error() {
        let prog = parse("Функция Ф()\nКонецФункции\nПроцедура ф()\nКонецПроцедуры").unwrap();
        assert!(matches!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::DuplicateFunction(_)
        ));
    }

    #[test]
    fn top_level_can_call_functions_declared_anywhere_in_module() {
        let rp = resolve_program_src("Процедура П(x)\nКонецПроцедуры\nП(1);");
        assert_eq!(
            rp.top_level.body,
            vec![RStmt::ExprStmt(RExpr::Call {
                func: 0,
                args: vec![RExpr::Number(BslNumber::from_i64(1))],
            })]
        );
    }

    #[test]
    fn new_array_resolves_dimensions() {
        let r = resolve_src("a = Новый Массив(3, 4);");
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewArray {
                    dims: vec![
                        RExpr::Number(BslNumber::from_i64(3)),
                        RExpr::Number(BslNumber::from_i64(4)),
                    ],
                },
            }
        );
    }

    #[test]
    fn new_structure_with_literal_keys_and_values() {
        let r = resolve_src(r#"s = Новый Структура("x,y", 1, 2);"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewStructure {
                    keys: vec!["x".to_string(), "y".to_string()],
                    values: vec![
                        RExpr::Number(BslNumber::from_i64(1)),
                        RExpr::Number(BslNumber::from_i64(2)),
                    ],
                },
            }
        );
    }

    #[test]
    fn new_structure_keys_only_defaults_values_to_undefined() {
        let r = resolve_src(r#"s = Новый Структура("x,y");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewStructure {
                    keys: vec!["x".to_string(), "y".to_string()],
                    values: vec![RExpr::Undefined, RExpr::Undefined],
                },
            }
        );
    }

    #[test]
    fn new_structure_with_dynamic_key_list_is_unsupported() {
        // Проверка формы аргумента (строковый литерал?) идёт раньше
        // резолвинга самого идентификатора, поэтому это Unsupported, а не
        // UndefinedVariable("k"), даже когда k нигде не объявлена.
        let prog = parse("s = Новый Структура(k);").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::Unsupported(_)
        ));
    }

    #[test]
    fn index_and_field_assignment_targets() {
        let r =
            resolve_src("a = Новый Массив(1);\ns = Новый Структура(\"x\");\na[0] = 1;\ns.x = 2;");
        assert!(matches!(r.body[2], RStmt::AssignIndex { .. }));
        assert!(matches!(r.body[3], RStmt::AssignField { .. }));
    }

    #[test]
    fn for_each_declares_loop_variable() {
        let r = resolve_src("a = Новый Массив();\nДля Каждого x Из a Цикл\ny = x;\nКонецЦикла");
        assert_eq!(r.locals[0], "a".to_string());
        assert!(matches!(r.body[1], RStmt::ForEach { .. }));
    }

    #[test]
    fn try_except_resolves_both_bodies() {
        let r = resolve_src("Попытка\nx = 1;\nИсключение\ny = 2;\nКонецПопытки");
        assert!(matches!(r.body[0], RStmt::Try { .. }));
    }

    #[test]
    fn raise_with_and_without_expression() {
        let r = resolve_src(
            "Попытка\nВызватьИсключение \"ошибка\";\nИсключение\nВызватьИсключение;\nКонецПопытки",
        );
        match &r.body[0] {
            RStmt::Try { body, except_body } => {
                assert_eq!(
                    body[0],
                    RStmt::Raise(Some(RExpr::Str("ошибка".to_string())))
                );
                assert_eq!(except_body[0], RStmt::Raise(None));
            }
            other => panic!("expected Try, got {other:?}"),
        }
    }

    #[test]
    fn execute_resolves_to_rstmt_execute() {
        let r = resolve_src(r#"Выполнить("x = 1");"#);
        assert_eq!(r.body[0], RStmt::Execute(RExpr::Str("x = 1".to_string())));
    }

    #[test]
    fn vychislit_resolves_to_dyn_eval() {
        let r = resolve_src(r#"y = Вычислить("2+2");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::DynEval(Box::new(RExpr::Str("2+2".to_string()))),
            }
        );
    }

    #[test]
    fn resolve_snippet_stmts_seeds_existing_locals_and_extends() {
        let existing = vec!["x".to_string()];
        let prog = parse("x = x + 1;\ny = 2;").unwrap();
        let stmts = items_to_stmts(prog.items);
        let (locals, body) = resolve_snippet_stmts(&existing, &[], &stmts, &[]).unwrap();
        assert_eq!(locals, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(
            body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Binary {
                    op: bsl_syntax::BinaryOp::Add,
                    lhs: Box::new(RExpr::Local(0)),
                    rhs: Box::new(RExpr::Number(BslNumber::from_i64(1))),
                },
            }
        );
        assert_eq!(
            body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::Number(BslNumber::from_i64(2))
            }
        );
    }

    #[test]
    fn command_line_arguments_reads_bare_and_a_local_shadows_it() {
        // Голое имя без объявления — чтение встроенной функции, а не
        // UndefinedVariable; регистр и английский синоним не важны.
        for name in [
            "АргументыКоманднойСтроки",
            "аргументыкоманднойстроки",
            "CommandLineArguments",
        ] {
            let r = resolve_src(&format!("А = {name};"));
            assert_eq!(
                r.body[0],
                RStmt::AssignLocal {
                    slot: 0,
                    value: RExpr::CallBuiltinFn {
                        builtin: bsl_rt::BuiltinFn::CommandLineArguments,
                        args: Vec::new(),
                    },
                },
                "{name}"
            );
        }

        // Присваивание объявляет обычную локальную переменную, и дальше
        // имя читается из неё, а не из встроенной функции.
        let r = resolve_src("АргументыКоманднойСтроки = 1;\nБ = АргументыКоманднойСтроки;");
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::Local(0),
            }
        );
    }
}
