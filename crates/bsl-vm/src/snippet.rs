use super::{
    Frame, HostIo, JitMode, LinkedComponents, ModuleState, at, drive_linked, link_components,
    reg_load, reg_store,
};
use bsl_bytecode::{Instr, Program};
use bsl_rt::{BslValue, RtError};

/// Предел вложенности `Выполнить`/`Вычислить` друг в друге. В отличие от
/// кадров BSL, каждый уровень динамического кода — это настоящий вложенный
/// `drive` на стеке Rust (плюс разбор и компиляция фрагмента), поэтому
/// предел защищает стек процесса, а не память: без него рекурсия через
/// `Выполнить` валит процесс переполнением стека, минуя `Попытка`.
// НЕ ИЗМЕРЕНО(EXEC.DYNAMIC_DEPTH) — сколько уровней допускает платформа;
// замер даёт только нижнюю границу (40 уровней обязаны работать).
const MAX_DYNAMIC_DEPTH: usize = 64;

/// Вход в очередной уровень динамического кода; выход — в `Drop`, чтобы
/// счётчик не съезжал ни на одном из путей ошибки.
///
/// Счётчик — не потоковый, а поле [`HostIo`] прогона: две сессии в одном
/// потоке (например, вложенный `Engine` за обратным вызовом функции) не
/// делят вложенность `Выполнить`. Вложенный `drive` переиспользует тот же
/// `HostIo`, а обратный вызов функции модуля строит новый — но с тем же
/// `dynamic_depth` родителя, — так что уровень протаскивается через
/// прогон, а не через поток.
pub(super) struct DynamicDepthGuard<'a> {
    depth: &'a std::cell::Cell<usize>,
}

impl<'a> DynamicDepthGuard<'a> {
    pub(super) fn enter(depth: &'a std::cell::Cell<usize>) -> Result<Self, RtError> {
        if depth.get() >= MAX_DYNAMIC_DEPTH {
            Err(RtError::StackOverflow {
                what: "слишком глубокая вложенность Выполнить/Вычислить",
            })
        } else {
            depth.set(depth.get() + 1);
            Ok(DynamicDepthGuard { depth })
        }
    }
}

impl Drop for DynamicDepthGuard<'_> {
    fn drop(&mut self) {
        self.depth.set(self.depth.get() - 1);
    }
}

/// Исполняет `code` в контексте переменных текущего кадра (см.
/// `Instr::RunDynamic`).
///
/// Компилирует НЕ эта функция: текст, вид операции и описание области
/// уходят компилятору хоста (`bsl_bytecode::DynamicCompiler`), а обратно
/// приходит готовый `DynamicUnit`. Здесь остаётся ровно механика
/// исполнения — перенос значений внутрь фрагмента и обратно.
///
/// Изолированность: фрагмент исполняется на ОТДЕЛЬНОМ стеке — копии
/// текущих значений top-level переменных плюс новые слоты под то, что
/// фрагмент сам объявит. После исполнения обратно во внешний кадр
/// переносятся только значения уже существовавших до вызова слотов
/// (`0..old_count`) — их регистровые номера точно совпадают с тем, что
/// уже использует статический код вокруг, так что перезапись безопасна.
/// Новые имена, объявленные фрагментом, никуда не переносятся: чтобы это
/// сделать, статический код вокруг пришлось бы компилировать в режиме
/// материализованного кадра с именной таблицей (см. бриф) — это
/// отдельная, ещё не сделанная работа.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_dynamic_snippet(
    code: &str,
    is_eval: bool,
    program: &Program,
    scope_locals: &[String],
    scope_id: usize,
    stack: &mut [BslValue],
    frame: &Frame,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
) -> Result<BslValue, RtError> {
    // Предел вложенности — на входе, до обращения к хосту: компиляция
    // фрагмента рекурсивна так же, как его исполнение, и тоже расходует
    // стек Rust.
    let _depth = DynamicDepthGuard::enter(host.dynamic_depth)?;

    // Что фрагмент знает о функциях модуля, в порядке `chunks[1..]`: имя,
    // арность, вид объявления и режимы параметров. Всё заимствовано у
    // программы — запрос собирается на каждом `RunDynamic`, в том числе
    // когда фрагмент уже лежит в кэше хоста.
    let functions: Vec<bsl_bytecode::DynamicSignature<'_>> = program
        .function_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let chunk = program.chunks.get(i + 1);
            bsl_bytecode::DynamicSignature {
                name,
                arity: chunk.map_or(0, |c| c.n_params as usize),
                is_procedure: chunk.is_some_and(|c| c.is_procedure),
                is_async: chunk.is_some_and(|c| c.is_async),
                param_by_val: chunk.map_or(&[][..], |c| &c.param_by_val),
                param_has_default: chunk.map_or(&[][..], |c| &c.param_has_default),
            }
        })
        .collect();
    let request = bsl_bytecode::DynamicRequest {
        source: code,
        // Отладочность — свойство ОБЪЕМЛЮЩЕЙ программы, и признак её —
        // непустая таблица строк. Отдельного ключа у фрагмента нет:
        // отлаживают либо всё, либо ничего.
        debug_info: !program.lines.is_empty(),
        kind: if is_eval {
            bsl_bytecode::DynamicKind::Eval
        } else {
            bsl_bytecode::DynamicKind::Execute
        },
        scope: bsl_bytecode::DynamicScope {
            // Чанки 1.. — процедуры и функции ИСХОДНОГО модуля: во
            // фрагмент они едут как есть, таблицы локальных у них те же, и
            // область у них корневая на любой глубине вложенности. Своя
            // таблица только у нулевого чанка — вот он и берёт номер того,
            // чей он: модуля или фрагмента вокруг.
            program: if scope_id == 0 {
                linked.scope
            } else {
                bsl_bytecode::DynamicScope::ROOT
            },
            chunk: scope_id as u32,
        },
        caller_is_async: at(
            &program.chunks,
            scope_id,
            "номер чанка динамического вызова вне таблицы функций",
        )?
        .is_async,
        locals: scope_locals,
        module_vars: &program.module_vars,
        functions: &functions,
        names: &program.names,
        requirements: &program.requirements,
    };
    // Неудача любой фазы фронтенда — обычное исключение В МОМЕНТ
    // ИСПОЛНЕНИЯ, а не паника: текст фрагмента становится известен только
    // сейчас, и кривой текст обязан ловиться `Попытка`. Поэтому контракт
    // хоста и отдаёт текст ошибки, а не готовый `RtError`: каким видом
    // ошибки станет неудача, решает VM, а не хост.
    let compiled = host
        .dynamic()?
        .compile(&request)
        .map_err(RtError::DynamicError)?;

    // Значения существующих переменных кадра переезжают во фрагмент по
    // НОМЕРУ СЛОТА: раскладка совпадает, потому что фрагмент резолвился
    // поверх ровно этого `scope_locals` (см. `resolve_snippet_stmts`).
    // `reg_index` здесь обязателен, а не голое `stack[i]`: у кадра функции
    // параметры — алиасы на слоты ВЫЗЫВАЮЩЕГО, и параметр по ссылке
    // обязан быть виден фрагменту тем же, чем он виден статическому коду.
    let old_count = scope_locals.len();
    let mut snippet_stack: Vec<BslValue> = (0..old_count)
        .map(|i| reg_load(stack, frame.reg_index(i as u8)))
        .collect::<Result<_, _>>()?;
    snippet_stack.resize(compiled.chunk.n_regs as usize, BslValue::Undefined);

    // Чанки функций едут во фрагмент КАК ЕСТЬ, только нулевой заменён на
    // сам фрагмент: измерено, что `Вычислить("Удвоить(21)")` на платформе
    // работает, а `Call func=N` у нас индексирует ровно `chunks[N]`.
    // Поэтому нумерация обязана совпасть с исходной программой.
    let mut chunks = program.chunks.clone();
    for chunk in &mut chunks {
        remap_chunk_libraries(chunk, &program.requirements, &compiled.requirements)?;
    }
    if chunks.is_empty() {
        chunks.push(compiled.chunk.clone());
    } else {
        chunks[0] = compiled.chunk.clone();
    }
    // Таблица строк собирается так же, как чанки: чужие записи переезжают
    // как есть — они описывают те же инструкции тех же функций и остаются
    // в координатах файла, — а нулевая заменяется строками фрагмента, то
    // есть переходит в координаты его СОБСТВЕННОГО текста.
    //
    // Различить эти две системы по самой таблице нельзя: у чанка нет
    // отметки, какому источнику он принадлежит. Здесь это и не нужно —
    // нулевой чанк программы фрагмента И ЕСТЬ фрагмент, — но отладчику
    // нужно, и там это отдельная работа (`dap-debugger`, показ кадра со
    // своим источником).
    let mut lines = program.lines.clone();
    if !lines.is_empty() || !compiled.lines.is_empty() {
        lines.resize(chunks.len(), Vec::new());
        lines[0] = compiled.lines.clone();
    }
    // Разметка бандлов фрагмента остаётся ПУСТОЙ (поинструкционное
    // исполнение). Прежний расчёт в `compile_snippet` звал `compute` с
    // `overlap = None`, тогда как таблица эффектов считает модульный слот
    // нулевого чанка пересекающимся с регистром кадра — пересечение это
    // сегодня преднамеренно ложное (см. модульный доклад `analysis`), но
    // пока оно в модели, разметка с `None` расходится с нею и делает
    // утверждение о независимости, которого модель не даёт. Верный
    // `overlap` известен только здесь, но пересчитывать его на КАЖДОМ
    // `Выполнить` —
    // измеренные +9.6 % на eval-в-цикле ради соундности доказательства,
    // которое над фрагментами ни в рантайме, ни в тестах не проверяется
    // (`bundle::verify` идёт по статическому корпусу). Пустой вектор не
    // делает никакого утверждения — он и сонадёжен, и бесплатен; фрагмент
    // одноразовый, потеря пакетной диспетчеризации на нём незначима.
    let snippet_program = Program {
        requirements: compiled.requirements.clone(),
        chunks,
        // СОБСТВЕННАЯ таблица фрагмента, не `program.names`: она — префикс
        // (те же имена, в том же порядке, значит те же `NameId`) плюс,
        // возможно, новые поля, которых в статическом коде не было (см.
        // doc comment `bsl_bytecode::DynamicUnit`). Старые `GetProp`/`SetProp` — и
        // статического кода вокруг, и вложенных вызовов функций программы
        // (см. ниже про `chunks`) — по-прежнему резолвятся: их `NameId`
        // меньше длины `program.names` и указывают на тот же префикс.
        names: compiled.names.clone(),
        shapes: compiled.shapes.clone(),
        top_level_locals: Vec::new(),
        function_names: program.function_names.clone(),
        exported_functions: program.exported_functions.clone(),
        module_vars: program.module_vars.clone(),
        exported_module_vars: program.exported_module_vars.clone(),
        links: Vec::new(),
        // Таблица, собранная выше: чужие записи в координатах файла,
        // нулевая — в координатах собственного текста фрагмента. Раньше
        // здесь стоял пустой вектор, и вся работа выше пропадала.
        lines,
    };

    let snippet_linked = link_components(
        &snippet_program,
        linked.registry,
        std::rc::Rc::clone(&linked.zone),
        std::rc::Rc::clone(&linked.files),
        linked.random.clone(),
        linked.network.as_ref().map(std::rc::Rc::clone),
        linked.background_jobs.as_ref().map(std::rc::Rc::clone),
        linked.temp_storage.as_ref().map(std::rc::Rc::clone),
        linked.message_sink.as_ref().map(std::rc::Rc::clone),
        compiled.scope.get(),
    )?;
    let (value, final_stack) = drive_linked(
        &snippet_program,
        0,
        snippet_stack,
        JitMode::Off,
        &snippet_linked,
        host,
        module_state,
        None,
    )?;

    // Обратно переносятся ТОЛЬКО уже существовавшие слоты: их номера
    // совпадают с теми, что использует окружающий скомпилированный код.
    // Имена, объявленные самим фрагментом, получили слоты ЗА `old_count` и
    // никуда не переносятся — расширить статически размеченный кадр нечем.
    //
    // ИЗМЕРЕНО на 8.3.27: платформа ведёт себя ТАК ЖЕ — имя, впервые
    // созданное внутри `Выполнить`, вызов не переживает. Выбор оказался
    // верным, кадр с именной таблицей не нужен.
    //
    for i in 0..old_count {
        let d = frame.reg_index(i as u8);
        reg_store(stack, d, reg_load(&final_stack, i)?)?;
    }

    Ok(value)
}

fn remap_chunk_libraries(
    chunk: &mut bsl_bytecode::Chunk,
    from: &[bsl_bytecode::LibraryRequirement],
    to: &[bsl_bytecode::LibraryRequirement],
) -> Result<(), RtError> {
    for instruction in &mut chunk.instrs {
        let library = match instruction {
            Instr::CallComponent { library, .. } | Instr::CreateObject { library, .. } => library,
            _ => continue,
        };
        let requirement = from.get(*library as usize).ok_or(RtError::InvalidBytecode(
            "индекс библиотеки вне таблицы requirements",
        ))?;
        let target = to
            .iter()
            .position(|candidate| candidate.package == requirement.package)
            .ok_or(RtError::InvalidBytecode(
                "компонент чанка отсутствует в объединённых requirements",
            ))?;
        *library = target.try_into().map_err(|_| {
            RtError::InvalidBytecode("индекс библиотеки не помещается в операнд u8")
        })?;
    }
    // Чанки едут во фрагмент ПО ОДНОМУ, программы вокруг них здесь нет,
    // поэтому финализация не программы, а одиночного чанка: нулевой
    // будет заменён самим фрагментом (его разметка остаётся пустой —
    // поинструкционное исполнение), а у остальных пересечения с
    // модульными слотами нет по определению `module_overlap`.
    bsl_bytecode::image::finalize_lone_chunk(chunk);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Счётчик вложенности `Выполнить`/`Вычислить` принадлежит ПРОГОНУ, а не
    /// потоку (шаг 16 плана abi-refactor-f). Раньше он был `thread_local` и
    /// делился между сессиями одного потока: набранная одной сессией глубина
    /// урезала бы вложенность у другой (например, у вложенного `Engine` за
    /// обратным вызовом). Теперь у каждого `HostIo` свой `Cell`, и полностью
    /// занятый счётчик одной сессии не мешает другой.
    #[test]
    fn two_sessions_do_not_share_dynamic_nesting_depth() {
        let first = std::cell::Cell::new(0);
        let second = std::cell::Cell::new(0);

        let mut held = Vec::new();
        for _ in 0..MAX_DYNAMIC_DEPTH {
            held.push(DynamicDepthGuard::enter(&first).expect("в пределах лимита"));
        }
        // Первая сессия заполнена — следующий уровень в ней отвергается.
        assert!(DynamicDepthGuard::enter(&first).is_err());
        // Вторая сессия того же потока не затронута.
        assert!(DynamicDepthGuard::enter(&second).is_ok());
    }
}
