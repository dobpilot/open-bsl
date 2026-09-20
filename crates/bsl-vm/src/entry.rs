use super::snippet::DynamicDepthGuard;
use super::{
    HostIo, LinkedComponents, ModuleState, at, drive_linked, link_components, push_own_registers,
    reg_load, reg_store,
};
use bsl_bytecode::{DynamicCompiler, Program};
use bsl_rt::{BslValue, RtError};
use std::io::Write;

/// Выполняет модуль с точки входа — операторов верхнего уровня (`chunks[0]`)
/// — и возвращает значение, которым он завершился (через `Возврат` на
/// верхнем уровне, что нетипично, но не запрещено; обычно — `Неопределено`).
///
/// Исключения (`Попытка`/`ВызватьИсключение`) ловятся здесь, а не внутри
/// `step`: если очередная инструкция вернула `Err`, кадр(ы) разматываются
/// (`unwind_to_handler`) в поисках защищённого диапазона, который её
/// накрывает — начиная с того кадра, где ошибка произошла, и дальше наружу
/// через вызовы. Не нашли нигде — ошибка настоящая, возвращаем её вызывающему
/// Rust-коду.
///
/// Компилятора динамического кода у этого входа нет: `Выполнить` и
/// `Вычислить` дают ловимую [`RtError::DynamicError`]. Прогон с
/// динамическим кодом идёт через `*_and_io` — там компилятор фрагментов
/// передаётся явно (см. [`bsl_bytecode::DynamicCompiler`]).
///
/// # Errors
///
/// Возвращает [`RtError`], если выполнение завершилось неперехваченным исключением или
/// программа содержит некорректный байт-код.
pub fn run_program(program: &Program) -> Result<BslValue, RtError> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut env = bsl_rt::HostEnv::process();
    run_program_with_host(program, None, &mut stdout, &mut stderr, None, &mut env)
}

/// Исполняет программу с выводом в потоки, принадлежащие host-приложению.
/// `Сообщить` пишет только в `stdout`; библиотечный API возвращает ошибки и
/// не печатает их в `stderr` автоматически.
///
/// `dynamic` — компилятор `Выполнить`/`Вычислить` этого прогона. VM
/// динамический код только исполняет: текст, вид операции и описание
/// области видимости уходят сюда, а обратно приходит готовый чанк.
///
/// # Errors
///
/// До первой инструкции возвращает [`RtError::Link`], если требуемый пакет,
/// версия или код функции отсутствует; далее — те же ошибки, что
/// [`run_program`], включая ошибку записи в пользовательский поток.
pub fn run_program_with_registry_and_io<'a>(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: &'a mut dyn DynamicCompiler,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<BslValue, RtError> {
    run_program_with_host(
        program,
        Some(registry),
        stdout,
        stderr,
        Some(dynamic),
        host_env,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_program_with_host<'a>(
    program: &Program,
    registry: Option<&bsl_rt::RuntimeRegistry>,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: Option<&'a mut dyn DynamicCompiler>,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<BslValue, RtError> {
    let mut stack = Vec::new();
    push_own_registers(
        &mut stack,
        at(&program.chunks, 0, "в программе нет чанка верхнего уровня")?,
    );
    let linked = link_components(
        program,
        registry,
        host_env.zone(),
        host_env.files(),
        host_env.random(),
        host_env.network(),
        host_env.background_jobs(),
        host_env.temp_storage(),
        host_env.message_sink(),
        bsl_bytecode::DynamicScope::ROOT,
    )?;
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout,
        stderr,
        env: Some(host_env),
        dynamic,
        dynamic_depth: &dynamic_depth,
        cancel_flag: None,
        file_promises: None,
    };
    let mut module_state = ModuleState::new(program);
    let (value, _) = drive_linked(
        program,
        0,
        stack,
        &linked,
        &mut host,
        &mut module_state,
        None,
    )?;
    Ok(value)
}

/// Исполняет чанк REPL с каталогом компонентов: фрагмент, скомпилированный
/// с реестром, несёт `CreateObject`/`CallComponent`, и его требования
/// связываются перед исполнением. Стек предыдущего чанка передаётся внутрь
/// и возвращается наружу — так в сессии живут накопленные переменные.
///
/// # Errors
///
/// Возвращает [`RtError`] при неперехваченном исключении или некорректных
/// таблицах имён, форм и регистров чанка, плюс ошибку
/// связывания компонентов.
// Семь параметров сверх `unit` — это состояние REPL-сессии, разложенное по
// местам: локали, стек и требования собираются вызывающим по одному, а
// потоки и окружение — сервисы прогона. Чанк, имена и формы, чей
// инвариант связан позицией, теперь приезжают одним [`SnippetUnit`].
#[allow(clippy::too_many_arguments)]
pub fn run_repl_chunk_with_registry<'a>(
    unit: &bsl_bytecode::SnippetUnit,
    locals: Vec<String>,
    stack: Vec<BslValue>,
    requirements: Vec<bsl_bytecode::LibraryRequirement>,
    registry: &bsl_rt::RuntimeRegistry,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: &'a mut dyn DynamicCompiler,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = Program {
        requirements,
        chunks: vec![unit.chunk.clone()],
        names: unit.names.clone(),
        shapes: unit.shapes.clone(),
        top_level_locals: locals,
        function_names: Vec::new(),
        exported_functions: Vec::new(),
        module_vars: Vec::new(),
        exported_module_vars: Vec::new(),
        imports: Vec::new(),
        links: Vec::new(),
        // Чанк REPL собирается без сведений об отладке, поэтому строк у
        // него нет; форма таблицы всё равно соблюдается — запись на чанк.
        lines: if unit.lines.is_empty() {
            Vec::new()
        } else {
            vec![unit.lines.clone()]
        },
    };
    let linked = link_components(
        &program,
        Some(registry),
        host_env.zone(),
        host_env.files(),
        host_env.random(),
        host_env.network(),
        host_env.background_jobs(),
        host_env.temp_storage(),
        host_env.message_sink(),
        bsl_bytecode::DynamicScope::ROOT,
    )?;
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout,
        stderr,
        env: Some(host_env),
        dynamic: Some(dynamic),
        dynamic_depth: &dynamic_depth,
        cancel_flag: None,
        file_promises: None,
    };
    let mut module_state = ModuleState::new(&program);
    drive_linked(
        &program,
        0,
        stack,
        &linked,
        &mut host,
        &mut module_state,
        None,
    )
}

/// Вызывает процедуру или функцию модуля ПО ИМЕНИ — узкая точка входа для
/// рантайма, которому надо позвать пользовательский код по строке, пришедшей
/// из данных (функция восстановления `ПрочитатьJSON`, обработчик события и
/// далее в том же духе). Машинерия под ней та же, что у изолированного
/// исполнения `Выполнить`/`Вычислить` (см. `run_dynamic_snippet`), только
/// без текстовой прослойки: чанк уже скомпилирован, компилировать и
/// кэшировать нечего.
///
/// Имя ищется регистронезависимо (через `to_uppercase`, как сравнивает
/// идентификаторы `bsl-sema`) по [`Program::function_names`], куда входят и
/// процедуры, и функции.
///
/// `stack` — стек значений вызывающего: из его первых слотов читается
/// блок модульных переменных и в них же он возвращается после вызова, поэтому запись в модульную переменную из вызванной процедуры
/// переживает вызов ровно так же, как при обычном `Call`.
///
/// Аргументы передаются ПО ЗНАЧЕНИЮ, даже для параметров без `Знач`:
/// алиасить нечего — вызов приходит не с места вызова в BSL, а изнутри
/// рантайма, и «переменной вызывающего» здесь не существует. Наблюдать то,
/// что вызванный код записал в свой параметр, позволяет второй элемент
/// возвращаемой пары — ФИНАЛЬНЫЕ значения слотов параметров, длиной
/// `n_params` и в порядке объявления (это будущий канал для `Отказ`).
/// Первый элемент — значение `Возврат`; у процедуры и у функции, дошедшей
/// до конца тела без `Возврат`, это `Неопределено`.
///
/// # Errors
///
/// - [`RtError::DynamicError`] — имени нет в модуле либо число аргументов
///   не совпало с числом параметров. И то, и другое приходит из
///   пользовательских данных, поэтому это перехватываемая `Попытка` ошибка,
///   а не паника. Аргументов обязано быть РОВНО `n_params`: пропустить
///   позицию, как это делает `Ф(1, , 3)` в BSL, отсюда нельзя — режим
///   аргумента живёт в инструкции `Call`, а этот вызов её не проходит.
/// - [`RtError::StackOverflow`] — превышена вложенность динамических
///   вызовов: вызов по имени — такой же вложенный `drive` на стеке Rust,
///   как `Выполнить`, и рекурсия через него не должна валить процесс мимо
///   `Попытка`.
/// - [`RtError::DynamicError`] — вызванный код содержит
///   `Выполнить`/`Вычислить`: компилятора фрагментов у этого входа нет,
///   он есть у [`call_module_function_with_registry_and_io`].
/// - Любая ошибка самого исполнения, не перехваченная внутри вызванного
///   кода, а также [`RtError::InvalidBytecode`], если `program`
///   рассогласована (имя функции есть, чанка под него нет) или `stack`
///   короче блока модульных переменных.
pub fn call_module_function(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let mut env = bsl_rt::HostEnv::process();
    let linked = link_components(
        program,
        None,
        env.zone(),
        env.files(),
        env.random(),
        env.network(),
        env.background_jobs(),
        env.temp_storage(),
        env.message_sink(),
        bsl_bytecode::DynamicScope::ROOT,
    )?;
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
        env: Some(&mut env),
        dynamic: None,
        dynamic_depth: &dynamic_depth,
        cancel_flag: None,
        file_promises: None,
    };
    call_module_function_with_host(program, stack, name, args, &linked, &mut host)
}

/// Вызывает функцию модуля с реестром компонентов и потоками текущего
/// host-состояния.
///
/// # Errors
///
/// Помимо ошибок [`call_module_function`] возвращает ошибку связывания,
/// если модулю недоступен требуемый компонент или его точная версия.
#[allow(clippy::too_many_arguments)]
pub fn call_module_function_with_registry_and_io<'a>(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
    registry: &bsl_rt::RuntimeRegistry,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: &'a mut dyn DynamicCompiler,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let linked = link_components(
        program,
        Some(registry),
        host_env.zone(),
        host_env.files(),
        host_env.random(),
        host_env.network(),
        host_env.background_jobs(),
        host_env.temp_storage(),
        host_env.message_sink(),
        bsl_bytecode::DynamicScope::ROOT,
    )?;
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout,
        stderr,
        env: Some(host_env),
        dynamic: Some(dynamic),
        dynamic_depth: &dynamic_depth,
        cancel_flag: None,
        file_promises: None,
    };
    call_module_function_with_host(program, stack, name, args, &linked, &mut host)
}

pub(super) fn call_module_function_with_host(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    // Блок модульных переменных начинается с нуля стека вызывающего.
    // Смещения здесь нет и быть не может: у фрагмента `Выполнить` стек
    // СВОЙ, и его блок начинается с нуля так же, как у верхнего уровня
    // в своём.
    let mut module_state = ModuleState::new(program);
    module_state.slots = (0..program.module_vars.len())
        .map(|i| reg_load(stack, i))
        .collect::<Result<_, _>>()?;
    let result =
        call_module_function_in_execution(program, name, args, linked, host, &mut module_state);
    for (i, value) in module_state.slots.into_iter().enumerate() {
        reg_store(stack, i, value)?;
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub(super) fn call_module_function_in_execution(
    program: &Program,
    name: &str,
    args: Vec<BslValue>,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let _depth = DynamicDepthGuard::enter(host.dynamic_depth)?;

    let upper = name.to_uppercase();
    let index = program
        .function_names
        .iter()
        .position(|n| n.to_uppercase() == upper)
        .ok_or_else(|| {
            RtError::DynamicError(format!(
                "Процедура или функция «{name}» в модуле не найдена"
            ))
        })?;
    // `function_names[i]` — это `chunks[i + 1]`: нулевой чанк занят
    // операторами верхнего уровня.
    let func_id = index + 1;
    let chunk = at(&program.chunks, func_id, "номер чанка вне таблицы функций")?;
    let n_params = chunk.n_params as usize;
    if args.len() != n_params {
        return Err(RtError::DynamicError(format!(
            "Неверное число аргументов при вызове «{name}»: передано {}, а параметров {n_params}",
            args.len()
        )));
    }

    // Кадр вызванной функции строится так же, как его строит `drive`:
    // параметры — слоты `0..n_params`, собственные регистры сразу за ними.
    // Алиасов на слоты вызывающего у этого кадра нет (`drive` заводит его с
    // пустым `Frame::param_aliases`), поэтому аргументы просто лежат
    // значениями в начале стека.
    let mut call_stack = args;
    push_own_registers(&mut call_stack, chunk);
    // ИЗВЕСТНАЯ ЦЕНА: вложенный `drive_linked` — отдельный прогон, и по
    // требованию изоляции прогонов (спецификация `bytecode-image`,
    // «Инлайн-кэши принадлежат прогону») он заводит СВЕЖИЕ ячейки на всю
    // программу и стартует с холодным кэшем на каждый вызов. Для
    // callback-плотного кода (функция восстановления `ПрочитатьJSON` зовёт
    // сюда на каждое значение) это аллокация, пропорциональная модулю, и
    // потеря мономорфности на каждом вызове — раньше ячейки жили в `Chunk`
    // общей программы и оставались тёплыми. Прогрев через разделение
    // ячеек с объемлющим прогоном требует поправки требования изоляции,
    // не локальной правки (см. риски в `docs/plans/bsl-vm-refactor.md`).
    let (value, final_stack) = drive_linked(
        program,
        func_id,
        call_stack,
        linked,
        host,
        module_state,
        None,
    )?;

    // Финальные значения слотов параметров: верхний кадр при возврате стек
    // не усекает (см. `do_return_with_value`), поэтому они всё ещё на
    // месте — в слотах `0..n_params`.
    let mut final_params = Vec::with_capacity(n_params);
    for i in 0..n_params {
        final_params.push(reg_load(&final_stack, i)?);
    }
    Ok((value, final_params))
}
