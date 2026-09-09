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
    /// переиспользует тот же `HostIo`, а обратный вызов функции модуля
    /// строит новый с тем же счётчиком (см. `DynamicDepthGuard`).
    pub(super) dynamic_depth: &'a std::cell::Cell<usize>,
}

impl HostIo<'_, '_> {
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
    /// Чей нулевой чанк исполняется: `DynamicScope::ROOT` у самого модуля
    /// и номер фрагмента (`DynamicUnit::scope`) у программы, собранной
    /// вокруг фрагмента. Половина ключа, по которому хост кэширует
    /// скомпилированные фрагменты, — и приезжает она СНАРУЖИ, потому что
    /// раздаёт номера хост, а не VM.
    pub(super) scope: u64,

    functions: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    constructors: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    /// Обработчики встроенных методов по номеру имени программы. Открытый
    /// `CallObjectMethod` исполняется миллионами, и поиск по строке на
    /// каждом вызове недопустим: измерено флеймграфом, до трёх четвертей
    /// времени `csv_write` уходило в `to_uppercase` внутри
    /// `BuiltinMethod::lookup`. Таблица строится один раз при связывании.
    pub(super) builtin_methods: Vec<Option<bsl_rt::BuiltinMethod>>,
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

impl LinkedComponents<'_> {
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

    /// Обработчик встроенного метода по номеру имени — без строковых
    /// операций на вызове. `None` — имя не из таблицы ядра: для нативного
    /// получателя это ошибка «метод не применим».
    pub(super) fn builtin_method(&self, name: bsl_rt::NameId) -> Option<bsl_rt::BuiltinMethod> {
        self.builtin_methods.get(name.index()).copied().flatten()
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
    let upper = field_name(program, name)?.to_uppercase();
    let resolved = table.iter().find(|descriptor| {
        descriptor
            .names()
            .iter()
            .any(|candidate| candidate.to_uppercase() == upper)
    });
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
    for chunk in &program.chunks {
        let mut function_slots = vec![None; chunk.instrs.len()];
        let mut constructor_slots = vec![None; chunk.instrs.len()];
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
                _ => {}
            }
        }
        functions.push(function_slots);
        constructors.push(constructor_slots);
    }
    // Диспетчеризация открытых методов: карта «номер имени → обработчик»
    // строится здесь один раз, чтобы `CallObjectMethod` не искал метод по
    // строке на каждом вызове. Сама `lookup` дешёвая (хеш-карта на
    // процесс), поэтому связывание фрагмента `Выполнить` она не утяжеляет.
    let builtin_methods = program
        .names
        .iter()
        .map(|name| bsl_rt::BuiltinMethod::lookup(name))
        .collect();
    Ok(LinkedComponents {
        registry,
        scope,
        zone,
        files,
        random,
        network,
        background_jobs,
        temp_storage,
        message_sink,
        functions,
        constructors,
        builtin_methods,
        component_methods: std::cell::RefCell::new(std::collections::HashMap::new()),
        component_properties: std::cell::RefCell::new(std::collections::HashMap::new()),
    })
}
