use super::{Frame, LinkedComponents, MAX_CALL_DEPTH, RunCaches, at, push_own_registers};
use bsl_bytecode::Program;
use bsl_rt::{BslValue, RtError};

mod reference_call;
pub(super) use reference_call::call_reference;
mod aliases;
pub(super) use aliases::{
    bind_module_argument, ensure_imported_arguments_ready, find_module_alias,
    load_imported_argument, load_module_argument, park_task, publish_aliases, refresh_aliases,
};

pub(super) struct ModuleState {
    pub(super) slots: Vec<BslValue>,
    // Разделяется с динамическими фрагментами, но не с новым запуском.
    // Пустой scratch при перестановке кадра не создаёт ссылку.
    identity: Option<BslValue>,
}

impl ModuleState {
    pub(super) fn new(program: &Program) -> Self {
        Self::new_with_type(program, &bsl_rt::BSL_MODULE_TYPE)
    }

    fn new_with_type(program: &Program, ty: &'static bsl_rt::TypeDescriptor) -> Self {
        Self {
            slots: vec![BslValue::Undefined; program.module_vars.len()],
            identity: Some(BslValue::new_object(ModuleReference {
                ty,
                exports: program
                    .function_names
                    .iter()
                    .zip(&program.exported_functions)
                    .filter(|(_, exported)| **exported)
                    .map(|(name, _)| name.to_uppercase())
                    .collect(),
            })),
        }
    }

    pub(super) fn empty() -> Self {
        Self {
            slots: Vec::new(),
            identity: None,
        }
    }

    /// Переносит переменные во вложенный исполнитель того же модуля.
    pub(super) fn take_for_execution(&mut self) -> Self {
        Self {
            slots: std::mem::take(&mut self.slots),
            identity: self.identity.clone(),
        }
    }

    /// Подменяет значение `ЭтотОбъект` на время вызова, инициированного
    /// компонентом, и возвращает прежнее значение для восстановления.
    pub(super) fn replace_reference(&mut self, value: BslValue) -> Option<BslValue> {
        self.identity.replace(value)
    }

    pub(super) fn reference(&self) -> Result<BslValue, RtError> {
        let identity = self.identity.as_ref().ok_or(RtError::InvalidBytecode(
            "ссылка на модуль запрошена у временного пустого состояния",
        ))?;
        Ok(identity.clone())
    }

    fn owns_reference(&self, value: &BslValue) -> bool {
        self.identity.as_ref() == Some(value)
    }
}

/// Не содержит переменных или ссылки обратно на ModuleState: значение
/// в собственной модульной переменной не образует цикл владения.
#[derive(Debug)]
pub(super) struct ModuleReference {
    ty: &'static bsl_rt::TypeDescriptor,
    exports: Vec<String>,
}

impl bsl_rt::ObjectProtocol for ModuleReference {
    fn type_descriptor(&self) -> &'static bsl_rt::TypeDescriptor {
        self.ty
    }

    fn has_method(&self, name: &str) -> bool {
        let name = name.to_uppercase();
        self.exports.iter().any(|export| export == &name)
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;

    #[test]
    fn an_external_reference_does_not_resurrect_a_completed_module_instance() {
        let ast = bsl_syntax::parse("Функция Значение() Экспорт Возврат 42; КонецФункции Функция Вызвать(Получатель) Возврат Получатель.Значение(); КонецФункции Возврат ЭтотОбъект;").unwrap();
        let resolved = bsl_sema::resolve_program(&ast.items).unwrap();
        let program = bsl_compiler::compile_program(&resolved).unwrap();
        let reference = crate::run_program(&program).unwrap();
        let error =
            crate::call_module_function(&program, &mut [], "Вызвать", vec![reference]).unwrap_err();
        assert!(
            matches!(error, RtError::DynamicError(message) if message.contains("не принадлежит текущему исполнению"))
        );
    }

    #[test]
    fn a_module_variable_holding_its_owner_reference_does_not_retain_the_state() {
        let ast = bsl_syntax::parse("Перем Ссылка; Процедура Обработать() Экспорт КонецПроцедуры")
            .unwrap();
        let resolved = bsl_sema::resolve_program(&ast.items).unwrap();
        let program = bsl_compiler::compile_program(&resolved).unwrap();
        let mut state = ModuleState::new(&program);
        let reference = state.reference().unwrap();
        state.slots[0] = reference.clone();
        let nested = state.take_for_execution();
        assert_eq!(nested.reference().unwrap(), reference);
        drop(nested);
        drop(state);
        assert!(reference.object_ref().unwrap().has_method("Обработать"));
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

impl ModuleInstance {
    pub(super) fn prepare(&mut self, program: &Program) {
        if self.state.identity.is_none() {
            self.state = ModuleState::new_with_type(program, &bsl_rt::COMMON_MODULE_TYPE);
        }
    }
}

/// Сессионные экземпляры всех модулей каталога, индекс — `ModuleId`.
/// У одиночной программы пуст.
#[derive(Default)]
pub struct SessionModules {
    pub(super) instances: Vec<ModuleInstance>,
}

impl SessionModules {
    /// Создаёт пустые экземпляры модулей одного сеанса конфигурации.
    #[must_use]
    pub fn for_catalog(catalog: &dyn bsl_bytecode::ExecutionCatalog) -> Self {
        Self {
            instances: (0..catalog.module_count())
                .map(|index| ModuleInstance {
                    // ИЗМЕРЕНО на 1С 8.3.27: ссылка имеет тип
                    // `ОбщийМодуль` с отображением `Common module`.
                    state: catalog
                        .prepared_module(bsl_bytecode::ModuleId::new(index as u32))
                        .map_or_else(ModuleState::empty, |module| {
                            ModuleState::new_with_type(&module.program, &bsl_rt::COMMON_MODULE_TYPE)
                        }),
                    init: ModuleInitState::NotStarted,
                })
                .collect(),
        }
    }
}

/// Каталожный контекст одного poll: программы модулей и их связанные
/// компонентные таблицы. Живёт не дольше poll, как `LinkedComponents`.
pub struct CatalogContext<'a> {
    pub(super) catalog: &'a dyn bsl_bytecode::ExecutionCatalog,
    pub(super) linked: ModuleLinks<'a>,
}

type LinkFactory<'a> = dyn Fn(u32) -> Result<LinkedComponents<'a>, RtError> + 'a;

/// Таблицы host-сервисов создаются только для подготовленного кода.
/// Ошибка также сохраняется: повторное обращение в одном poll не запускает
/// загрузку или связывание повторно.
pub(super) struct ModuleLinks<'a> {
    slots: Vec<std::cell::OnceCell<Result<LinkedComponents<'a>, RtError>>>,
    factory: Option<Box<LinkFactory<'a>>>,
}

impl<'a> ModuleLinks<'a> {
    pub(super) fn for_catalog(
        catalog: &'a dyn bsl_bytecode::ExecutionCatalog,
        registry: &'a bsl_rt::RuntimeRegistry,
        env: &bsl_rt::HostEnv,
    ) -> Result<Self, RtError> {
        let (zone, files, random, clock) = (env.zone(), env.files(), env.random(), env.clock());
        let (network, data, jobs) = (env.network(), env.host_data(), env.background_jobs());
        let (storage, sink) = (env.temp_storage(), env.message_sink());
        let verifier = std::cell::RefCell::new(bsl_bytecode::image::VerificationCache::default());
        let links = Self {
            slots: (0..catalog.module_count())
                .map(|_| std::cell::OnceCell::new())
                .collect(),
            factory: Some(Box::new(move |index| {
                let module = bsl_bytecode::ModuleId::new(index);
                let program = &catalog.module_code(module)?.program;
                // Тела проверяются до связывания, общие неизменяемые
                // таблицы каталога проходят ту же проверку только один раз.
                verifier.borrow_mut().verify(program)?;
                super::linking::link_verified(
                    program,
                    Some(registry),
                    zone.clone(),
                    files.clone(),
                    random.clone(),
                    network.clone(),
                    data.clone(),
                    jobs.clone(),
                    storage.clone(),
                    sink.clone(),
                    bsl_bytecode::DynamicScope::ROOT,
                )
                .map(|linked| {
                    linked
                        .with_clock(Some(clock.clone()))
                        .with_scope_module(Some(module))
                })
            })),
        };
        // Для полного образа сохраняем прежние ранние ошибки связывания.
        // Индексный каталог не запрашивает здесь ещё не подготовленные тела.
        for index in 0..catalog.module_count() {
            if catalog
                .prepared_module(bsl_bytecode::ModuleId::new(index as u32))
                .is_some()
            {
                links.get(index as u32)?;
            }
        }
        Ok(links)
    }

    fn get(&self, module: u32) -> Result<&LinkedComponents<'a>, RtError> {
        let cell = self
            .slots
            .get(module as usize)
            .ok_or(RtError::InvalidBytecode(
                "номер модуля кадра вне таблиц линковки",
            ))?;
        cell.get_or_init(|| {
            self.factory
                .as_ref()
                .ok_or(RtError::InvalidBytecode("для модуля нет таблиц связывания"))?(
                module,
            )
        })
        .as_ref()
        .map_err(Clone::clone)
    }
}

impl<'a> FromIterator<LinkedComponents<'a>> for ModuleLinks<'a> {
    fn from_iter<T: IntoIterator<Item = LinkedComponents<'a>>>(iter: T) -> Self {
        Self {
            slots: iter
                .into_iter()
                .map(|linked| std::cell::OnceCell::from(Ok(linked)))
                .collect(),
            factory: None,
        }
    }
}

#[cfg(test)]
impl<'a> std::ops::Index<usize> for ModuleLinks<'a> {
    type Output = LinkedComponents<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index as u32)
            .expect("таблицы тестового каталога согласованы")
    }
}

/// Модульный контекст шага: сессия, каталог и корневое состояние одним
/// указателем. Горячий `step` получает его вместо трёх отдельных
/// параметров — регистровое давление в цикле диспетчеризации измеримо
/// (A/B чередованием: +10% `call_overhead` на трёх параметрах).
pub(super) struct ModulesCtx<'a, 'b> {
    pub(super) root_program: &'a Program,
    pub(super) session: &'a mut SessionModules,
    pub(super) catalog: Option<&'a CatalogContext<'b>>,
    /// Корневое состояние, когда текущий кадр — модульный (его собственное
    /// состояние на время шага изъято из сессии); `None` у корневого кадра.
    pub(super) root_state: Option<&'a mut ModuleState>,
    /// Номер каталожного модуля, который временно исполняется как корневая
    /// программа вложенного компонентного вызова.
    pub(super) root_catalog_module: Option<u32>,
    /// Режимы безопасности и номер текущего кадра.
    pub(super) security_modes: &'a super::TaskSecurityModes,
    pub(super) security_frame: usize,
}

impl<'a> CatalogContext<'a> {
    pub(super) fn program(&self, module: u32) -> Result<&'a Program, RtError> {
        if self
            .catalog
            .module_name(bsl_bytecode::ModuleId::new(module))
            .is_none()
        {
            return Err(RtError::InvalidBytecode(
                "номер модуля кадра вне каталога конфигурации",
            ));
        }
        Ok(&self
            .catalog
            .module_code(bsl_bytecode::ModuleId::new(module))?
            .program)
    }

    pub(super) fn linked(&self, module: u32) -> Result<&LinkedComponents<'a>, RtError> {
        self.linked.get(module)
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
        // Цикл исполнения зовёт это на каждом шаге кадра модуля, а каждое
        // обращение к каталогу — два динамических вызова: программа
        // берётся один раз.
        let program = self.program(module)?;
        Ok((
            program,
            self.linked(module)?,
            at(
                caches,
                module as usize,
                "номер модуля вне таблицы кэшей запуска",
            )?
            .ready(program),
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
            let body = ctx.program(target)?;
            instance.prepare(body);
            instance.init = ModuleInitState::Initializing;
            let chunk0 = at(&body.chunks, 0, "у модуля каталога нет тела")?;
            let call_start = stack.len();
            let own_base = stack.len();
            push_own_registers(stack, chunk0);
            frames.push(Frame {
                code: None,
                module: target,
                func_id: 0,
                pc: 0,
                param_aliases: Vec::new(),
                own_base,
                call_start,
                return_reg: 0,
                numeric_for_state: None,
                cold: None,
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
                    entry_visible: true,
                    global: false,
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
                    env.host_data(),
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
            linked: std::iter::empty().collect(),
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
