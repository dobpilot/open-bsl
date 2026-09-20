use super::{
    Frame, HostIo, LinkedComponents, ModuleState, at, link_components, reg_load, reg_store,
};
use bsl_bytecode::{Instr, Program};
use bsl_rt::{BslValue, RtError};

/// Предел вложенности `Выполнить`/`Вычислить`: обычное исполнение считает
/// динамические кадры, блокирующее отладочное — вложенные входы HostIo.
/// Сохраняется прежняя граница, пока замер не определил предел платформы.
// НЕ ИЗМЕРЕНО(EXEC.DYNAMIC_DEPTH) — сколько уровней допускает платформа;
// замер даёт только нижнюю границу (40 уровней обязаны работать).
const MAX_DYNAMIC_DEPTH: usize = 64;

/// Вход в очередной уровень динамического кода; выход — в `Drop`, чтобы
/// счётчик не съезжал ни на одном из путей ошибки.
///
/// Счётчик — не потоковый, а поле [`HostIo`] прогона: две сессии в одном
/// потоке (например, вложенный `Engine` за обратным вызовом функции) не
/// делят вложенность `Выполнить`. Вложенный `poll` и обратный вызов
/// функции модуля строят заимствованный `HostIo` с тем же `dynamic_depth`
/// родителя, так что уровень протаскивается через прогон, а не через поток.
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

/// Код, таблицы и кэши живут до выхода последнего использующего их кадра,
/// в том числе дочерней async-задачи после возврата фрагмента.
pub(super) struct DynamicImage {
    pub(super) program: Program,
    pub(super) tables: std::rc::Rc<super::linking::LinkedTables>,
    pub(super) shapes: std::cell::RefCell<bsl_rt::RuntimeShapes>,
    pub(super) caches: super::RunCaches,
}

#[allow(clippy::too_many_arguments)]
fn prepare_dynamic_snippet(
    code: &str,
    is_eval: bool,
    program: &Program,
    scope_locals: &[String],
    scope_id: usize,
    frame: &Frame,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    catalog: Option<&bsl_bytecode::ConfigurationProgram>,
) -> Result<std::rc::Rc<DynamicImage>, RtError> {
    // Вход в host-компилятор тоже учитывается в глубине: он может
    // обратиться к исполнителю через обратный вызов.
    let _depth = DynamicDepthGuard::enter(host.dynamic_depth)?;

    let scope_module = if frame.module == super::ROOT_MODULE {
        linked.scope_module
    } else {
        Some(bsl_bytecode::ModuleId::new(frame.module))
    };

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
            module: scope_module,
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
        imports: &program.imports,
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
    // Статические функции сохраняют свои номера форм. Формы фрагмента
    // добавляются после них; его операнды проверяются до перенумерации,
    // чтобы ошибочный индекс не стал допустимым за счёт чужой таблицы.
    let mut shapes = program.shapes.clone();
    for instruction in &mut chunks[0].instrs {
        if let Instr::NewStructure { shape, .. } = instruction {
            if usize::from(*shape) >= compiled.shapes.len() {
                return Err(RtError::InvalidBytecode(
                    "номер формы структуры вне таблицы форм фрагмента",
                ));
            }
            *shape = shapes
                .len()
                .checked_add(usize::from(*shape))
                .and_then(|index| u16::try_from(index).ok())
                .ok_or(RtError::InvalidBytecode(
                    "объединённый номер формы структуры не помещается в u16",
                ))?;
        }
    }
    shapes.extend(compiled.shapes.iter().cloned());
    let mut links = program.links.clone();
    remap_dynamic_links(&mut chunks[0], links.len(), compiled.links.len())?;
    for link in &compiled.links {
        let target = match link {
            bsl_bytecode::LinkEntry::Function { module, .. }
            | bsl_bytecode::LinkEntry::Variable { module, .. } => module,
            bsl_bytecode::LinkEntry::ObjectMethod { .. } => continue,
        };
        if !program
            .imports
            .iter()
            .any(|import| import.module == *target)
        {
            return Err(RtError::InvalidBytecode(
                "фрагмент ссылается на неимпортированный модуль",
            ));
        }
    }
    links.extend_from_slice(&compiled.links);
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
        shapes,
        top_level_locals: Vec::new(),
        function_names: program.function_names.clone(),
        exported_functions: program.exported_functions.clone(),
        module_vars: program.module_vars.clone(),
        exported_module_vars: program.exported_module_vars.clone(),
        links,
        imports: program.imports.clone(),
        // Таблица, собранная выше: чужие записи в координатах файла,
        // нулевая — в координатах собственного текста фрагмента. Раньше
        // здесь стоял пустой вектор, и вся работа выше пропадала.
        lines,
    };

    if compiled.links.iter().any(|link| {
        matches!(
            link,
            bsl_bytecode::LinkEntry::Function { .. } | bsl_bytecode::LinkEntry::Variable { .. }
        )
    }) {
        let catalog = catalog.ok_or(RtError::InvalidBytecode("импортный фрагмент без каталога"))?;
        bsl_bytecode::image::verify_linked_program(&snippet_program, catalog, scope_module)?;
    }
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
    )?
    .with_scope_module(scope_module);
    Ok(std::rc::Rc::new(DynamicImage {
        shapes: std::cell::RefCell::new(super::drive_prologue(&snippet_program, &snippet_linked)),
        caches: super::RunCaches::for_program(&snippet_program),
        tables: snippet_linked.tables,
        program: snippet_program,
    }))
}

fn remap_dynamic_links(
    chunk: &mut bsl_bytecode::Chunk,
    offset: usize,
    count: usize,
) -> Result<(), RtError> {
    if offset
        .checked_add(count)
        .is_none_or(|length| length > u16::MAX as usize)
    {
        return Err(RtError::InvalidBytecode(
            "слишком много связей динамического фрагмента",
        ));
    }
    let remap = |slot: &mut u16| -> Result<(), RtError> {
        if usize::from(*slot) >= count {
            return Err(RtError::InvalidBytecode("связь вне таблицы фрагмента"));
        }
        *slot = (offset + usize::from(*slot)) as u16;
        Ok(())
    };
    for instruction in &mut chunk.instrs {
        match instruction {
            Instr::CallImported { link_slot, .. }
            | Instr::GetImportedVar { link_slot, .. }
            | Instr::SetImportedVar { link_slot, .. } => remap(link_slot)?,
            _ => {}
        }
    }
    for mode in chunk.call_arg_modes.iter_mut().flatten() {
        if let bsl_bytecode::ArgMode::ByRefImportedVar(slot) = mode {
            remap(slot)?;
        }
    }
    Ok(())
}

/// Фрагмент использует стек, каталог и планировщик вызывающего запуска.
/// Существующие локали — ссылки на ячейки вызывающего; новые не переживают
/// возврат. Ошибка не требует копирования локалей из отдельного execution.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_dynamic_frame(
    code: &str,
    is_eval: bool,
    dst: u8,
    program: &Program,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    catalog: Option<&bsl_bytecode::ConfigurationProgram>,
) -> Result<(), RtError> {
    let depth = frames
        .iter()
        .filter(|f| f.code.is_some() && f.func_id == 0)
        .count();
    if host.dynamic_depth.get() + depth >= MAX_DYNAMIC_DEPTH
        || frames.len() >= super::MAX_CALL_DEPTH
    {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая вложенность Выполнить/Вычислить",
        });
    }
    let frame = frames
        .last()
        .ok_or(RtError::InvalidBytecode("динамический вызов без кадра"))?;
    let scope_locals = &program.chunks[frame.func_id].local_names;
    let image = prepare_dynamic_snippet(
        code,
        is_eval,
        program,
        scope_locals,
        frame.func_id,
        frame,
        linked,
        host,
        catalog,
    )?;
    let extra = (image.program.chunks[0].n_regs as usize)
        .checked_sub(scope_locals.len())
        .ok_or(RtError::InvalidBytecode(
            "фрагмент потерял существующие локальные слоты",
        ))?;
    let param_aliases = (0..scope_locals.len())
        .map(|slot| super::ParamSlot {
            idx: frame.reg_index(slot as u8),
            provided: true,
        })
        .collect();
    let module = frame.module;
    let call_start = stack.len();
    stack.resize(call_start + extra, BslValue::Undefined);
    frames.last_mut().unwrap().pc += 1;
    frames.push(Frame {
        module,
        code: Some(image),
        func_id: 0,
        pc: 0,
        param_aliases,
        own_base: call_start,
        call_start,
        return_reg: dst,
        module_copybacks: Vec::new(),
        numeric_for_state: None,
    });
    Ok(())
}

/// Отладочное вычисление остаётся отдельным блокирующим запуском:
/// остановленный стек копируется, затем существующие локали переносятся обратно.
#[allow(clippy::too_many_arguments)]
fn start_dynamic_snippet(
    code: &str,
    is_eval: bool,
    program: &Program,
    scope_locals: &[String],
    scope_id: usize,
    stack: &mut [BslValue],
    frame: &Frame,
    module_aliases: &[(usize, u32, usize)],
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
    async_state: &mut super::AsyncState,
    root_program: &Program,
    root_linked: &LinkedComponents<'_>,
    catalog: Option<&super::CatalogContext<'_>>,
) -> Result<DynamicExecution, RtError> {
    let image = prepare_dynamic_snippet(
        code,
        is_eval,
        program,
        scope_locals,
        scope_id,
        frame,
        linked,
        host,
        catalog.map(|ctx| ctx.catalog),
    )?;
    let old_count = scope_locals.len();
    let mut snippet_stack = (0..old_count)
        .map(|i| reg_load(stack, frame.reg_index(i as u8)))
        .collect::<Result<Vec<_>, _>>()?;
    snippet_stack.resize(image.program.chunks[0].n_regs as usize, BslValue::Undefined);
    let mut execution = super::ProgramExecution::new_linked(
        root_program,
        0,
        snippet_stack,
        root_linked,
        module_state.take_for_execution(),
        super::SchedulerConfig {
            safe_points_per_quantum: async_state.scheduler_quantum(),
        },
    );
    // Два имени параметров могут обозначать одну ячейку вызывающего.
    // Во фрагмент переносится эта связь, а не две независимые копии.
    let mut local_indices = Vec::with_capacity(old_count);
    let mut source_indices = Vec::with_capacity(old_count);
    let task = execution.async_state.tasks[0]
        .as_mut()
        .expect("создан корневой кадр фрагмента");
    let root_frame = &mut task.frames[0];
    root_frame.module = frame.module;
    root_frame.code = Some(image.clone());
    for index in 0..old_count {
        let source = frame.reg_index(index as u8);
        let canonical = source_indices
            .iter()
            .position(|existing| *existing == source)
            .unwrap_or(index);
        source_indices.push(source);
        let module_slot = module_aliases
            .iter()
            .find_map(|&(register, owner, slot)| (register == source).then_some((owner, slot)));
        // Связь со своим модулем восстанавливается из ModuleState, который
        // мог измениться дочерней задачей уже после возврата корня.
        local_indices.push(module_slot.is_none().then_some(canonical));
        root_frame.param_aliases.push(super::ParamSlot {
            idx: canonical,
            provided: true,
        });
        if canonical == index
            && let Some((owner, slot)) = module_slot
        {
            root_frame.module_copybacks.push((canonical, owner, slot));
        }
    }
    root_frame.own_base = old_count;
    execution.cancel_flag = host.cancel_flag.clone();
    if let Some(catalog) = catalog {
        execution.catalog_caches = catalog
            .catalog
            .modules
            .iter()
            .map(|module| super::RunCaches::for_program(&module.program))
            .collect();
    }
    Ok(DynamicExecution {
        execution,
        local_indices,
    })
}

/// Состояние блокирующего отладочного вычисления.
struct DynamicExecution {
    execution: super::ProgramExecution,
    /// None — ссылка на свой модуль: её значение уже восстановлено из
    /// актуального ModuleState и не должно заменяться снимком корня.
    local_indices: Vec<Option<usize>>,
}

impl DynamicExecution {
    #[allow(clippy::too_many_arguments)]
    fn poll(
        &mut self,
        linked: &LinkedComponents<'_>,
        host: &mut HostIo<'_, '_>,
        root_program: &Program,
        catalog: Option<&super::CatalogContext<'_>>,
        host_slice: usize,
        quanta_budget: Option<usize>,
        force_scheduled: bool,
    ) -> Result<super::ProgramPoll, RtError> {
        let _depth = DynamicDepthGuard::enter(host.dynamic_depth)?;
        let mut snippet_host = HostIo {
            stdout: &mut *host.stdout,
            stderr: &mut *host.stderr,
            env: host.env.as_deref_mut(),
            dynamic: host.dynamic.as_deref_mut(),
            dynamic_depth: host.dynamic_depth,
            cancel_flag: host.cancel_flag.clone(),
            file_promises: None,
        };
        self.execution.force_scheduled = force_scheduled;
        self.execution.poll_linked(
            root_program,
            linked,
            catalog,
            &mut snippet_host,
            host_slice,
            quanta_budget,
        )
    }

    fn copy_locals(
        &self,
        final_stack: &[BslValue],
        stack: &mut [BslValue],
        frame: &Frame,
    ) -> Result<(), RtError> {
        // Обратно переносятся ТОЛЬКО уже существовавшие слоты: их номера
        // совпадают с теми, что использует окружающий скомпилированный код.
        // Имена, объявленные самим фрагментом, получили слоты ЗА `old_count` и
        // никуда не переносятся — расширить статически размеченный кадр нечем.
        //
        // ИЗМЕРЕНО на 8.3.27: платформа ведёт себя ТАК ЖЕ — имя, впервые
        // созданное внутри `Выполнить`, вызов не переживает. Выбор оказался
        // верным, кадр с именной таблицей не нужен.
        //
        for (i, source) in self.local_indices.iter().enumerate() {
            if let Some(source) = source {
                let d = frame.reg_index(i as u8);
                reg_store(stack, d, reg_load(final_stack, *source)?)?;
            }
        }
        Ok(())
    }
}

/// Блокирующий вход для отладочного вычисления в остановленном кадре.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_dynamic_snippet(
    code: &str,
    is_eval: bool,
    program: &Program,
    scope_locals: &[String],
    scope_id: usize,
    stack: &mut [BslValue],
    frame: &Frame,
    module_aliases: &[(usize, u32, usize)],
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
    async_state: &mut super::AsyncState,
    root_program: &Program,
    root_linked: &LinkedComponents<'_>,
    catalog: Option<&super::CatalogContext<'_>>,
    session_modules: &mut super::SessionModules,
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
) -> Result<BslValue, RtError> {
    for &(index, owner, slot) in module_aliases {
        let state = if owner == super::ROOT_MODULE {
            &mut *module_state
        } else {
            &mut session_modules
                .instances
                .get_mut(owner as usize)
                .ok_or(RtError::InvalidBytecode("владелец ссылки вне каталога"))?
                .state
        };
        reg_store(&mut state.slots, slot, reg_load(stack, index)?)?;
    }
    let mut pending = start_dynamic_snippet(
        code,
        is_eval,
        program,
        scope_locals,
        scope_id,
        stack,
        frame,
        module_aliases,
        linked,
        host,
        module_state,
        async_state,
        root_program,
        root_linked,
        catalog,
    )?;
    std::mem::swap(&mut pending.execution.session_modules, session_modules);
    std::mem::swap(&mut pending.execution.runtime_shapes, runtime_shapes);
    let task = pending.execution.async_state.tasks[0]
        .take()
        .expect("корень evaluate");
    std::mem::swap(&mut pending.execution.async_state, async_state);
    let mut paused_ready = std::mem::take(&mut pending.execution.async_state.ready);
    let root_id = pending.execution.async_state.insert_task(task);
    pending.execution.debug_task_floor = Some(root_id);
    pending
        .execution
        .async_state
        .ready
        .push_back(super::ReadyEvent::Task(root_id));
    let result = loop {
        match pending.poll(
            root_linked,
            host,
            root_program,
            catalog,
            usize::MAX,
            None,
            false,
        ) {
            Ok(super::ProgramPoll::Complete(value, stack)) => break Ok((value, stack)),
            Ok(super::ProgramPoll::Runnable | super::ProgramPoll::Waiting) => {}
            Err(error) => break Err(error),
        }
    };
    let failed_stack = result
        .is_err()
        .then(|| pending.execution.take_root_stack_after_error())
        .flatten();
    pending.execution.async_state.discard_debug_task(root_id);
    paused_ready.append(&mut pending.execution.async_state.ready);
    pending.execution.async_state.ready = paused_ready;
    std::mem::swap(&mut pending.execution.async_state, async_state);
    std::mem::swap(&mut pending.execution.session_modules, session_modules);
    std::mem::swap(&mut pending.execution.runtime_shapes, runtime_shapes);
    module_state.slots = std::mem::take(&mut pending.execution.module_state.slots);
    for &(index, owner, slot) in module_aliases {
        let state = if owner == super::ROOT_MODULE {
            &*module_state
        } else {
            &session_modules
                .instances
                .get(owner as usize)
                .ok_or(RtError::InvalidBytecode("владелец ссылки вне каталога"))?
                .state
        };
        reg_store(stack, index, reg_load(&state.slots, slot)?)?;
    }
    if let Some(final_stack) = failed_stack {
        pending.copy_locals(&final_stack, stack, frame)?;
    }
    let (value, final_stack) = result?;
    pending.copy_locals(&final_stack, stack, frame)?;
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

    #[test]
    fn dynamic_link_remapping_checks_each_instruction_and_argument_before_offsetting() {
        for instruction in [
            Instr::GetImportedVar {
                dst: 0,
                link_slot: 0,
            },
            Instr::SetImportedVar {
                src: 0,
                link_slot: 0,
            },
            Instr::CallImported {
                base: 0,
                ret: 0,
                link_slot: 0,
                arg_modes: 0,
            },
        ] {
            let mut chunk = bsl_bytecode::Chunk::new();
            chunk.instrs.push(instruction);
            chunk
                .call_arg_modes
                .push(vec![bsl_bytecode::ArgMode::ByRefImportedVar(0)]);
            assert!(remap_dynamic_links(&mut chunk.clone(), 10, 0).is_err());
            remap_dynamic_links(&mut chunk, 10, 1).unwrap();
            assert_eq!(
                chunk.call_arg_modes[0],
                [bsl_bytecode::ArgMode::ByRefImportedVar(10)]
            );
            match chunk.instrs[0] {
                Instr::GetImportedVar { link_slot, .. }
                | Instr::SetImportedVar { link_slot, .. }
                | Instr::CallImported { link_slot, .. } => assert_eq!(link_slot, 10),
                _ => unreachable!(),
            }
        }
        let mut chunk = bsl_bytecode::Chunk::new();
        chunk
            .call_arg_modes
            .push(vec![bsl_bytecode::ArgMode::ByRefImportedVar(1)]);
        assert!(remap_dynamic_links(&mut chunk, 10, 1).is_err());
        assert!(remap_dynamic_links(&mut chunk, u16::MAX as usize, 1).is_err());
    }

    #[test]
    fn dynamic_images_survive_the_fragment_but_not_their_last_task() {
        let program = crate::tests::compile_module(
            "Асинх Функция Позже() Ждать 0; Ждать 0; Возврат 17; КонецФункции
             Возврат Вычислить(\"Позже()\");",
        );
        let mut builder = bsl_rt::RuntimeBuilder::new();
        builder.register(bsl_rt::core_library());
        let registry = builder.build().unwrap();
        for cancel in [false, true] {
            let mut env = bsl_rt::HostEnv::process();
            let mut execution =
                crate::ProgramExecution::start_with_registry(&program, &registry, &env).unwrap();
            execution.merge_linear = false;
            let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            execution.set_cancel_flag(flag.clone());
            execution.force_scheduled = true;
            let mut dynamic = crate::tests::TestDynamic::bare();
            let linked = link_components(
                &program,
                Some(&registry),
                env.zone(),
                env.files(),
                env.random(),
                env.network(),
                env.background_jobs(),
                env.temp_storage(),
                env.message_sink(),
                0,
            )
            .unwrap();
            let depth = std::cell::Cell::new(0);
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut host = HostIo {
                stdout: &mut stdout,
                stderr: &mut stderr,
                env: Some(&mut env),
                dynamic: Some(&mut dynamic),
                dynamic_depth: &depth,
                cancel_flag: Some(flag.clone()),
                file_promises: None,
            };
            let mut images = Vec::new();
            let mut survived_root = false;
            let mut finished = false;
            for _ in 0..200 {
                let poll = execution.poll_linked(&program, &linked, None, &mut host, 1, Some(1));
                for task in execution.async_state.tasks.iter().flatten() {
                    for frame in &task.frames {
                        if let Some(code) = &frame.code {
                            let weak = std::rc::Rc::downgrade(code);
                            if !images.iter().any(|old| std::rc::Weak::ptr_eq(old, &weak)) {
                                images.push(weak);
                            }
                        }
                    }
                }
                if execution.root_result.is_some()
                    && images.iter().any(|weak| weak.strong_count() > 0)
                {
                    survived_root = true;
                    if cancel {
                        flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                match poll {
                    Ok(crate::ProgramPoll::Runnable) => {}
                    Ok(crate::ProgramPoll::Complete(_, _)) => {
                        assert!(!cancel);
                        assert!(images.iter().all(|weak| weak.upgrade().is_none()));
                        finished = true;
                        break;
                    }
                    Err(RtError::Canceled) => {
                        assert!(cancel);
                        finished = true;
                        break;
                    }
                    other => panic!("неожиданный результат: {other:?}"),
                }
            }
            assert!(
                finished && survived_root,
                "cancel={cancel}, finished={finished}, survived_root={survived_root}, images={}",
                images.len()
            );
            assert_eq!(images.len(), 1);
            drop(execution);
            assert!(images.iter().all(|weak| weak.upgrade().is_none()));
        }
    }

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
