//! Проверка образа байт-кода перед исполнением.
//!
//! [`Program`] и [`Chunk`] — публичные типы с публичными полями, и приходят
//! они не только от собственного кодогена: их собирает разбор текстового
//! формата и может собрать Rust-клиент. Значит, у образа есть СТАТИЧЕСКИЕ
//! свойства, нарушение которых даёт не отказ, а неверный ответ, — и
//! проверять их обязан тот, кто владеет представлением, а не тот, кто его
//! исполняет: интерпретатору образ приходит уже проверенным.
//!
//! Проверка идёт ОДИН РАЗ, до первой инструкции (её зовёт связывание в
//! `bsl-vm`), поэтому горячий цикл за неё не платит. Рядом, в
//! [`crate::bundle::verify`], лежит проверка разметки бандлов — та же роль
//! на другом срезе представления.

use bsl_rt::RtError;

use crate::instr::{ArgMode, Instr};
use crate::{Chunk, Program};

/// Цели передачи управления и геометрия вызовов лежат внутри своих чанков.
///
/// `Program` — публичный тип с публичными полями, и приходит она не только
/// от собственного кодогена: её собирает разбор текстового байт-кода и
/// может собрать Rust-клиент. Проверять это на каждой инструкции нельзя —
/// это горячий цикл, — поэтому проверка идёт один раз при связывании, до
/// первой инструкции. Без неё цель за концом чанка давала `pc`, который
/// VM принимает за нормальное завершение: программа заканчивалась без
/// вывода и с кодом успеха, то есть выдавала неверный ответ вместо ошибки.
///
/// Всё, что проверяется здесь, — СТАТИЧЕСКИЕ свойства программы, и каждое
/// из них при нарушении даёт не отказ, а неверный ответ. Отсюда и
/// перечень: цели переходов, диапазоны `Попытка`, а также три свойства
/// инструкции `Call` и пролога умолчаний, каждое из которых умеет молча
/// испортить чужой регистр или подменить значение параметра.
pub fn verify(program: &Program) -> Result<(), RtError> {
    check_program_tables(program)?;
    for chunk in &program.chunks {
        // Геометрия кадра упорядочена: параметры занимают первые слоты
        // локалей, локали — первые регистры (см. `bsl-sema`/`bsl-compiler`).
        // Нарушение не просто «странно»: границей для `ByRefLocal` служит
        // `n_locals` (см. `check_call_geometry`), поэтому при `n_locals >
        // n_regs` слот из промежутка проходит проверку, а `Frame::reg_index`
        // превращает его в абсолютный индекс ВЫШЕ вершины кадра вызывающего —
        // параметр по ссылке алиасит собственный регистр ВЫЗВАННОЙ функции.
        // Воспроизведено: запись по такому параметру не доходит до
        // переменной вызывающего, программа заканчивается с кодом успеха.
        if chunk.n_params > chunk.n_locals || chunk.n_locals > chunk.n_regs {
            return Err(RtError::InvalidBytecode(
                "геометрия кадра: параметры, локали и регистры не упорядочены",
            ));
        }
        // Режимы и умолчания параметров — по одному на параметр (кодоген
        // строит оба массива ровно длины `n_params`). При рассинхроне
        // фрагмент `Выполнить`/`Вычислить` резолвит вызов этой функции по
        // неверной арности (`param_has_default.len()`, см. `bsl-sema`) или по
        // неверному режиму: недостающий `param_by_val` кодоген фрагмента молча
        // считает `Знач` (см. `bsl-compiler`), и переданный ПО ССЫЛКЕ
        // параметр становится переданным ЗНАЧЕНИЕМ — образ выдаёт неверный
        // ответ вместо отказа. Проверка статическая, поэтому стоит здесь, а
        // не на каждом `RunDynamic`.
        if chunk.param_by_val.len() != chunk.n_params as usize
            || chunk.param_has_default.len() != chunk.n_params as usize
        {
            return Err(RtError::InvalidBytecode(
                "число режимов или умолчаний параметров не совпадает с числом параметров",
            ));
        }
        let limit = chunk.instrs.len();
        let mut runs_dynamic = false;
        for (pc, instr) in chunk.instrs.iter().enumerate() {
            runs_dynamic |= matches!(instr, Instr::RunDynamic { .. });
            if let Some(target) = instr.jump_target()
                && (target < 0 || target as usize > limit)
            {
                return Err(RtError::InvalidBytecode("цель перехода за пределами чанка"));
            }
            if let Instr::JumpIfNotEqConst { src, k, .. } | Instr::JumpIfNotLtConst { src, k, .. } =
                instr
            {
                if *src >= chunk.n_regs {
                    return Err(RtError::InvalidBytecode(
                        "регистр условного перехода выходит за кадр",
                    ));
                }
                if *k as usize >= chunk.consts.len() {
                    return Err(RtError::InvalidBytecode(
                        "номер константы условного перехода вне таблицы чанка",
                    ));
                }
            }
            if let Instr::AddConst { dst, src, k } = instr {
                if *dst >= chunk.n_regs || *src >= chunk.n_regs {
                    return Err(RtError::InvalidBytecode(
                        "регистр сложения с константой выходит за кадр",
                    ));
                }
                if *k as usize >= chunk.consts.len() {
                    return Err(RtError::InvalidBytecode(
                        "номер константы сложения вне таблицы чанка",
                    ));
                }
            }
            // `NumericForNextI64` несёт предусловие «цель — собственный pc»:
            // на продолжении цикла она прыгает на `target`, а скрытое
            // состояние ищется по совпадению `state.pc == pc`. Цель, не
            // равная своей позиции, увела бы управление на чужую инструкцию,
            // а состояние осиротело бы.
            if let Instr::NumericForNextI64 { target, .. } = instr
                && *target as usize != pc
            {
                return Err(RtError::InvalidBytecode(
                    "числовой цикл: цель не указывает на собственную инструкцию",
                ));
            }
            check_call_geometry(program, chunk, instr)?;
        }
        for range in &chunk.exception_ranges {
            // `handler_pc == limit` ЗАКОННО и означает пустой обработчик в
            // конце чанка: управление уходит за последнюю инструкцию, то
            // есть в обычное завершение — ровно как у перехода с целью
            // `limit`. Строгое `>=` здесь однажды отвергло корректную
            // программу с пустым `Исключение` в конце.
            if range.start_pc > range.end_pc || range.end_pc > limit || range.handler_pc > limit {
                return Err(RtError::InvalidBytecode(
                    "диапазон «Попытка» за пределами чанка",
                ));
            }
        }
        // Чанк с `Выполнить`/`Вычислить` обязан нести ПОЛНУЮ таблицу имён
        // кадра: `run_dynamic_snippet` берёт `local_names` за весь набор
        // переменных, которые фрагмент видит и куда пишет обратно. Урезанная
        // таблица не ошибка для фрагмента — он просто не увидит переменную и
        // заведёт свою, а присваивание в неё не вернётся в кадр.
        if runs_dynamic {
            if chunk.local_names.len() != chunk.n_locals as usize {
                return Err(RtError::InvalidBytecode(
                    "чанк с «Выполнить» не несёт полной таблицы имён кадра",
                ));
            }
            // ...и таблица эта — биекция «имя ↔ слот», как и прочие именные
            // таблицы образа. Повтор связывает одно написание с двумя
            // слотами: фрагмент резолвит переменную в один из них, а
            // окружающий код читает другой, и присваивание уходит не в ту
            // переменную (воспроизведено: `Выполнить("х = 5")` записал 5 в
            // соседнюю `у`, вернув «1|5» вместо «5|2»).
            if bsl_rt::first_folded_duplicate(&chunk.local_names).is_some() {
                return Err(RtError::InvalidBytecode(
                    "имя локальной переменной кадра встречается дважды",
                ));
            }
        }
    }
    Ok(())
}

/// Таблицы программы, общие для всех чанков: имена, формы и блок модульных
/// переменных. Проверяются один раз на связывание, до первой инструкции, —
/// как и всё остальное в [`check_control_flow`], потому что каждое из этих
/// свойств при нарушении даёт не отказ, а неверный ответ.
fn check_program_tables(program: &Program) -> Result<(), RtError> {
    // Индексы полей формы — те же `NameId`, что и у инструкций доступа к
    // полю: форма, ссылающаяся за таблицу имён, даёт структуру с полем без
    // имени. Такое поле не найти по имени, а печать документа молча его
    // пропускает — вместо отказа получается неполный документ.
    for shape in &program.shapes {
        if shape
            .names
            .iter()
            .any(|id| id.index() >= program.names.len())
        {
            return Err(RtError::InvalidBytecode(
                "имя поля формы вне таблицы имён программы",
            ));
        }
        // Поля формы РАЗЛИЧНЫ: форма — это набор имён, а не список. Повтор
        // даёт структуре два слота под одним именем: `Количество()` считает
        // оба, а доступ по имени попадает в один из них — значение поля
        // берётся из чужого слота (воспроизведено: форма `[0 0]` вернула
        // «2|2» вместо «2|1»).
        //
        // Сравниваются ДЛИНЫ, а не сами номера: `Shape::index` собирается
        // ровно из `names` в `HashMap`, а тот на повторе перезаписывает
        // запись, — значит индекс короче списка тогда и только тогда, когда
        // номер повторился. Обход не нужен, память не выделяется, второй
        // копии правила не заводится. Инвариант держится на том, что форму
        // больше негде построить: `ShapeTable::intern_with_depth` —
        // единственное место с литералом `Shape`, приватные поля не дают
        // собрать её снаружи `bsl-rt`, разбор листинга тоже идёт через
        // `intern`, и `index` после построения никто не меняет.
        if shape.names.len() != shape.index.len() {
            return Err(RtError::InvalidBytecode(
                "форма структуры содержит повтор имени поля",
            ));
        }
    }
    // Модульные переменные принадлежат отдельному `ModuleState`, поэтому их
    // число не связано с `n_regs` верхнего кадра. Границы конкретных
    // `GetModuleVar`/`SetModuleVar` проверяются ниже по `module_vars`.

    // Подписи и тела связаны ПОЗИЦИОННО: `function_names[i]` — это
    // `chunks[i+1]`, а `chunks[0]` — тело модуля. Одной проверки каждого
    // `Call` мало: пары «имя без тела» и «лишний безымянный чанк» ломают
    // соответствие ЦЕЛИКОМ, даже когда на них никто не ссылается. Удаление
    // тела из середины сдвигает все последующие, и тогда номер, прошедший
    // проверку, исполняет ЧУЖУЮ функцию, а поиск по имени находит ещё
    // третью. Поэтому таблицы сверяются глобально, по длине.
    if program.chunks.len() != program.function_names.len() + 1 {
        return Err(RtError::InvalidBytecode(
            "число чанков не равно числу функций плюс чанк верхнего уровня",
        ));
    }
    // Таблицы экспорта параллельны таблицам имён: рассинхрон длины означал
    // бы, что признак читается у чужого элемента.
    if program.exported_functions.len() != program.function_names.len() {
        return Err(RtError::InvalidBytecode(
            "таблица экспорта функций не совпадает по длине с именами функций",
        ));
    }
    if program.exported_module_vars.len() != program.module_vars.len() {
        return Err(RtError::InvalidBytecode(
            "таблица экспорта переменных не совпадает по длине с переменными модуля",
        ));
    }
    // Таблицы имён обязаны быть биекциями `NameId <-> написание`: их строит
    // интернер, у которого одно написание даёт один идентификатор. Повтор
    // разводит ключи, которые обязаны совпадать, — по имени находится первое
    // вхождение, по номеру адресуется второе:
    //
    // * `names` — имена полей и методов. Повтор дал одному написанию два
    //   разных поля структуры (воспроизведено: `с.а` и `с.б` при обеих
    //   записях «а» вернули 1 и 2 вместо отказа).
    // * `function_names` — по имени зовут `call_module_function` и обратный
    //   вызов компонента, по номеру исполняется `Call`.
    // * `module_vars` и `top_level_locals` — по ним фрагмент
    //   `Выполнить`/`Вычислить` резолвит переменные окружающего кадра, а
    //   пишет их обратно ПО НОМЕРУ СЛОТА.
    //
    // Правило свёртки берётся у самого интернера (`bsl-rt`), а не пишется
    // здесь второй раз, и проход линейный.
    for (table, duplicate) in [
        (&program.names, "таблица имён содержит повтор написания"),
        (
            &program.function_names,
            "имя процедуры или функции встречается дважды",
        ),
        (
            &program.module_vars,
            "имя переменной модуля встречается дважды",
        ),
        (
            &program.top_level_locals,
            "имя локальной переменной верхнего уровня встречается дважды",
        ),
    ] {
        if bsl_rt::first_folded_duplicate(table).is_some() {
            return Err(RtError::InvalidBytecode(duplicate));
        }
    }
    Ok(())
}

/// Геометрия одной инструкции вызова и пролога умолчаний.
///
/// Три свойства, каждое из которых воспроизводится правкой одной строки в
/// листинге и каждое из которых VM до этой проверки принимала молча:
///
/// - `base + argc` за числом регистров кадра. Номера регистров — `u8`, и
///   `base + i` заворачивалось: аргумент становился алиасом ЧУЖОГО
///   регистра вызывающего. С режимом `Default` вызванная функция ещё и
///   пишет туда своё умолчание, то есть портит переменную вызывающего.
/// - Длина набора режимов, не равная числу параметров вызываемой функции.
///   Остальная геометрия кадра считается по `n_params`, поэтому лишний
///   режим превращает СОБСТВЕННЫЙ регистр вызванной функции в алиас слота
///   вызывающего, а недостающий оставляет параметр без слота.
/// - `src` пролога умолчаний за числом параметров. Слот с таким номером
///   VM не находит и считает аргумент переданным, поэтому умолчание не
///   вычисляется и функция возвращает `Неопределено`.
fn check_call_geometry(program: &Program, chunk: &Chunk, instr: &Instr) -> Result<(), RtError> {
    match instr {
        Instr::JumpIfNotSkipped { src, .. } if *src >= chunk.n_params => Err(
            RtError::InvalidBytecode("пролог умолчаний ссылается на несуществующий параметр"),
        ),
        Instr::Call {
            func,
            base,
            arg_modes,
            ..
        } => {
            // `func` адресует ДВЕ связанные таблицы: `function_names[func-1]`
            // — подпись, `chunks[func]` — тело. Обе границы проверяются
            // вместе, ровно как их проверяет печать листинга
            // (`BadCallTarget`, см. `bsl-bytecode`); разбор листинга их не
            // проверяет, поэтому периметр образа обязан закрыть это здесь.
            //
            // Ноль — это `chunks[0]`, тело модуля, которого не вызывает
            // никто: `chunks.get(0)` вернул бы `Some`, и вызов рекурсивно
            // входил бы в верхний уровень — получался ловимый `StackOverflow`
            // (в отличие от `InvalidBytecode` он считается исключением BSL),
            // который окружающая `Попытка` проглотила бы, обратив битый образ
            // в неверный ответ с кодом успеха.
            //
            // Номер больше числа имён — тело БЕЗ подписи. Такой чанк
            // по-прежнему вызывается статически, но для `Выполнить`/
            // `Вычислить` его не существует: фрагмент видит функции модуля
            // ровно как `function_names`, сшитые с `chunks[i+1]` (см.
            // `run_dynamic_snippet`). Один и тот же образ означал бы разное
            // на статическом и на динамическом пути.
            if *func == 0 || *func as usize > program.function_names.len() {
                return Err(RtError::InvalidBytecode(
                    "номер вызываемой функции — ноль или вне таблицы имён функций",
                ));
            }
            let Some(callee) = program.chunks.get(*func as usize) else {
                return Err(RtError::InvalidBytecode(
                    "номер вызываемого чанка вне таблицы функций",
                ));
            };
            let Some(modes) = chunk.call_arg_modes.get(*arg_modes as usize) else {
                return Err(RtError::InvalidBytecode(
                    "номер набора режимов аргументов вне таблицы чанка",
                ));
            };
            if modes.len() != callee.n_params as usize {
                return Err(RtError::InvalidBytecode(
                    "режимов аргументов не столько, сколько параметров у вызываемой функции",
                ));
            }
            if *base as usize + modes.len() > chunk.n_regs as usize {
                return Err(RtError::InvalidBytecode(
                    "регистры аргументов вызова выходят за кадр",
                ));
            }
            // Параметр по ссылке (`byref:slot`) — алиас ЛОКАЛИ вызывающего.
            // Граница — именно `n_locals`, а не `n_regs`: временный регистр
            // локальной переменной не является, и проверка по `n_regs`
            // пропустила бы алиас на чужой временный слот.
            for (i, mode) in modes.iter().enumerate() {
                // Режим места вызова обязан согласовываться с ОБЪЯВЛЕНИЕМ
                // параметра: кадр строится по режимам, а не по `param_by_val`,
                // поэтому `byref` против `Знач` молча превращает копию в алиас
                // — вызванная функция пишет в переменную вызывающего там, где
                // исходник этого не позволяет. Обратное направление
                // (`Value` против параметра без `Знач`) законно: так уходит
                // аргумент-выражение, у которого нет слота (`Ф(х + 1)`).
                // `get`, а не индексация: длину `param_by_val` проверяет
                // отдельная проверка на СВОЁМ чанке, а чанки обходятся по
                // порядку — вызов может встретиться раньше, чем дойдёт черёд
                // до вызываемого. Короткий массив здесь просто не срабатывает,
                // а образ всё равно будет отвергнут этой проверкой.
                if callee.param_by_val.get(i) == Some(&true)
                    && matches!(
                        mode,
                        ArgMode::ByRefLocal(_)
                            | ArgMode::ByRefModuleVar(_)
                            | ArgMode::ByRefImportedVar(_)
                    )
                {
                    return Err(RtError::InvalidBytecode(
                        "вызов передаёт по ссылке параметр, объявленный «Знач»",
                    ));
                }
                match mode {
                    ArgMode::ByRefLocal(slot) if *slot as usize >= chunk.n_locals as usize => {
                        return Err(RtError::InvalidBytecode(
                            "параметр по ссылке указывает за локали кадра",
                        ));
                    }
                    // Модульная переменная по ссылке — алиас module-слота;
                    // граница — число переменных модуля, иначе алиас указывал
                    // бы за таблицу модульных слотов.
                    ArgMode::ByRefModuleVar(slot)
                        if *slot as usize >= program.module_vars.len() =>
                    {
                        return Err(RtError::InvalidBytecode(
                            "параметр по ссылке указывает за переменные модуля",
                        ));
                    }
                    // Импортированная переменная по ссылке обязана вести на
                    // запись-переменную СВОЕЙ таблицы связей; чужой модуль и
                    // его слот проверяет `verify_configuration`.
                    ArgMode::ByRefImportedVar(slot)
                        if !matches!(
                            program.links.get(*slot as usize),
                            Some(crate::configuration::LinkEntry::Variable { .. })
                        ) =>
                    {
                        return Err(RtError::InvalidBytecode(
                            "byimport ведёт мимо таблицы связей или на функцию",
                        ));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        Instr::CallImported {
            link_slot,
            base,
            arg_modes,
            ..
        } => {
            // Вид записи связи закреплён заранее: исполнить переменную
            // нельзя уже на периметре образа, а не в рантайме. Арность и
            // согласование «Знач» проверяет `verify_configuration` — целевой
            // чанк лежит в другом модуле каталога.
            if !matches!(
                program.links.get(*link_slot as usize),
                Some(crate::configuration::LinkEntry::Function { .. })
            ) {
                return Err(RtError::InvalidBytecode(
                    "CallImported ведёт мимо таблицы связей или на переменную",
                ));
            }
            let Some(modes) = chunk.call_arg_modes.get(*arg_modes as usize) else {
                return Err(RtError::InvalidBytecode(
                    "номер набора режимов аргументов вне таблицы чанка",
                ));
            };
            if *base as usize + modes.len() > chunk.n_regs as usize {
                return Err(RtError::InvalidBytecode(
                    "регистры аргументов вызова выходят за кадр",
                ));
            }
            for mode in modes {
                match mode {
                    ArgMode::ByRefLocal(slot) if *slot as usize >= chunk.n_locals as usize => {
                        return Err(RtError::InvalidBytecode(
                            "параметр по ссылке указывает за локали кадра",
                        ));
                    }
                    ArgMode::ByRefModuleVar(slot)
                        if *slot as usize >= program.module_vars.len() =>
                    {
                        return Err(RtError::InvalidBytecode(
                            "параметр по ссылке указывает за переменные модуля",
                        ));
                    }
                    ArgMode::ByRefImportedVar(slot)
                        if !matches!(
                            program.links.get(*slot as usize),
                            Some(crate::configuration::LinkEntry::Variable { .. })
                        ) =>
                    {
                        return Err(RtError::InvalidBytecode(
                            "byimport ведёт мимо таблицы связей или на функцию",
                        ));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        // Импортные обращения к переменным: вид записи связи — Variable.
        Instr::GetImportedVar { link_slot, .. } | Instr::SetImportedVar { link_slot, .. } => {
            if !matches!(
                program.links.get(*link_slot as usize),
                Some(crate::configuration::LinkEntry::Variable { .. })
            ) {
                return Err(RtError::InvalidBytecode(
                    "импортная переменная ведёт мимо таблицы связей или на функцию",
                ));
            }
            Ok(())
        }
        Instr::NewStructure {
            shape, base, count, ..
        } => {
            let Some(shape_rc) = program.shapes.get(*shape as usize) else {
                return Err(RtError::InvalidBytecode(
                    "номер формы структуры вне таблицы форм программы",
                ));
            };
            if *count as usize != shape_rc.names.len() {
                return Err(RtError::InvalidBytecode(
                    "число полей структуры не равно длине её формы",
                ));
            }
            // `base + i` в исполнении считается как `u8`, поэтому границу
            // проверяем тем же типом через `checked_add`: заворот дал бы
            // первое поле верным, а следующее — алиасом чужого регистра.
            match (*base).checked_add(*count) {
                Some(end) if end as usize <= chunk.n_regs as usize => Ok(()),
                _ => Err(RtError::InvalidBytecode(
                    "регистры полей структуры выходят за кадр",
                )),
            }
        }
        // Номер имени поля или метода — индекс в таблице имён программы. У
        // ЗАКРЫТЫХ опкодов (`GetProp`/`SetProp`) висячий номер даёт
        // `InvalidBytecode` уже в рантайме, а у ОТКРЫТЫХ (объектных) — нет:
        // там имя резолвится только на ветке «не объект», а натуральный
        // получатель отвечает обычным ловимым «нет такого свойства/метода».
        // `Попытка` вокруг такого доступа обращает битый образ в неверный
        // ответ с кодом успеха, поэтому граница проверяется статически и для
        // всех пяти опкодов сразу.
        Instr::GetProp { name, .. } | Instr::SetProp { name, .. } => {
            if name.index() >= program.names.len() {
                return Err(RtError::InvalidBytecode(
                    "номер имени поля вне таблицы имён программы",
                ));
            }
            Ok(())
        }
        Instr::GetObjectProp { name, .. }
        | Instr::SetObjectProp { name, .. }
        | Instr::CallObjectMethod { method: name, .. } => {
            if *name as usize >= program.names.len() {
                return Err(RtError::InvalidBytecode(
                    "номер имени свойства или метода вне таблицы имён программы",
                ));
            }
            Ok(())
        }
        // Число аргументов встроенной функции и метода проверяется здесь, на
        // связывании: `bsl-vm` не видит `bsl-sema`, и без этого крафтнутый
        // байт-код с недостающим аргументом ронял бы `call_builtin_*` на
        // `args[0]`. Арности берутся из `bsl-rt` — единственного источника,
        // видного и резолверу, и VM.
        Instr::CallBuiltin { builtin, count, .. } => {
            let (min, max) = builtin.arity_range();
            let argc = *count as usize;
            if argc < min || argc > max {
                return Err(RtError::InvalidBytecode(
                    "число аргументов встроенной функции вне её арности",
                ));
            }
            Ok(())
        }
        Instr::CallMethod { method, count, .. } => {
            // `None` — арность полиморфна по получателю; её проверяет сам
            // обработчик в рантайме (ловимо `Попыткой`), как на платформе.
            if let Some(expected) = method.static_arity()
                && *count as usize != expected
            {
                return Err(RtError::InvalidBytecode(
                    "число аргументов метода не совпадает с его арностью",
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Периметр конфигурационного образа: то, что нельзя проверить на одном
/// модуле, потому что цель связи лежит в другом.
///
/// Порядок обязанностей: сначала каждый модуль (и entry, если он есть)
/// проходит одиночный [`verify`], затем сверяются межмодульные свойства —
/// уникальность имён каталога, разрешимость и экспортность целей связей,
/// ацикличность графа импортов, арность и согласование «Знач» у
/// `CallImported`. Worker принимает каталог ровно через эту проверку и не
/// повторяет case-insensitive resolution имён: manifest уже числовой.
///
/// # Errors
///
/// `RtError::InvalidBytecode` с описанием первого найденного нарушения.
pub fn verify_configuration(
    catalog: &crate::ConfigurationProgram,
    entry: Option<&crate::EntryProgram>,
) -> Result<(), RtError> {
    use crate::configuration::LinkEntry;

    let names: Vec<&String> = catalog.modules.iter().map(|m| &m.name).collect();
    for name in &names {
        if name.is_empty() {
            return Err(RtError::InvalidBytecode("имя общего модуля пусто"));
        }
    }
    let owned: Vec<String> = names.iter().map(|n| n.as_str().to_owned()).collect();
    if bsl_rt::first_folded_duplicate(&owned).is_some() {
        return Err(RtError::InvalidBytecode(
            "имена общих модулей совпадают без учёта регистра",
        ));
    }
    if catalog.modules.len() > u32::MAX as usize {
        return Err(RtError::InvalidBytecode("каталог шире адресации ModuleId"));
    }

    for module in &catalog.modules {
        verify(&module.program)?;
    }
    if let Some(entry) = entry {
        verify(&entry.program)?;
    }

    // Разрешимость и экспортность целей. `own` — номер модуля-владельца
    // таблицы; у entry владельца в каталоге нет.
    let check_links = |program: &Program, own: Option<usize>| -> Result<(), RtError> {
        for link in &program.links {
            let (module, target_is_function) = match link {
                LinkEntry::Function { module, .. } => (*module, true),
                LinkEntry::Variable { module, .. } => (*module, false),
            };
            if own == Some(module.index()) {
                return Err(RtError::InvalidBytecode(
                    "связь указывает на собственный модуль",
                ));
            }
            let Some(target) = catalog.module(module) else {
                return Err(RtError::InvalidBytecode("связь ведёт мимо каталога"));
            };
            match link {
                LinkEntry::Function { func, .. } => {
                    debug_assert!(target_is_function);
                    let index = *func as usize;
                    if index == 0
                        || index >= target.program.chunks.len()
                        || index > target.program.function_names.len()
                    {
                        return Err(RtError::InvalidBytecode(
                            "связь ведёт на несуществующую функцию модуля",
                        ));
                    }
                    if !target.program.exported_functions[index - 1] {
                        return Err(RtError::InvalidBytecode(
                            "связь ведёт на неэкспортный метод модуля",
                        ));
                    }
                }
                LinkEntry::Variable { slot, .. } => {
                    let index = *slot as usize;
                    if index >= target.program.module_vars.len() {
                        return Err(RtError::InvalidBytecode(
                            "связь ведёт на несуществующую переменную модуля",
                        ));
                    }
                    if !target.program.exported_module_vars[index] {
                        return Err(RtError::InvalidBytecode(
                            "связь ведёт на неэкспортную переменную модуля",
                        ));
                    }
                }
            }
        }
        Ok(())
    };
    for (i, module) in catalog.modules.iter().enumerate() {
        check_links(&module.program, Some(i))?;
    }
    if let Some(entry) = entry {
        check_links(&entry.program, None)?;
    }

    // Граф импортов обязан быть ациклическим: цикл превратил бы ленивую
    // инициализацию модулей в бесконечную рекурсию, а CLI-проверка циклов
    // файлового графа образу, пришедшему извне, ничем не помогает.
    let mut state = vec![0u8; catalog.modules.len()]; // 0 — белый, 1 — серый, 2 — чёрный
    fn visit(
        catalog: &crate::ConfigurationProgram,
        state: &mut [u8],
        index: usize,
    ) -> Result<(), RtError> {
        match state[index] {
            1 => {
                return Err(RtError::InvalidBytecode(
                    "граф импортов модулей содержит цикл",
                ));
            }
            2 => return Ok(()),
            _ => {}
        }
        state[index] = 1;
        for link in &catalog.modules[index].program.links {
            let target = match link {
                LinkEntry::Function { module, .. } | LinkEntry::Variable { module, .. } => {
                    module.index()
                }
            };
            visit(catalog, state, target)?;
        }
        state[index] = 2;
        Ok(())
    }
    for index in 0..catalog.modules.len() {
        visit(catalog, &mut state, index)?;
    }

    // Арность и «Знач» у CallImported: целевой чанк известен только на
    // уровне каталога. Локальная геометрия (`base`, границы режимов) уже
    // проверена одиночным verify.
    let check_calls = |program: &Program| -> Result<(), RtError> {
        for chunk in &program.chunks {
            for instr in &chunk.instrs {
                let Instr::CallImported {
                    link_slot,
                    arg_modes,
                    ..
                } = instr
                else {
                    continue;
                };
                let Some(LinkEntry::Function { module, func }) =
                    program.links.get(*link_slot as usize)
                else {
                    unreachable!("одиночный verify уже проверил вид связи");
                };
                let callee = &catalog.modules[module.index()].program.chunks[*func as usize];
                let modes = &chunk.call_arg_modes[*arg_modes as usize];
                if modes.len() != callee.n_params as usize {
                    return Err(RtError::InvalidBytecode(
                        "режимов аргументов не столько, сколько параметров у импортированной функции",
                    ));
                }
                for (i, mode) in modes.iter().enumerate() {
                    if callee.param_by_val.get(i) == Some(&true)
                        && matches!(
                            mode,
                            ArgMode::ByRefLocal(_)
                                | ArgMode::ByRefModuleVar(_)
                                | ArgMode::ByRefImportedVar(_)
                        )
                    {
                        return Err(RtError::InvalidBytecode(
                            "вызов передаёт по ссылке параметр, объявленный «Знач»",
                        ));
                    }
                }
            }
        }
        Ok(())
    };
    for module in &catalog.modules {
        check_calls(&module.program)?;
    }
    if let Some(entry) = entry {
        check_calls(&entry.program)?;
    }
    Ok(())
}
