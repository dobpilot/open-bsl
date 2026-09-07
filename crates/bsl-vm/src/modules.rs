use super::{Frame, LinkedComponents, MAX_CALL_DEPTH, RunCaches, at, push_own_registers};
use bsl_bytecode::Program;
use bsl_rt::{BslValue, RtError};

pub(super) struct ModuleState {
    pub(super) slots: Vec<BslValue>,
}

impl ModuleState {
    pub(super) fn new(program: &Program) -> Self {
        Self {
            slots: vec![BslValue::Undefined; program.module_vars.len()],
        }
    }
}

/// Номер модуля кадра. `ROOT_MODULE` — программа, переданная в poll
/// (одиночная либо entry конфигурации); остальные номера — позиции в
/// каталоге. Сравнение с sentinel дешевле `Option<u32>` в горячем цикле.
pub const ROOT_MODULE: u32 = u32::MAX;

/// Состояние инициализации общего модуля в ОДНОМ сеансе. Политика ленивая:
/// тело модуля исполняется при первом обращении к его символу; момент и
/// повторная попытка после ошибки уточняются замером `JOB.MODULE.INIT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModuleInitState {
    NotStarted,
    Initializing,
    Ready,
    Failed,
}

/// Экземпляр общего модуля в сеансе: его переменные и стадия
/// инициализации. Между сеансами и потоками не разделяется.
pub(super) struct ModuleInstance {
    pub(super) state: ModuleState,
    pub(super) init: ModuleInitState,
}

/// Сессионные экземпляры всех модулей каталога, индекс — `ModuleId`.
/// У одиночной программы пуст.
#[derive(Default)]
pub struct SessionModules {
    pub(super) instances: Vec<ModuleInstance>,
}

impl SessionModules {
    pub(super) fn for_catalog(catalog: &bsl_bytecode::ConfigurationProgram) -> Self {
        Self {
            instances: catalog
                .modules
                .iter()
                .map(|module| ModuleInstance {
                    state: ModuleState::new(&module.program),
                    init: ModuleInitState::NotStarted,
                })
                .collect(),
        }
    }
}

/// Каталожный контекст одного poll: программы модулей и их связанные
/// компонентные таблицы. Живёт не дольше poll, как `LinkedComponents`.
pub struct CatalogContext<'a> {
    pub(super) catalog: &'a bsl_bytecode::ConfigurationProgram,
    pub(super) linked: Vec<LinkedComponents<'a>>,
}

/// Модульный контекст шага: сессия, каталог и корневое состояние одним
/// указателем. Горячий `step` получает его вместо трёх отдельных
/// параметров — регистровое давление в цикле диспетчеризации измеримо
/// (A/B чередованием: +10% `call_overhead` на трёх параметрах).
pub(super) struct ModulesCtx<'a, 'b> {
    pub(super) session: &'a mut SessionModules,
    pub(super) catalog: Option<&'a CatalogContext<'b>>,
    /// Корневое состояние, когда текущий кадр — модульный (его собственное
    /// состояние на время шага изъято из сессии); `None` у корневого кадра.
    pub(super) root_state: Option<&'a mut ModuleState>,
}

impl<'a> CatalogContext<'a> {
    pub(super) fn program(&self, module: u32) -> Result<&'a Program, RtError> {
        self.catalog
            .modules
            .get(module as usize)
            .map(|m| &m.program)
            .ok_or(RtError::InvalidBytecode(
                "номер модуля кадра вне каталога конфигурации",
            ))
    }

    fn linked(&self, module: u32) -> Result<&LinkedComponents<'a>, RtError> {
        self.linked
            .get(module as usize)
            .ok_or(RtError::InvalidBytecode(
                "номер модуля кадра вне таблиц линковки",
            ))
    }

    /// Программа, линковка и кэши одного модуля выбираются общим ключом.
    /// Кэши заимствуются отдельно от сеанса: шаг одновременно держит их
    /// разделяемо и изменяет модульные переменные через `ModulesCtx`.
    /// Порядок проверок сохраняет прежний выбор ошибки при неверном номере.
    pub(super) fn execution_parts<'s>(
        &'s self,
        module: u32,
        caches: &'s [RunCaches],
    ) -> Result<(&'a Program, &'s LinkedComponents<'a>, &'s RunCaches), RtError> {
        Ok((
            self.program(module)?,
            self.linked(module)?,
            at(
                caches,
                module as usize,
                "номер модуля вне таблицы кэшей запуска",
            )?,
        ))
    }
}

/// `frames.pop().expect(...)` ниже — ВНУТРЕННИЙ ИНВАРИАНТ VM, а не входные
/// данные (см. классификацию в шапке модуля): `step`/`drive` вызывают эту
/// функцию только пока `frames` не пуст — сам факт того, что мы исполняем
/// инструкцию, это гарантирует. Никакой байт-код, корректный или нет, сюда
/// с пустым стеком кадров не приведёт.
/// Готовит модуль каталога к обращению. `Ok(true)` означает, что кадр
/// тела модуля запушен и текущая инструкция должна исполниться повторно
/// после его возврата; `pc` вызывающего при этом не продвинут.
///
/// # Errors
///
/// Ловимая ошибка при циклической инициализации и при обращении к модулю,
/// чьё тело уже завершилось ошибкой; политика повторного запуска — за
/// замером `JOB.MODULE.INIT`.
pub(super) fn ensure_module_ready(
    target: u32,
    ctx: &CatalogContext<'_>,
    session: &mut SessionModules,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
) -> Result<bool, RtError> {
    let instance = session
        .instances
        .get_mut(target as usize)
        .ok_or(RtError::InvalidBytecode("связь ведёт мимо сессии модулей"))?;
    match instance.init {
        ModuleInitState::Ready => Ok(false),
        ModuleInitState::NotStarted => {
            if frames.len() >= MAX_CALL_DEPTH {
                return Err(RtError::StackOverflow {
                    what: "слишком глубокая рекурсия вызовов",
                });
            }
            instance.init = ModuleInitState::Initializing;
            let body = ctx.program(target)?;
            let chunk0 = at(&body.chunks, 0, "у модуля каталога нет тела")?;
            let call_start = stack.len();
            let own_base = stack.len();
            push_own_registers(stack, chunk0);
            frames.push(Frame {
                module: target,
                func_id: 0,
                pc: 0,
                param_aliases: Vec::new(),
                own_base,
                call_start,
                return_reg: 0,
                module_copybacks: Vec::new(),
                numeric_for_state: None,
            });
            Ok(true)
        }
        ModuleInitState::Initializing => Err(RtError::DynamicError(
            "циклическая инициализация общего модуля".into(),
        )),
        ModuleInitState::Failed => Err(RtError::DynamicError(
            "инициализация общего модуля завершилась ошибкой".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> bsl_bytecode::ConfigurationProgram {
        bsl_bytecode::ConfigurationProgram {
            modules: ["Возврат 1;", "а = 2; Возврат а;"]
                .into_iter()
                .enumerate()
                .map(|(index, source)| bsl_bytecode::ModuleProgram {
                    name: format!("Модуль{index}"),
                    program: crate::tests::compile_module(source),
                })
                .collect(),
        }
    }

    fn context(catalog: &bsl_bytecode::ConfigurationProgram) -> CatalogContext<'_> {
        let env = bsl_rt::HostEnv::process();
        let linked = catalog
            .modules
            .iter()
            .map(|module| {
                crate::link_components(
                    &module.program,
                    None,
                    env.zone(),
                    env.files(),
                    env.random(),
                    env.network(),
                    env.background_jobs(),
                    env.temp_storage(),
                    env.message_sink(),
                    bsl_bytecode::DynamicScope::ROOT,
                )
                .expect("модуль связывается")
            })
            .collect();
        CatalogContext { catalog, linked }
    }

    #[test]
    fn execution_parts_use_one_module_key_without_moving_caches_into_the_session() {
        let catalog = catalog();
        let ctx = context(&catalog);
        let caches: Vec<_> = catalog
            .modules
            .iter()
            .map(|module| RunCaches::for_program(&module.program))
            .collect();
        let mut session = SessionModules::for_catalog(&catalog);

        for index in 0..catalog.modules.len() {
            let (program, linked, selected_caches) = ctx
                .execution_parts(index as u32, &caches)
                .expect("согласованные таблицы");
            assert!(std::ptr::eq(program, &catalog.modules[index].program));
            assert!(std::ptr::eq(linked, &ctx.linked[index]));
            assert!(std::ptr::eq(selected_caches, &caches[index]));
            session.instances[index].init = ModuleInitState::Ready;
            assert_eq!(selected_caches.prop.len(), program.chunks.len());
            assert_eq!(session.instances[index].init, ModuleInitState::Ready);
        }
    }

    #[test]
    fn execution_parts_preserve_the_order_and_text_of_invalid_table_errors() {
        let catalog = catalog();
        let ctx = CatalogContext {
            catalog: &catalog,
            linked: Vec::new(),
        };
        assert!(matches!(
            ctx.execution_parts(2, &[]),
            Err(RtError::InvalidBytecode(
                "номер модуля кадра вне каталога конфигурации"
            ))
        ));
        assert!(matches!(
            ctx.execution_parts(0, &[]),
            Err(RtError::InvalidBytecode(
                "номер модуля кадра вне таблиц линковки"
            ))
        ));
        let ctx = context(&catalog);
        assert!(matches!(
            ctx.execution_parts(0, &[]),
            Err(RtError::InvalidBytecode(
                "номер модуля вне таблицы кэшей запуска"
            ))
        ));
    }
}
