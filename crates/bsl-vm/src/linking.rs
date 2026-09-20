use super::field_name;
use bsl_bytecode::{DynamicCompiler, Instr, Program};
use bsl_rt::{BslValue, RtError};
use std::io::Write;

/// Host-сервисы одного прогона: куда писать и откуда брать то, чего нет в
/// аргументах BSL-функции (аргументы запуска, часы, случайность).
///
/// Вывод и окружение ходят вместе, потому что оба принадлежат ПРОГОНУ, а
/// не программе: `Program` сериализуется, а это — нет.
pub(super) struct HostIo<'a, 'd> {
    pub(super) stdout: &'a mut dyn Write,
    pub(super) stderr: &'a mut dyn Write,
    /// `None` бывает только во время переноса окружения во вложенный
    /// обратный вызов компонента.
    pub(super) env: Option<&'a mut bsl_rt::HostEnv>,
    /// Компилятор динамического кода. Он тоже принадлежит ПРОГОНУ, а не
    /// программе, и лежит здесь по той же причине, что и вывод: VM его
    /// зовёт, но не реализует (см. `bsl_bytecode::dynamic`).
    ///
    /// `None` — прогон запущен входом без компилятора фрагментов
    /// (`run_program`, `call_module_function`): тогда `Выполнить` и
    /// `Вычислить` дают ловимую динамическую ошибку, а не тихо ничего.
    pub(super) dynamic: Option<&'a mut (dyn DynamicCompiler + 'd)>,
    /// Текущая вложенность `Выполнить`/`Вычислить` ЭТОГО прогона. Раньше
    /// была потоковой (`thread_local`) и делилась между сессиями одного
    /// потока; теперь принадлежит прогону. Вложенный `drive`
    /// и обратный вызов функции модуля строят заимствованный `HostIo`
    /// с тем же счётчиком (см. `DynamicDepthGuard`).
    pub(super) dynamic_depth: &'a std::cell::Cell<usize>,
    /// Тот же флаг отмены, который проверяется между квантами исполнения.
    pub(super) cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Владелец файловых обещаний окружающего динамический фрагмент
    /// исполнения. Заимствование не переживает вложенный drive; сами
    /// операции принадлежат внешнему AsyncState, а не стеку фрагмента.
    pub(super) file_promises: Option<&'a mut super::AsyncState>,
}

impl HostIo<'_, '_> {
    /// Проверка отмены внутри длительной синхронной операции host.
    pub(super) fn check_canceled(&self) -> Result<(), RtError> {
        if self
            .cancel_flag
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
        {
            Err(RtError::Canceled)
        } else {
            Ok(())
        }
    }
    /// Окружение прогона или ошибка, если его нет.
    ///
    /// # Errors
    ///
    /// [`RtError::InvalidBytecode`], если окружение не было передано во
    /// вложенный вызов.
    pub(super) fn env(&mut self) -> Result<&mut bsl_rt::HostEnv, RtError> {
        self.env.as_deref_mut().ok_or(RtError::InvalidBytecode(
            "функция окружения вызвана там, где окружения прогона нет",
        ))
    }

    /// Компилятор фрагментов этого прогона или ловимая ошибка.
    ///
    /// # Errors
    ///
    /// [`RtError::DynamicError`], если прогон запущен входом без
    /// компилятора: динамический код в таком прогоне недоступен, и узнаётся
    /// это только сейчас — значит, ошибка обычная, ловимая `Попытка`.
    pub(super) fn dynamic(&mut self) -> Result<&mut dyn DynamicCompiler, RtError> {
        match self.dynamic.as_deref_mut() {
            Some(compiler) => Ok(compiler),
            None => Err(RtError::DynamicError(
                "Выполнить/Вычислить недоступны: прогон запущен без компилятора динамического кода"
                    .to_string(),
            )),
        }
    }
}

pub(super) struct LinkedComponents<'a> {
    /// Каталог компонентов прогона. `None` — прогон без реестра: базовый
    /// рантайм и ничего сверх него.
    pub(super) registry: Option<&'a bsl_rt::RuntimeRegistry>,
    pub(super) tables: std::rc::Rc<LinkedTables>,
}

/// Связанные таблицы не заимствуют Program или RuntimeRegistry: дескрипторы
/// содержат статические обработчики, а host-возможности имеют своих владельцев.
/// Динамический образ может удерживать их между опросами исполнения.
pub(super) struct LinkedTables {
    /// Чей нулевой чанк исполняется: `DynamicScope::ROOT` у самого модуля
    /// и номер фрагмента (`DynamicUnit::scope`) у программы, собранной
    /// вокруг фрагмента. Половина ключа, по которому хост кэширует
    /// скомпилированные фрагменты, — и приезжает она СНАРУЖИ, потому что
    /// раздаёт номера хост, а не VM.
    pub(super) scope: u64,
    /// Владелец кода фрагмента. Сохраняет исходный модуль в том числе
    /// при отладочном вычислении отдельным исполнителем с ROOT_MODULE.
    pub(super) scope_module: Option<bsl_bytecode::ModuleId>,

    functions: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    constructors: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    object_methods: Vec<Vec<Option<LinkedObjectMethod>>>,
    /// Мемоизация «(статическая таблица типа, номер имени) → обработчик»
    /// для компонентных объектов со статическими таблицами методов
    /// (см. `ObjectProtocol::method_table`): разрешение по строке
    /// происходит один раз на пару, дальше — целочисленный поиск по хешу.
    /// `None` запоминает и промахи, чтобы типы без таблиц не платили за
    /// строку на каждом вызове. Рантайм однопоточный, `RefCell` достаточно.
    pub(super) component_methods: ComponentMethodMap,
    /// То же для статических таблиц СВОЙСТВ (`ObjectProtocol::property_table`).
    /// Ячейки инструкции у свойств пока нет: она — производная таблица
    /// `Chunk`, её добавление стоит правок формата байт-кода, а выигрыш не
    /// измерен.
    pub(super) component_properties: ComponentPropertyMap,
    /// Часовой пояс прогона, доступный компонентам.
    pub(super) zone: std::rc::Rc<dyn bsl_rt::TimeZone>,
    pub(super) files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    pub(super) random: bsl_rt::RandomHandle,
    pub(super) network: Option<std::rc::Rc<dyn bsl_rt::HttpClientFactory>>,
    pub(super) background_jobs: Option<std::rc::Rc<dyn bsl_rt::BackgroundJobService>>,
    pub(super) temp_storage: Option<std::rc::Rc<std::cell::RefCell<bsl_rt::TempStorageSession>>>,
    pub(super) message_sink: Option<std::rc::Rc<dyn bsl_rt::UserMessageSink>>,
}

#[derive(Clone, Copy)]
pub(super) struct LinkedObjectMethod {
    pub(super) ty: &'static bsl_rt::TypeDescriptor,
    pub(super) name: bsl_rt::NameId,
    pub(super) descriptor: Option<&'static bsl_rt::MethodDescriptor>,
    // Геометрия и успешные проверки не меняются между вызовами.
    // Ошибочные сайты остаются ловимыми: флаги ниже возвращают
    // редкий путь к исходной проверке в точке исполнения.
    pub(super) count: usize,
    pub(super) invalid_arity: bool,
    pub(super) invalid_result_use: bool,
}

impl LinkedComponents<'_> {
    pub(super) fn with_scope_module(mut self, module: Option<bsl_bytecode::ModuleId>) -> Self {
        // Область устанавливается сразу после связывания, до разделения
        // таблиц с кадрами или продолжениями.
        std::rc::Rc::get_mut(&mut self.tables)
            .expect("область задаётся до разделения связанных таблиц")
            .scope_module = module;
        self
    }

    pub(super) fn function(
        &self,
        func_id: usize,
        pc: usize,
    ) -> Result<bsl_rt::ComponentCall, RtError> {
        self.functions
            .get(func_id)
            .and_then(|chunk| chunk.get(pc))
            .and_then(|slot| *slot)
            .ok_or(RtError::InvalidBytecode(
                "CallComponent не связан с функцией реестра",
            ))
    }

    pub(super) fn constructor(
        &self,
        func_id: usize,
        pc: usize,
    ) -> Result<bsl_rt::ComponentCall, RtError> {
        self.constructors
            .get(func_id)
            .and_then(|chunk| chunk.get(pc))
            .and_then(|slot| *slot)
            .ok_or(RtError::InvalidBytecode(
                "CreateObject не связан с конструктором реестра",
            ))
    }

    pub(super) fn object_method(
        &self,
        func_id: usize,
        pc: usize,
    ) -> Result<LinkedObjectMethod, RtError> {
        self.object_methods
            .get(func_id)
            .and_then(|chunk| chunk.get(pc))
            .and_then(|slot| *slot)
            .ok_or(RtError::InvalidBytecode(
                "типизированный вызов не связан с методом объекта",
            ))
    }
}

impl std::ops::Deref for LinkedComponents<'_> {
    type Target = LinkedTables;

    fn deref(&self) -> &Self::Target {
        &self.tables
    }
}

/// Карта мемоизации «(статическая таблица типа, номер имени) → дескриптор».
/// Хранится дескриптор, а не голый обработчик: рантаймная проверка арности
/// метода (арм `CallObjectMethod`) читает из него `arity`.
pub(super) type ComponentMethodMap = std::cell::RefCell<
    std::collections::HashMap<(usize, u32), Option<&'static bsl_rt::MethodDescriptor>>,
>;

/// Разрешение метода компонентного объекта по статической таблице типа и
/// номеру имени. Строка разбирается один раз на пару «таблица, имя»;
/// установившийся режим — поиск по хешу от двух целых, промахи тоже
/// запоминаются. `None` — имени в таблице нет (или таблица пустая):
/// вызывающий уходит в строковый `call_method`, чтобы текст ошибки остался
/// одним, у самого типа.
/// Карта мемоизации «(статическая таблица типа, номер имени) → пара
/// обработчиков свойства».
pub(super) type ComponentPropertyMap = std::cell::RefCell<
    std::collections::HashMap<
        (usize, u32),
        Option<(bsl_rt::PropertyGet, Option<bsl_rt::PropertySet>)>,
    >,
>;

/// Разрешение свойства компонентного объекта — зеркало
/// [`resolve_component_method`]: строка разбирается один раз на пару
/// «таблица, имя», промахи запоминаются тоже. `None` — имени в таблице нет
/// (или таблица пустая), и вызывающий уходит строковым путём, где у типа
/// остаётся единственный источник текста ошибки.
fn resolve_component_property(
    map: &ComponentPropertyMap,
    table: &'static [bsl_rt::PropertyDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<(bsl_rt::PropertyGet, Option<bsl_rt::PropertySet>)>, RtError> {
    let key = (table.as_ptr() as usize, name.index() as u32);
    if let Some(resolved) = map.borrow().get(&key) {
        return Ok(*resolved);
    }
    let written = field_name(program, name)?;
    let resolved = table
        .iter()
        .find(|descriptor| {
            descriptor
                .names
                .iter()
                .any(|candidate| bsl_rt::folded_eq(candidate, written))
        })
        .map(|descriptor| (descriptor.get, descriptor.set));
    map.borrow_mut().insert(key, resolved);
    Ok(resolved)
}

/// Чтение свойства компонентного объекта: таблица типа — быстрым путём,
/// промах и тип без таблицы — строковым `get_property`. Вынесено из арма и
/// помечено `#[inline(never)]`: тело крупное, а горячий цикл живёт на
/// грани кеша микроопераций (см. комментарий у `step_cold`).
#[inline(never)]
pub(super) fn component_prop_get(
    object: &bsl_rt::ObjectRef,
    properties: &ComponentPropertyMap,
    name: bsl_rt::NameId,
    program: &Program,
    context: &mut bsl_rt::CallContext<'_>,
) -> Result<BslValue, RtError> {
    let table = object.property_table();
    if !table.is_empty()
        && let Some((get, _)) = resolve_component_property(properties, table, name, program)?
    {
        return get(object.as_dyn(), context);
    }
    object.get_property(field_name(program, name)?, context)
}

/// Запись свойства компонентного объекта — двойник `component_prop_get`.
#[inline(never)]
pub(super) fn component_prop_set(
    object: &bsl_rt::ObjectRef,
    properties: &ComponentPropertyMap,
    name: bsl_rt::NameId,
    value: BslValue,
    program: &Program,
    context: &mut bsl_rt::CallContext<'_>,
) -> Result<(), RtError> {
    let table = object.property_table();
    if !table.is_empty()
        && let Some((_, set)) = resolve_component_property(properties, table, name, program)?
    {
        return match set {
            Some(set) => set(object.as_dyn(), value, context),
            None => Err(RtError::PropertyReadOnly {
                property: field_name(program, name)?.to_string(),
                receiver: object.type_descriptor().name,
            }),
        };
    }
    object.set_property(field_name(program, name)?, value, context)
}

pub(super) fn resolve_component_method(
    map: &ComponentMethodMap,
    table: &'static [bsl_rt::MethodDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<&'static bsl_rt::MethodDescriptor>, RtError> {
    let key = (table.as_ptr() as usize, name.index() as u32);
    if let Some(resolved) = map.borrow().get(&key) {
        return Ok(*resolved);
    }
    let resolved = bsl_rt::find_method_from_table(table, field_name(program, name)?);
    map.borrow_mut().insert(key, resolved);
    Ok(resolved)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn link_components<'a>(
    program: &'a Program,
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    zone: std::rc::Rc<dyn bsl_rt::TimeZone>,
    files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    random: bsl_rt::RandomHandle,
    network: Option<std::rc::Rc<dyn bsl_rt::HttpClientFactory>>,
    background_jobs: Option<std::rc::Rc<dyn bsl_rt::BackgroundJobService>>,
    temp_storage: Option<std::rc::Rc<std::cell::RefCell<bsl_rt::TempStorageSession>>>,
    message_sink: Option<std::rc::Rc<dyn bsl_rt::UserMessageSink>>,
    scope: u64,
) -> Result<LinkedComponents<'a>, RtError> {
    // Периметр стоит ПЕРВОЙ строкой и отделён от самого связывания
    // (`link_verified` ниже) ради одного-единственного теста — того, что
    // проверяет РАЗМОТКУ ошибки из вложенного кадра. Классы порчи,
    // которыми он раньше пользовался, периметр теперь отвергает до
    // исполнения, и это правильно; но проверять размотку всё равно надо,
    // а сохранять ради неё дыру в периметре — нельзя. Шов приватный и
    // из рабочего пути недостижим.
    bsl_bytecode::image::verify(program)?;
    link_verified(
        program,
        registry,
        zone,
        files,
        random,
        network,
        background_jobs,
        temp_storage,
        message_sink,
        scope,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn link_verified<'a>(
    program: &'a Program,
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    zone: std::rc::Rc<dyn bsl_rt::TimeZone>,
    files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    random: bsl_rt::RandomHandle,
    network: Option<std::rc::Rc<dyn bsl_rt::HttpClientFactory>>,
    background_jobs: Option<std::rc::Rc<dyn bsl_rt::BackgroundJobService>>,
    temp_storage: Option<std::rc::Rc<std::cell::RefCell<bsl_rt::TempStorageSession>>>,
    message_sink: Option<std::rc::Rc<dyn bsl_rt::UserMessageSink>>,
    scope: u64,
) -> Result<LinkedComponents<'a>, RtError> {
    let Some(core) = program.requirements.first() else {
        return Err(RtError::Link(
            "в требованиях отсутствует bsl-rt".to_string(),
        ));
    };
    if core.package != bsl_rt::PACKAGE_NAME || core.version != bsl_rt::PACKAGE_VERSION {
        return Err(RtError::Link(format!(
            "необходим {}={}, исполнитель предоставляет {}={}",
            core.package,
            core.version,
            bsl_rt::PACKAGE_NAME,
            bsl_rt::PACKAGE_VERSION
        )));
    }

    for requirement in &program.requirements[1..] {
        let Some(registry) = registry else {
            return Err(RtError::Link(format!(
                "необходим пакет {}={}, но реестр компонентов не предоставлен",
                requirement.package, requirement.version
            )));
        };
        let Some(library) = registry.library_by_package(&requirement.package) else {
            return Err(RtError::Link(format!(
                "необходим пакет {}={}, но он не зарегистрирован",
                requirement.package, requirement.version
            )));
        };
        if library.version() != requirement.version {
            return Err(RtError::Link(format!(
                "для {} требуется {}, зарегистрирована версия {}",
                requirement.package,
                requirement.version,
                library.version()
            )));
        }
    }

    let mut functions = Vec::with_capacity(program.chunks.len());
    let mut constructors = Vec::with_capacity(program.chunks.len());
    let mut object_methods = Vec::with_capacity(program.chunks.len());
    for chunk in &program.chunks {
        let mut function_slots = vec![None; chunk.instrs.len()];
        let mut constructor_slots = vec![None; chunk.instrs.len()];
        let mut object_method_slots = vec![None; chunk.instrs.len()];
        for (pc, instruction) in chunk.instrs.iter().enumerate() {
            match instruction {
                Instr::CallComponent {
                    library,
                    function,
                    count,
                    ..
                } => {
                    let requirement = program.requirements.get(*library as usize).ok_or(
                        RtError::InvalidBytecode("индекс библиотеки вне таблицы requirements"),
                    )?;
                    let Some(registry) = registry else {
                        return Err(RtError::Link(format!(
                            "функция {}/{} требует реестр компонентов",
                            requirement.package, function
                        )));
                    };
                    let library_descriptor = registry
                        .library_by_package(&requirement.package)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "необходим пакет {}={}, но он не зарегистрирован",
                                requirement.package, requirement.version
                            ))
                        })?;
                    let descriptor = library_descriptor
                        .functions()
                        .iter()
                        .find(|descriptor| descriptor.code.get() == *function)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "компонент {} не содержит функцию с кодом {}",
                                requirement.package, function
                            ))
                        })?;
                    if !descriptor.arity.accepts(*count) {
                        return Err(RtError::InvalidBytecode(
                            "арность CallComponent не совпадает с дескриптором",
                        ));
                    }
                    function_slots[pc] = Some(descriptor.call);
                }
                Instr::CreateObject {
                    library,
                    constructor,
                    count,
                    ..
                } => {
                    let requirement = program.requirements.get(*library as usize).ok_or(
                        RtError::InvalidBytecode("индекс библиотеки вне таблицы requirements"),
                    )?;
                    let library_descriptor = match registry {
                        Some(registry) => *registry
                            .library_by_package(&requirement.package)
                            .ok_or_else(|| {
                                RtError::Link(format!(
                                    "необходим пакет {}={}, но он не зарегистрирован",
                                    requirement.package, requirement.version
                                ))
                            })?,
                        None if requirement.package == bsl_rt::PACKAGE_NAME => {
                            bsl_rt::core_library()
                        }
                        None => {
                            return Err(RtError::Link(format!(
                                "конструктор {}/{} требует реестр компонентов",
                                requirement.package, constructor
                            )));
                        }
                    };
                    let descriptor = library_descriptor
                        .constructors()
                        .iter()
                        .find(|descriptor| descriptor.code.get() == *constructor)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "компонент {} не содержит конструктор с кодом {}",
                                requirement.package, constructor
                            ))
                        })?;
                    if !descriptor.arity.accepts(*count) {
                        return Err(RtError::InvalidBytecode(
                            "арность CreateObject не совпадает с дескриптором",
                        ));
                    }
                    constructor_slots[pc] = Some(descriptor.call);
                }
                Instr::CallLinkedObjectMethod {
                    link_slot,
                    arg_modes,
                    ..
                }
                | Instr::CallLinkedObjectProcedure {
                    link_slot,
                    arg_modes,
                    ..
                } => {
                    let bsl_bytecode::LinkEntry::ObjectMethod {
                        library,
                        type_name,
                        method,
                    } = program.links[*link_slot as usize]
                    else {
                        return Err(RtError::InvalidBytecode(
                            "типизированный вызов ссылается не на метод объекта",
                        ));
                    };
                    let requirement = &program.requirements[library as usize];
                    let registry = registry.ok_or_else(|| {
                        RtError::Link(format!(
                            "метод типа из {} требует реестр компонентов",
                            requirement.package
                        ))
                    })?;
                    let library_descriptor = registry
                        .library_by_package(&requirement.package)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "необходим пакет {}={}, но он не зарегистрирован",
                                requirement.package, requirement.version
                            ))
                        })?;
                    let type_written =
                        field_name(program, bsl_rt::NameId::from_index(type_name as u32))?;
                    let ty = library_descriptor
                        .types()
                        .iter()
                        .copied()
                        .find(|ty| ty.name == type_written)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "компонент {} не содержит тип {type_written}",
                                requirement.package
                            ))
                        })?;
                    let members = library_descriptor
                        .object_members()
                        .find(|members| std::ptr::eq(members.ty(), ty))
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "тип {type_written} не объявляет таблицу методов"
                            ))
                        })?;
                    let name = bsl_rt::NameId::from_index(method as u32);
                    let descriptor = bsl_rt::find_method_from_table(
                        members.methods(),
                        field_name(program, name)?,
                    );
                    let count = chunk.call_arg_modes[*arg_modes as usize].len();
                    let result_required =
                        matches!(instruction, Instr::CallLinkedObjectMethod { .. });
                    let invalid_arity = descriptor.is_some_and(|descriptor| {
                        descriptor
                            .check_arity(u8::try_from(count).unwrap_or(u8::MAX), ty.name)
                            .is_err()
                    });
                    let invalid_result_use = descriptor.is_some_and(|descriptor| {
                        descriptor.check_result_use(result_required).is_err()
                    });
                    object_method_slots[pc] = Some(LinkedObjectMethod {
                        ty,
                        name,
                        descriptor,
                        count,
                        invalid_arity,
                        invalid_result_use,
                    });
                }
                _ => {}
            }
        }
        functions.push(function_slots);
        constructors.push(constructor_slots);
        object_methods.push(object_method_slots);
    }
    Ok(LinkedComponents {
        registry,
        tables: std::rc::Rc::new(LinkedTables {
            scope,
            scope_module: None,
            zone,
            files,
            random,
            network,
            background_jobs,
            temp_storage,
            message_sink,
            functions,
            constructors,
            object_methods,
            component_methods: std::cell::RefCell::new(std::collections::HashMap::new()),
            component_properties: std::cell::RefCell::new(std::collections::HashMap::new()),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_tables_outlive_the_program_and_registry_and_share_their_caches() {
        let (tables, files) = {
            let program = crate::tests::compile_module("Возврат 1;");
            let mut builder = bsl_rt::RuntimeBuilder::new();
            builder.register(bsl_rt::core_library());
            let registry = builder.build().unwrap();
            let env = bsl_rt::HostEnv::process();
            let files = std::rc::Rc::downgrade(&env.files());
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
                7,
            )
            .unwrap()
            .with_scope_module(Some(bsl_bytecode::ModuleId::new(2)));
            (linked.tables.clone(), files)
        };
        assert!(files.upgrade().is_some());
        let first = LinkedComponents {
            registry: None,
            tables: tables.clone(),
        };
        let second = LinkedComponents {
            registry: None,
            tables: tables.clone(),
        };
        assert_eq!(second.scope, 7);
        assert_eq!(second.scope_module, Some(bsl_bytecode::ModuleId::new(2)));
        first.component_methods.borrow_mut().insert((1, 2), None);
        assert!(second.component_methods.borrow().contains_key(&(1, 2)));
        assert!(std::rc::Rc::ptr_eq(&first.tables, &second.tables));
        drop(first);
        drop(tables);
        assert!(files.upgrade().is_some());
        drop(second);
        assert!(files.upgrade().is_none());
    }
}
