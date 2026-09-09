use super::runtime::SystemJobIds;
use super::{
    BackgroundJobConfig, JobMessageRoute, JobRuntime, JobRuntimeShared, WorkerJobService,
    random_uuid,
};
use bsl_rt::{JobErrorDto, JobId, SerializedValueGraph};
use std::sync::Arc;

/// Экспортная поверхность каталога для разрешения целей submit:
/// снимается один раз при создании runtime, чтобы каждый запуск задания
/// не разбирал текстовый образ заново.
pub(crate) struct TargetTable {
    modules: Vec<(String, Vec<TargetFunction>)>,
}

struct TargetFunction {
    name: String,
    chunk: u16,
    exported: bool,
    is_async: bool,
}

impl TargetTable {
    fn from_catalog(catalog: &bsl_bytecode::ConfigurationProgram) -> Self {
        Self {
            modules: catalog
                .modules
                .iter()
                .map(|module| {
                    let functions = module
                        .program
                        .function_names
                        .iter()
                        .enumerate()
                        .map(|(i, name)| TargetFunction {
                            name: name.clone(),
                            chunk: (i + 1) as u16,
                            exported: module.program.exported_functions[i],
                            is_async: module.program.chunks[i + 1].is_async,
                        })
                        .collect();
                    (module.name.clone(), functions)
                })
                .collect(),
        }
    }

    /// Разрешает «Модуль.Метод»: неглобальный общий модуль, экспортная
    /// не-async цель. Детали validation — за `JOB.EXECUTE.VALIDATION`.
    pub(super) fn resolve(&self, method_name: &str) -> Result<(u32, u16), String> {
        let Some((module_name, function_name)) = method_name.split_once('.') else {
            return Err(format!("цель «{method_name}» не в форме «Модуль.Метод»"));
        };
        let module_name = module_name.trim();
        let function_name = function_name.trim();
        let Some((module_index, (_, functions))) = self
            .modules
            .iter()
            .enumerate()
            .find(|(_, (name, _))| bsl_rt::folded_eq(name, module_name))
        else {
            return Err(format!("общий модуль «{module_name}» не найден"));
        };
        let Some(function) = functions
            .iter()
            .find(|function| bsl_rt::folded_eq(&function.name, function_name))
        else {
            return Err(format!(
                "в модуле «{module_name}» нет метода «{function_name}»"
            ));
        };
        if !function.exported {
            return Err(format!(
                "метод «{module_name}.{function_name}» не экспортирован"
            ));
        }
        if function.is_async {
            // НЕ ИЗМЕРЕНО(JOB.ASYNC.TARGET): ИЗМЕРЕНО на файловой базе
            // (2026-08-27), что «Асинх» в серверном общем модуле ломает
            // инициализацию ВСЕГО модуля при первом обращении, то есть
            // легальной async-цели у платформы нет; клиент-серверное
            // подтверждение и критерий завершения — за следующей сессией.
            // Здесь отказ синхронный: у платформы падает не submit, а
            // любой вызов отравленного модуля — расхождение осознанное.
            return Err(
                "асинхронная цель фонового задания не поддержана до замера JOB.ASYNC.TARGET"
                    .to_string(),
            );
        }
        Ok((module_index as u32, function.chunk))
    }
}

/// `Send`-рецепт worker: текстовый образ каталога, статические
/// `LibraryDescriptor` пользовательских библиотек и символы
/// препроцессора. Скрытого второго формата нет — worker разбирает тот же
/// публичный `BytecodeImage::Configuration`, что пишет `--emit-bytecode`,
/// один раз, и разделяет разобранные программы между своими сеансами.
#[derive(Clone)]
pub(crate) struct WorkerRecipe {
    pub image_text: Arc<str>,
    /// Библиотеки, добавленные `register_library` родительского движка:
    /// без них цель с пользовательским типом не слинкуется в worker.
    pub libraries: Vec<bsl_rt::LibraryDescriptor>,
    /// Символы условной компиляции — их видит динамический код задания.
    pub symbols: bsl_syntax::PreprocSymbols,
}

/// Движок worker строится из того же публичного образа, которым ходит
/// `--emit-bytecode`: стандартный состав компонентов плюс
/// пользовательские библиотеки родительского движка и его символы
/// препроцессора — цель с пользовательским типом линкуется, а
/// динамический код задания видит те же `#Если`.
pub(super) fn build_worker_engine(recipe: &WorkerRecipe) -> Result<crate::Engine, String> {
    let image = bsl_bytecode::parse_image(&recipe.image_text).map_err(|e| e.to_string())?;
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry: _ } = image else {
        return Err("рецепт worker обязан быть конфигурацией".to_string());
    };
    let mut builder = crate::Engine::builder();
    for library in &recipe.libraries {
        builder = builder.register_library(*library);
    }
    builder
        .preproc_symbols_all(recipe.symbols)
        .configuration_image(catalog, false)
        .build()
        .map_err(|e| e.to_string())
}

/// Готовит сеанс задания без компилятора: entry-программа собирается
/// руками — аргументы приходят константами чанка, вызов идёт обычным
/// `CallImported` по числовому манифесту. Возвращает изолированный сеанс,
/// модуль entry и pollable-прогон с постоянным квантованием.
// НЕ ИЗМЕРЕНО(JOB.MODULE.INIT): ИЗМЕРЕНО на файловой базе (2026-08-27),
// что у платформенных серверных общих модулей тел НЕТ вовсе — модуль с
// телом грузится молча и падает «Ошибка инициализации модуля» при первом
// обращении (проверено и с `Перем`, и без него, и с наблюдаемым без
// `Сообщить`); клиент-серверное подтверждение остаётся. Тела модулей
// open-bsl — осознанное расширение, их инициализация ленивая на сеанс
// (та же `ModuleInitState`, что у обычного прогона).
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_job(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    id: JobId,
    target: (u32, u16),
    params: &SerializedValueGraph,
    caller_token: Option<[u8; 16]>,
    profile_index: u32,
) -> Result<(crate::State, crate::Module, bsl_vm::ProgramExecution), JobErrorDto> {
    let catalog = engine
        .catalog()
        .expect("worker строится только с каталогом");
    let callee = catalog
        .modules
        .get(target.0 as usize)
        .and_then(|module| module.program.chunks.get(target.1 as usize))
        .ok_or_else(|| JobErrorDto::from_text("цель задания вне каталога"))?;
    let n_params = callee.n_params as usize;
    let argc = params.root_count();
    let mut modes = Vec::with_capacity(n_params);
    for position in 0..n_params {
        if position < argc {
            modes.push(bsl_bytecode::ArgMode::Value);
        } else if callee.param_has_default.get(position) == Some(&true) {
            // Хвост без аргументов берёт умолчания; точная политика
            // платформы — за замером JOB.EXECUTE.DEFAULTS.
            modes.push(bsl_bytecode::ArgMode::Default);
        } else {
            return Err(JobErrorDto::from_text(format!(
                "цели передано {argc} аргументов, а обязательных параметров {n_params}"
            )));
        }
    }
    if argc > n_params {
        return Err(JobErrorDto::from_text(format!(
            "цели передано {argc} аргументов при {n_params} параметрах"
        )));
    }
    let base_program = &catalog.modules[0].program;
    // Аргументы материализуются ЗАРАНЕЕ интернером, который станет
    // таблицами entry-программы: `NameId` внутри значений согласованы с
    // `RuntimeShapes::seeded` прогона. Сами значения едут константами
    // чанка и раскладываются по регистрам обычными `LoadConst`.
    let mut shapes = bsl_rt::RuntimeShapes::seeded(
        base_program.names.clone(),
        base_program.shapes.clone(),
        Some(engine.registry()),
    );
    let arguments = params
        .materialize(&mut shapes)
        .map_err(|error| JobErrorDto::from_text(error.to_string()))?;
    let names = shapes.names.into_names();
    let shape_table = shapes.shapes.into_shapes();

    let mut instrs = Vec::with_capacity(argc + 2);
    for position in 0..argc {
        instrs.push(bsl_bytecode::Instr::LoadConst {
            dst: position as u8,
            k: position as u16,
        });
    }
    instrs.push(bsl_bytecode::Instr::CallImported {
        link_slot: 0,
        base: 0,
        arg_modes: 0,
        ret: n_params as u8,
    });
    instrs.push(bsl_bytecode::Instr::Return { src: None });
    let regs = (n_params + 1).max(1) as u8;
    let mut chunk = bsl_bytecode::Chunk::new();
    chunk.instrs = instrs;
    // Таблица констант здесь — ТРАНСПОРТ аргументов, а не литеральный
    // пул: значения задания материализуются в объекты и типы, которых
    // текстовый формат не представляет. Программа собирается в памяти и
    // не печатается — сериализуется каталог с `entry: None`, — поэтому
    // непредставимость ей ничем не грозит.
    chunk.consts = arguments
        .into_iter()
        .map(bsl_bytecode::BytecodeConst::transient)
        .collect();
    chunk.call_arg_modes = vec![modes];
    chunk.n_locals = n_params as u8;
    chunk.n_regs = regs;
    let mut entry = bsl_bytecode::Program {
        requirements: base_program.requirements.clone(),
        chunks: vec![chunk],
        names,
        shapes: shape_table,
        top_level_locals: (0..n_params).map(|i| format!("Параметр{i}")).collect(),
        function_names: Vec::new(),
        exported_functions: Vec::new(),
        module_vars: Vec::new(),
        exported_module_vars: Vec::new(),
        links: vec![bsl_bytecode::LinkEntry::Function {
            module: bsl_bytecode::ModuleId::new(target.0),
            func: target.1,
        }],
        // Entry-программа воркера собирается в рантайме, отлаживать её
        // отдельно от вызывающей программы нечем и незачем.
        lines: Vec::new(),
    };
    bsl_bytecode::image::finalize(&mut entry);
    let module = engine
        .load_entry(bsl_bytecode::EntryProgram {
            id: bsl_bytecode::EntryId::new(0),
            program: entry,
        })
        .map_err(|error| JobErrorDto::from_text(error.to_string()))?;

    // Сеанс задания строит выбранный host-профиль: системный (0) — это
    // process-default сервисы worker-движка, зарегистрированный — его
    // фабрика. Ошибка фабрики без паники завершает ТОЛЬКО этот job как
    // `Failed` с кодом `HostProfileUnavailable`; worker остаётся жив.
    let state_builder = match profile_index.checked_sub(1) {
        None => engine.state_builder(),
        Some(index) => {
            let Some(factory) = shared.profiles.get(index as usize) else {
                // Защитная ветка: выбор профиля валидируется на
                // `StateBuilder::host_profile`, сюда попадает только
                // рассинхронизация таблиц.
                return Err(JobErrorDto::from_text(
                    "host-профиль недоступен: не зарегистрирован в этом движке",
                ));
            };
            factory.configure(engine.state_builder()).map_err(|text| {
                JobErrorDto::from_text(format!("host-профиль недоступен: {text}"))
            })?
        }
    };
    let mut state = state_builder.build();
    // stdout задания — сток: `Сообщить` идёт через sink сообщений ниже, а
    // прямых писателей stdout в фоновом сеансе не остаётся.
    state.host.stdout = Box::new(std::io::sink());
    // `Сообщить` задания кладёт владеющий DTO в FIFO-историю записи (её
    // читает `ПолучитьСообщенияПользователю`), затем — внешнему sink
    // представления, если он зарегистрирован.
    state
        .host
        .env
        .set_message_sink(std::rc::Rc::new(JobMessageRoute {
            shared: Arc::clone(shared),
            id,
        }));
    // Сеанс задания получает СВОЁ временное хранилище со ссылкой на
    // вызывателя: запись по его адресу — staging до terminal, под
    // per-job и глобальным кредитами. Mailbox сеанса регистрируется в
    // ОБЩЕМ hub движка: дочернее задание публикует write-set сюда — и
    // только сюда, транзитивного повышения capability нет.
    let session_token = random_uuid();
    {
        let session = std::rc::Rc::new(std::cell::RefCell::new(match caller_token {
            Some(caller) => bsl_rt::TempStorageSession::for_job(
                session_token,
                caller,
                state.host.env.random(),
                bsl_rt::StagingBudget::new(
                    shared.config.max_staged_temp_bytes_per_job,
                    Arc::clone(&shared.staging_global),
                ),
            ),
            None => bsl_rt::TempStorageSession::new(session_token, state.host.env.random()),
        }));
        shared
            .temp_hub
            .register(session_token, session.borrow().mailbox());
        state.host.env.set_temp_storage(session);
    }
    // Вложенные задания идут в ОБЩИЙ реестр родительского runtime, а не в
    // пул воркерного движка: сервис сеанса подменяется worker-обёрткой с
    // helping-ожиданием. Профиль наследуется как есть — повысить
    // возможности дочернему заданию негде.
    state
        .host
        .env
        .set_background_jobs(std::rc::Rc::new(WorkerJobService {
            shared: Arc::clone(shared),
            engine: engine.clone(),
            session_token,
            profile_index,
        }));
    let mut vm = bsl_vm::ProgramExecution::start_with_registry_and_scheduler(
        &module.program,
        engine.registry(),
        &state.host.env,
        state.scheduler,
    )
    .map_err(|error| JobErrorDto::from_text(error.to_string()))?;
    let catalog = engine
        .catalog()
        .expect("worker строится только с каталогом");
    vm.attach_catalog(catalog);
    // Фоновый прогон всегда квантуется: бюджетный poll возвращает
    // управление драйверу, и worker чередует резидентов.
    vm.set_always_scheduled(true);
    Ok((state, module, vm))
}

/// Разрешает цель `Модуль.Метод` по каталогу: неглобальный общий модуль,
/// экспортная не-async функция или процедура — тестовая поверхность;
/// рабочий путь идёт через `TargetTable::resolve` (ИЗМЕРЕНО,
/// `JOB.EXECUTE.VALIDATION`: синхронная валидация цели).
#[cfg(test)]
pub(crate) fn resolve_target(
    catalog: &bsl_bytecode::ConfigurationProgram,
    method_name: &str,
) -> Result<(u32, u16), String> {
    let Some((module_name, function_name)) = method_name.split_once('.') else {
        return Err(format!("цель «{method_name}» не в форме «Модуль.Метод»"));
    };
    let Some((module_id, module)) = catalog.find(module_name.trim()) else {
        return Err(format!("общий модуль «{module_name}» не найден"));
    };
    let program = &module.program;
    let function_name = function_name.trim();
    let Some(index) = program
        .function_names
        .iter()
        .position(|name| bsl_rt::folded_eq(name, function_name))
    else {
        return Err(format!(
            "в модуле «{module_name}» нет метода «{function_name}»"
        ));
    };
    if !program.exported_functions[index] {
        return Err(format!(
            "метод «{module_name}.{function_name}» не экспортирован"
        ));
    }
    let chunk = (index + 1) as u16;
    if program.chunks[chunk as usize].is_async {
        return Err(
            "асинхронная цель фонового задания не поддержана: у платформы \
             «Асинх» в общем модуле ломает инициализацию всего модуля"
                .to_string(),
        );
    }
    Ok((module_id.index() as u32, chunk))
}

/// Runtime для движка с каталогом: рецепт — публичный текстовый образ
/// каталога без entry.
pub(crate) fn runtime_for_engine(
    engine: &crate::Engine,
    config: BackgroundJobConfig,
) -> Result<JobRuntime, String> {
    config.validate()?;
    let catalog = engine
        .catalog()
        .ok_or("фоновые задания требуют движка с каталогом общих модулей")?;
    let image = bsl_bytecode::BytecodeImage::Configuration {
        catalog: catalog.clone(),
        entry: None,
    };
    let text = bsl_bytecode::write_image(&image, None).map_err(|e| e.to_string())?;
    Ok(JobRuntime::new(
        config,
        WorkerRecipe {
            image_text: Arc::from(text),
            libraries: engine.extra_libraries().to_vec(),
            symbols: engine.preproc_symbols(),
        },
        TargetTable::from_catalog(catalog),
        Arc::new(SystemJobIds),
        Arc::clone(engine.temp_hub()),
        engine.job_profiles(),
        engine.job_message_display(),
    ))
}
