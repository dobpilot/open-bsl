//! Цикл диспетчеризации VM: `match` в `loop`, без computed goto (в Rust его
//! нет и явные хвостовые вызовы не стабилизированы — не воюем с этим здесь,
//! оптимизация диспетчеризации приходит после профилирования, не раньше).
//!
//! Параметры без `Знач` передаются по ссылке, а указатель на регистр
//! вызывающего брать нельзя — рост общего стека значений (`Vec<BslValue>`)
//! его инвалидирует. Вместо указателя параметр хранит АБСОЛЮТНЫЙ ИНДЕКС в
//! этом стеке (см. `Frame::param_aliases`): индекс переживает любой рост
//! `Vec`, а времени жизни хватает, потому что в BSL нельзя сохранить ссылку
//! на переменную за пределы вызова.

/// Компиляция байт-кода в машинный код x86-64. Включается ТОЛЬКО ключом
/// `--jit`: по умолчанию исполнение идёт интерпретатором, и любой отказ
/// JIT-а (неподдержанная инструкция, ядро не дало исполняемую страницу,
/// другая архитектура) молча возвращает на него же.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub(crate) mod jit;

/// На всех прочих платформах JIT-а нет, и `--jit` там просто ничего не
/// меняет. Заглушка, а не `cfg` на каждом месте использования: условная
/// компиляция, размазанная по циклу диспетчеризации, читается хуже.
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
pub(crate) mod jit {
    pub const AVAILABLE: bool = false;
    pub struct CompiledChunk;
    impl CompiledChunk {
        pub(crate) fn run(
            &self,
            _pc: usize,
            _frames: &mut Vec<crate::Frame>,
            _stack: &mut Vec<bsl_rt::BslValue>,
            _program: &bsl_bytecode::Program,
            _runtime_shapes: &mut bsl_rt::RuntimeShapes,
        ) -> Option<Result<usize, bsl_rt::RtError>> {
            None
        }
    }
    pub fn compile(_chunk: &bsl_bytecode::Chunk) -> Option<CompiledChunk> {
        None
    }
}

use bsl_bytecode::{ArgMode, Instr, Program};
use bsl_rt::{BslValue, RtError};
use std::io::Write;

/// Один активный вызов. Регистры кадра не хранятся отдельным `Vec` — все
/// кадры делят один сквозной стек значений (`Vm::stack`), кадр — это лишь
/// окно в него, как в Lua.
struct Frame {
    func_id: usize,
    pc: usize,
    /// Абсолютные индексы в `Vm::stack` для параметров (длина — `n_params`
    /// вызванной функции). Для `Знач`-параметров и для параметров без
    /// `Знач`, но с не-переменным аргументом, это индекс материализованного
    /// значения (временный регистр вызывающего или свежая ячейка). Для
    /// параметров без `Знач` с голой переменной на месте вызова — это
    /// индекс самой переменной вызывающего: чтение/запись слота параметра
    /// напрямую видны вызывающему.
    param_aliases: Vec<usize>,
    /// Абсолютный индекс начала "собственных" регистров кадра (локалы
    /// сверх параметров + временные) — они всегда свежие, только что
    /// вытолкнутые в стек под этот вызов.
    own_base: usize,
    /// Абсолютный индекс, до которого укоротить `Vm::stack` при возврате —
    /// всё, что вызывающий вычислил ДО этого вызова, останется нетронутым;
    /// алиасы параметров, указывающие в более ранние кадры, не пострадают.
    call_start: usize,
    /// Регистр РОДИТЕЛЬСКОГО кадра, куда положить результат при возврате
    /// (не используется для самого нижнего/верхнего кадра).
    return_reg: u8,
    /// Активен только внутри доказанно пустого числового цикла. Его тело не
    /// может наблюдать счётчик, поэтому обычный `BslValue` материализуется
    /// лишь при выходе из цикла.
    numeric_for_state: Option<NumericForState>,
}

struct NumericForState {
    pc: usize,
    current: i64,
    bound: i64,
}

impl Frame {
    #[inline]
    fn reg_index(&self, r: u8) -> usize {
        let r = r as usize;
        if r < self.param_aliases.len() {
            self.param_aliases[r]
        } else {
            self.own_base + (r - self.param_aliases.len())
        }
    }
}

/// Аргументы подавляющего большинства встроенных вызовов помещаются сюда
/// без heap-аллокации. Более длинные вариативные вызовы используют `Vec`.
enum CallArgs {
    Inline { values: [BslValue; 3], len: usize },
    Heap(Vec<BslValue>),
}

impl CallArgs {
    fn load(stack: &[BslValue], frame: &Frame, base: u8, count: u8) -> Result<Self, RtError> {
        if count <= 3 {
            let mut values = [
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
            ];
            for i in 0..count {
                let reg = base.checked_add(i).ok_or(RtError::InvalidBytecode(
                    "переполнение номера регистра аргумента",
                ))?;
                values[i as usize] = reg_load(stack, frame.reg_index(reg))?;
            }
            Ok(CallArgs::Inline {
                values,
                len: count as usize,
            })
        } else {
            let mut values = Vec::with_capacity(count as usize);
            for i in 0..count {
                let reg = base.checked_add(i).ok_or(RtError::InvalidBytecode(
                    "переполнение номера регистра аргумента",
                ))?;
                values.push(reg_load(stack, frame.reg_index(reg))?);
            }
            Ok(CallArgs::Heap(values))
        }
    }

    fn as_slice(&self) -> &[BslValue] {
        match self {
            CallArgs::Inline { values, len } => &values[..*len],
            CallArgs::Heap(values) => values,
        }
    }
}

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
/// # Errors
///
/// Возвращает [`RtError`], если выполнение завершилось неперехваченным исключением или
/// программа содержит некорректный байт-код.
pub fn run_program(program: &Program) -> Result<BslValue, RtError> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    run_program_with_host(
        program,
        CompileEnv::bare(),
        JitMode::Off,
        &mut stdout,
        &mut stderr,
    )
}

/// Исполняет программу с неизменяемым реестром статически подключённых
/// runtime-компонентов.
///
/// # Errors
///
/// До первой инструкции возвращает [`RtError::Component`], если требуемый
/// пакет, версия или код функции отсутствует. Остальные ошибки совпадают с
/// [`run_program`].
pub fn run_program_with_registry(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<BslValue, RtError> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    run_program_with_host(
        program,
        CompileEnv::with_registry(registry),
        JitMode::Off,
        &mut stdout,
        &mut stderr,
    )
}

/// Исполняет программу с выводом в потоки, принадлежащие host-приложению.
/// `Сообщить` пишет только в `stdout`; библиотечный API возвращает ошибки и
/// не печатает их в `stderr` автоматически.
///
/// # Errors
///
/// Возвращает те же ошибки, что [`run_program_with_registry`], включая
/// ошибку записи в пользовательский поток.
pub fn run_program_with_registry_and_io(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    symbols: bsl_syntax::PreprocSymbols,
) -> Result<BslValue, RtError> {
    let env = CompileEnv {
        registry: Some(registry),
        symbols,
    };
    run_program_with_host(program, env, JitMode::Off, stdout, stderr)
}

/// Вариант [`run_program_with_registry`] с включённым JIT.
///
/// # Errors
///
/// Возвращает те же ошибки, что [`run_program_with_registry`].
pub fn run_program_jit_with_registry(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
) -> Result<BslValue, RtError> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    run_program_with_host(
        program,
        CompileEnv::with_registry(registry),
        JitMode::On,
        &mut stdout,
        &mut stderr,
    )
}

/// Исполняет программу с JIT, реестром компонентов и потоками конкретного
/// host-состояния.
///
/// # Errors
///
/// Возвращает те же ошибки, что [`run_program_with_registry_and_io`].
pub fn run_program_jit_with_registry_and_io(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    symbols: bsl_syntax::PreprocSymbols,
) -> Result<BslValue, RtError> {
    let env = CompileEnv {
        registry: Some(registry),
        symbols,
    };
    run_program_with_host(program, env, JitMode::On, stdout, stderr)
}

fn run_program_with_host(
    program: &Program,
    env: CompileEnv<'_>,
    jit_mode: JitMode,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<BslValue, RtError> {
    let mut stack = Vec::new();
    push_own_registers(
        &mut stack,
        at(&program.chunks, 0, "в программе нет чанка верхнего уровня")?,
    );
    let linked = link_components(program, env)?;
    let mut host = HostIo { stdout, stderr };
    let (value, _) = drive_linked(program, 0, stack, jit_mode, &linked, &mut host)?;
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
pub fn run_repl_chunk_with_registry(
    chunk: &bsl_bytecode::Chunk,
    names: Vec<String>,
    shapes: Vec<std::rc::Rc<bsl_rt::Shape>>,
    locals: Vec<String>,
    stack: Vec<BslValue>,
    requirements: Vec<bsl_bytecode::LibraryRequirement>,
    env: CompileEnv<'_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = Program {
        requirements,
        chunks: vec![chunk.clone()],
        names,
        shapes,
        top_level_locals: locals,
        function_names: Vec::new(),
        module_vars: Vec::new(),
        module_base: 0,
    };
    let linked = link_components(&program, env)?;
    let mut host = HostIo {
        stdout: &mut std::io::stdout(),
        stderr: &mut std::io::stderr(),
    };
    drive_linked(&program, 0, stack, JitMode::Off, &linked, &mut host)
}

/// Прогон без реестра — остался входом для собственных тестов VM:
/// production-путь (CLI и фасад) всюду ходит через `*_with_registry*`.
#[cfg(test)]
/// Выполняет `program.chunks[func_id]` с нуля, используя `stack` как
/// начальное содержимое регистров (уже дополненное/подготовленное
/// вызывающим), и возвращает и значение, и финальный стек — нужен
/// `run_isolated` для `Выполнить`/`Вычислить`, которому важно прочитать
/// состояние регистров ПОСЛЕ завершения, а не только значение `Возврат`.
fn drive(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    drive_with(program, func_id, stack, JitMode::Off)
}

struct HostIo<'a> {
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
}

#[derive(Default)]
/// Всё, что нужно, чтобы скомпилировать текст ЗДЕСЬ И СЕЙЧАС: каталог
/// компонентов и символы условной компиляции.
///
/// Эти две вещи всегда ходят вместе, потому что обе описывают не программу,
/// а окружение, в котором её собирают, — и обе нужны фрагменту
/// `Выполнить`/`Вычислить`, который компилируется уже во время исполнения.
#[derive(Clone, Copy)]
pub struct CompileEnv<'a> {
    /// Каталог компонентов. `None` — сборка без реестра.
    pub registry: Option<&'a bsl_rt::RuntimeRegistry>,
    /// Символы условной компиляции. Платформа гасит их все у динамического
    /// кода; здесь фрагмент видит тот же набор, что и модуль вокруг него —
    /// сознательное отступление, см. `docs/bsl-preproc.md`.
    pub symbols: bsl_syntax::PreprocSymbols,
}

impl<'a> CompileEnv<'a> {
    /// Окружение с реестром и набором символов по умолчанию.
    #[must_use]
    pub fn with_registry(registry: &'a bsl_rt::RuntimeRegistry) -> Self {
        CompileEnv {
            registry: Some(registry),
            symbols: bsl_syntax::PreprocSymbols::new(),
        }
    }

    /// Окружение без реестра: только базовый рантайм.
    #[must_use]
    pub fn bare() -> Self {
        CompileEnv {
            registry: None,
            symbols: bsl_syntax::PreprocSymbols::new(),
        }
    }
}

struct LinkedComponents<'a> {
    env: CompileEnv<'a>,
    functions: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    constructors: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    /// Обработчики встроенных методов по номеру имени программы. Открытый
    /// `CallObjectMethod` исполняется миллионами, и поиск по строке на
    /// каждом вызове недопустим: измерено флеймграфом, до трёх четвертей
    /// времени `csv_write` уходило в `to_uppercase` внутри
    /// `BuiltinMethod::lookup`. Таблица строится один раз при связывании.
    builtin_methods: Vec<Option<bsl_rt::BuiltinMethod>>,
    /// Мемоизация «(статическая таблица типа, номер имени) → обработчик»
    /// для компонентных объектов со статическими таблицами методов
    /// (см. `ObjectProtocol::method_table`): разрешение по строке
    /// происходит один раз на пару, дальше — целочисленный поиск по хешу.
    /// `None` запоминает и промахи, чтобы типы без таблиц не платили за
    /// строку на каждом вызове. Рантайм однопоточный, `RefCell` достаточно.
    component_methods: ComponentMethodMap,
    /// То же для статических таблиц СВОЙСТВ (`ObjectProtocol::property_table`).
    /// Ячейки инструкции у свойств пока нет: она — производная таблица
    /// `Chunk`, её добавление стоит правок формата байт-кода, а выигрыш не
    /// измерен.
    component_properties: ComponentPropertyMap,
}

impl LinkedComponents<'_> {
    /// Типы всех библиотек реестра: список короткий (десятки записей на
    /// всю сборку) и строится один раз на прогон.
    fn component_types(&self) -> Vec<&'static bsl_rt::TypeDescriptor> {
        self.env
            .registry
            .map(|registry| registry.types().collect())
            .unwrap_or_default()
    }

    fn function(&self, func_id: usize, pc: usize) -> Result<bsl_rt::ComponentCall, RtError> {
        self.functions
            .get(func_id)
            .and_then(|chunk| chunk.get(pc))
            .and_then(|slot| *slot)
            .ok_or(RtError::InvalidBytecode(
                "CallComponent не связан с функцией реестра",
            ))
    }

    fn constructor(&self, func_id: usize, pc: usize) -> Result<bsl_rt::ComponentCall, RtError> {
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
    fn builtin_method(&self, name: bsl_rt::NameId) -> Option<bsl_rt::BuiltinMethod> {
        self.builtin_methods.get(name.index()).copied().flatten()
    }
}

/// Карта мемоизации «(статическая таблица типа, номер имени) → обработчик».
type ComponentMethodMap =
    std::cell::RefCell<std::collections::HashMap<(usize, u32), Option<bsl_rt::MethodCall>>>;

/// Разрешение метода компонентного объекта по статической таблице типа и
/// номеру имени. Строка разбирается один раз на пару «таблица, имя»;
/// установившийся режим — поиск по хешу от двух целых, промахи тоже
/// запоминаются. `None` — имени в таблице нет (или таблица пустая):
/// вызывающий уходит в строковый `call_method`, чтобы текст ошибки остался
/// одним, у самого типа. Свободная функция, а не метод: тем же разрешением
/// пользуется шим открытого метода в JIT, у которого карта приходит сырым
/// указателем из `JitCtx`.
/// Карта мемоизации «(статическая таблица типа, номер имени) → пара
/// обработчиков свойства».
type ComponentPropertyMap = std::cell::RefCell<
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
fn component_prop_get(
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
fn component_prop_set(
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

fn resolve_component_method(
    map: &ComponentMethodMap,
    table: &'static [bsl_rt::MethodDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<bsl_rt::MethodCall>, RtError> {
    let key = (table.as_ptr() as usize, name.index() as u32);
    if let Some(resolved) = map.borrow().get(&key) {
        return Ok(*resolved);
    }
    let upper = field_name(program, name)?.to_uppercase();
    let resolved = table
        .iter()
        .find(|descriptor| {
            descriptor
                .names
                .iter()
                .any(|candidate| candidate.to_uppercase() == upper)
        })
        .map(|descriptor| descriptor.call);
    map.borrow_mut().insert(key, resolved);
    Ok(resolved)
}

fn link_components<'a>(
    program: &Program,
    env: CompileEnv<'a>,
) -> Result<LinkedComponents<'a>, RtError> {
    let registry = env.registry;
    let Some(core) = program.requirements.first() else {
        return Err(RtError::Component(
            "в требованиях отсутствует bsl-rt".to_string(),
        ));
    };
    if core.package != bsl_rt::PACKAGE_NAME || core.version != bsl_rt::PACKAGE_VERSION {
        return Err(RtError::Component(format!(
            "необходим {}={}, исполнитель предоставляет {}={}",
            core.package,
            core.version,
            bsl_rt::PACKAGE_NAME,
            bsl_rt::PACKAGE_VERSION
        )));
    }

    for requirement in &program.requirements[1..] {
        let Some(registry) = registry else {
            return Err(RtError::Component(format!(
                "необходим пакет {}={}, но реестр компонентов не предоставлен",
                requirement.package, requirement.version
            )));
        };
        let Some(library) = registry.library_by_package(&requirement.package) else {
            return Err(RtError::Component(format!(
                "необходим пакет {}={}, но он не зарегистрирован",
                requirement.package, requirement.version
            )));
        };
        if library.version != requirement.version {
            return Err(RtError::Component(format!(
                "для {} требуется {}, зарегистрирована версия {}",
                requirement.package, requirement.version, library.version
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
                        return Err(RtError::Component(format!(
                            "функция {}/{} требует реестр компонентов",
                            requirement.package, function
                        )));
                    };
                    let library_descriptor = registry
                        .library_by_package(&requirement.package)
                        .ok_or_else(|| {
                            RtError::Component(format!(
                                "необходим пакет {}={}, но он не зарегистрирован",
                                requirement.package, requirement.version
                            ))
                        })?;
                    let descriptor = library_descriptor
                        .functions
                        .iter()
                        .find(|descriptor| descriptor.code.get() == *function)
                        .ok_or_else(|| {
                            RtError::Component(format!(
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
                    let Some(registry) = registry else {
                        return Err(RtError::Component(format!(
                            "конструктор {}/{} требует реестр компонентов",
                            requirement.package, constructor
                        )));
                    };
                    let library_descriptor = registry
                        .library_by_package(&requirement.package)
                        .ok_or_else(|| {
                            RtError::Component(format!(
                                "необходим пакет {}={}, но он не зарегистрирован",
                                requirement.package, requirement.version
                            ))
                        })?;
                    let descriptor = library_descriptor
                        .constructors
                        .iter()
                        .find(|descriptor| descriptor.code.get() == *constructor)
                        .ok_or_else(|| {
                            RtError::Component(format!(
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
        env,
        functions,
        constructors,
        builtin_methods,
        component_methods: std::cell::RefCell::new(std::collections::HashMap::new()),
        component_properties: std::cell::RefCell::new(std::collections::HashMap::new()),
    })
}

/// Предел числа одновременно активных кадров BSL. Кадры лежат в куче
/// (`Vec<Frame>` плюс общий стек значений), поэтому без предела
/// бесконечная рекурсия не падает, а молча растит память до OOM — процесс
/// умирает без диагностики и без шанса на `Попытка`. С пределом это
/// перехватываемая [`RtError::StackOverflow`].
// НЕ ИЗМЕРЕНО(EXEC.MAX_CALL_DEPTH) — какую глубину рекурсии допускает
// платформа и какой ошибкой отвечает на превышение; замер даёт только
// нижнюю границу (900 уровней обязаны работать).
const MAX_CALL_DEPTH: usize = 1000;

/// Предел вложенности `Выполнить`/`Вычислить` друг в друге. В отличие от
/// кадров BSL, каждый уровень динамического кода — это настоящий вложенный
/// `drive` на стеке Rust (плюс разбор и компиляция фрагмента), поэтому
/// предел защищает стек процесса, а не память: без него рекурсия через
/// `Выполнить` валит процесс переполнением стека, минуя `Попытка`.
// НЕ ИЗМЕРЕНО(EXEC.DYNAMIC_DEPTH) — сколько уровней допускает платформа;
// замер даёт только нижнюю границу (40 уровней обязаны работать).
const MAX_DYNAMIC_DEPTH: usize = 64;

thread_local! {
    /// Текущая вложенность динамических фрагментов этого потока. Счётчик
    /// потоковый, а не поле VM: вложенный `drive` создаётся заново на
    /// каждый фрагмент, и общего состояния, через которое уровень можно
    /// было бы протащить, у них нет.
    static DYNAMIC_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Вход в очередной уровень динамического кода; выход — в `Drop`, чтобы
/// счётчик не съезжал ни на одном из путей ошибки.
struct DynamicDepthGuard;

impl DynamicDepthGuard {
    fn enter() -> Result<Self, RtError> {
        DYNAMIC_DEPTH.with(|d| {
            if d.get() >= MAX_DYNAMIC_DEPTH {
                Err(RtError::StackOverflow {
                    what: "слишком глубокая вложенность Выполнить/Вычислить",
                })
            } else {
                d.set(d.get() + 1);
                Ok(DynamicDepthGuard)
            }
        })
    }
}

impl Drop for DynamicDepthGuard {
    fn drop(&mut self) {
        DYNAMIC_DEPTH.with(|d| d.set(d.get() - 1));
    }
}

/// Включён ли JIT. Отдельный тип, а не `bool`: у вызова `drive(.., true)`
/// на месте вызова не видно, что именно включается.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JitMode {
    Off,
    On,
}

#[cfg(test)]
fn drive_with(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
    jit_mode: JitMode,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let linked = link_components(program, CompileEnv::bare())?;
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
    };
    drive_linked(program, func_id, stack, jit_mode, &linked, &mut host)
}

fn drive_linked(
    program: &Program,
    func_id: usize,
    mut stack: Vec<BslValue>,
    jit_mode: JitMode,
    linked: &LinkedComponents,
    host: &mut HostIo<'_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let mut frames = vec![Frame {
        func_id,
        pc: 0,
        param_aliases: Vec::new(),
        own_base: 0,
        call_start: 0,
        return_reg: 0,
        numeric_for_state: None,
    }];
    let mut current_exception: Option<BslValue> = None;
    // Кэш скомпилированных фрагментов `Выполнить`/`Вычислить` на всё это
    // исполнение — см. `SnippetCache`.
    let mut snippets: SnippetCache = SnippetCache::new();
    // Затравлена формами/именами ЭТОЙ программы — см. `bsl_rt::RuntimeShapes`
    // doc comment про то, почему не общий на процесс синглтон: у вложенного
    // `Program` (см. `run_dynamic_snippet`) свои `names`/`shapes`, и рантайм-
    // расширения этой таблицы (`Вставить`/`Удалить` на структуре, меняющие
    // её форму) актуальны только для объектов внутри ОДНОГО такого вызова.
    let mut runtime_shapes =
        bsl_rt::RuntimeShapes::seeded(program.names.clone(), program.shapes.clone());
    // Типы, объявленные компонентами этого прогона: по ним `Тип("Имя")`
    // находит то, чего нет в закрытом реестре ядра (см. `TypeRef`).
    runtime_shapes.component_types = linked.component_types();
    // Скомпилированные чанки. Внешний `None` — «ещё не пробовали»,
    // внутренний — «пробовали, JIT отказался»: компилировать чанк заново
    // на каждом входе в него стоило бы дороже любого выигрыша.
    let mut native: Vec<Option<Option<jit::CompiledChunk>>> =
        if jit_mode == JitMode::On && jit::AVAILABLE {
            (0..program.chunks.len()).map(|_| None).collect()
        } else {
            Vec::new()
        };
    // Без JIT `step` может сцеплять линейные цепочки бандлов, не
    // возвращаясь сюда: единственная оставшаяся проба — fast numeric-for,
    // а её `pc` достижим только взятым back-edge, на котором цепочка и так
    // рвётся. С JIT-ом же возврат нужен после каждого бандла — иначе
    // интерпретатор пробежит мимо позиции, где мог бы стартовать натив.
    let merge_linear = native.is_empty();

    loop {
        // После инициализации пустой numeric-for не обращается к
        // регистрам. Обслуживаем его back-edge в компактном внешнем цикле,
        // не входя на каждой итерации в большой универсальный `step`.
        // Логических итераций по-прежнему столько же: цикл не сворачивается
        // в вычисление финального значения.
        let fast_numeric_for = {
            let frame = frames
                .last_mut()
                .expect("инвариант VM: drive всегда держит хотя бы один кадр");
            match frame.numeric_for_state.as_mut() {
                Some(state) if state.pc == frame.pc => match state.current.checked_add(1) {
                    Some(next) if next <= state.bound => {
                        state.current = next;
                        true
                    }
                    _ => false,
                },
                _ => false,
            }
        };
        if fast_numeric_for {
            continue;
        }

        // Нативный путь. Он не обязан ничего исполнить: если на текущей
        // позиции входа нет, управление просто идёт в `step`, и это же
        // происходит при любом отказе JIT-а.
        if !native.is_empty() {
            let (fid, pc) = {
                let frame = frames
                    .last()
                    .expect("инвариант VM: drive всегда держит хотя бы один кадр");
                (frame.func_id, frame.pc)
            };
            if let Some(slot) = native.get_mut(fid) {
                if slot.is_none() {
                    *slot = Some(program.chunks.get(fid).and_then(jit::compile));
                }
                // Два слоя внутри уже найденной ячейки: пробовали ли этот
                // чанк и вышло ли. Повторно искать его в таблице незачем.
                if let Some(Some(code)) = slot.as_ref()
                    && let Some(outcome) = code.run(
                        pc,
                        &mut frames,
                        &mut stack,
                        program,
                        &mut runtime_shapes,
                        linked,
                    )
                {
                    match outcome {
                        Ok(next_pc) => {
                            if let Some(frame) = frames.last_mut() {
                                frame.pc = next_pc;
                            }
                            continue;
                        }
                        Err(e) => {
                            if !unwind_to_handler(
                                &mut frames,
                                &mut stack,
                                program,
                                &e,
                                &mut current_exception,
                            ) {
                                return Err(e);
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // `step` исполняет целый VLIW-бандл (см. `bsl_bytecode::bundle`),
        // так что проверки fast numeric-for и JIT-входа выше происходят на
        // границах бандлов, а не на каждой инструкции. При ошибке члена
        // `pc` стоит на нём самом, и `unwind_to_handler` находит обработчик
        // как при поинструкционном исполнении; обработчик по построению
        // разметки — начало бандла.
        match step(
            &mut frames,
            &mut stack,
            program,
            &mut current_exception,
            &mut runtime_shapes,
            &mut snippets,
            linked,
            host,
            merge_linear,
        ) {
            Ok(Step::Continue) => continue,
            Ok(Step::Done(v)) => return Ok((v, stack)),
            Err(e) => {
                if !unwind_to_handler(&mut frames, &mut stack, program, &e, &mut current_exception)
                {
                    return Err(e);
                }
                // Иначе кадры/pc уже поправлены внутри unwind_to_handler —
                // просто продолжаем цикл со следующей итерации.
            }
        }
    }
}

enum Step {
    Continue,
    Done(BslValue),
}

// --- Доступ к байт-коду и регистрам ------------------------------------
//
// Ни одна паника в этом модуле не должна зависеть от ВХОДНЫХ ДАННЫХ. Всё,
// что инструкция читает по индексу — номер регистра, номер чанка,
// константы, формы, имени, аргумент builtin'а, — приходит из `Program`, а
// `Program` VM получает не только от собственного кодогена: её собирает
// `Выполнить`/`Вычислить` в рантайме, REPL `bsl-cli` по строке, и любой
// внешний пользователь публичных `run_program` и
// `run_repl_chunk_with_registry`. Поэтому такие обращения дают
// `RtError::InvalidBytecode`, а не роняют процесс.
//
// Остаются ровно два `expect()` — на `frames.pop()` в
// `do_return_with_value` и `unwind_to_handler`: это внутренние инварианты
// самого цикла диспетчеризации (стек кадров непуст, пока мы исполняем
// инструкцию), недостижимые никаким байт-кодом. Голых `unwrap()` вне
// тестов нет ни одного.

// Три следующие функции помечены `#[inline(always)]`, а не подсказкой
// `#[inline]`, намеренно. Это самые горячие операции цикла диспетчеризации,
// и решение инлайнера по ним оплачивает не тот код, который его сдвинул:
// коммит, добавивший к `match` в `step` один опкод для JSON, перевесил
// бюджет инлайнера, `reg_load` выехал наружу — и `pi_leibniz`, ни о каком
// JSON не знающий, замедлился на десятую часть (2,98 -> 3,16 млрд
// инструкций, 11% времени в вызовах `reg_load`). Пока диспетчер — одна
// огромная функция, размер которой меняется с каждым новым опкодом, такую
// связь надо разрывать явно.
#[inline(always)]
fn at<'a, T>(xs: &'a [T], i: usize, what: &'static str) -> Result<&'a T, RtError> {
    xs.get(i).ok_or(RtError::InvalidBytecode(what))
}

#[inline(always)]
fn reg_load(stack: &[BslValue], i: usize) -> Result<BslValue, RtError> {
    stack.get(i).cloned().ok_or(RtError::InvalidBytecode(
        "чтение регистра за границей стека значений",
    ))
}

#[inline(always)]
fn reg_store(stack: &mut [BslValue], i: usize, v: BslValue) -> Result<(), RtError> {
    match stack.get_mut(i) {
        Some(slot) => {
            *slot = v;
            Ok(())
        }
        None => Err(RtError::InvalidBytecode(
            "запись регистра за границей стека значений",
        )),
    }
}

/// Одновременно заимствует изменяемый счётчик и неизменяемую границу без
/// клонирования `BslValue` на каждой итерации числового цикла.
#[inline]
fn reg_pair_mut(
    stack: &mut [BslValue],
    mutable: usize,
    other: usize,
) -> Result<(&mut BslValue, &BslValue), RtError> {
    if mutable == other {
        return Err(RtError::InvalidBytecode(
            "счётчик и граница числового цикла используют один регистр",
        ));
    }
    if mutable >= stack.len() {
        return Err(RtError::InvalidBytecode(
            "счётчик числового цикла вне стека значений",
        ));
    }
    if other >= stack.len() {
        return Err(RtError::InvalidBytecode(
            "граница числового цикла вне стека значений",
        ));
    }
    if mutable < other {
        let (left, right) = stack.split_at_mut(other);
        Ok((&mut left[mutable], &right[0]))
    } else {
        let (left, right) = stack.split_at_mut(mutable);
        Ok((&mut right[0], &left[other]))
    }
}

/// Первый заход в `Instr::NumericForNextI64`: счётчик и граница ещё не
/// сняты в `i64`. Отдельной функцией с `#[inline(never)]`, потому что на
/// цикл это выполняется однажды, а код занимает место ровно там, где
/// крутится тело цикла — а горячий путь диспетчера живёт на грани кеша
/// микроопераций (см. комментарий у `step_cold`).
///
/// `Ok(None)` означает, что цикл не уложился в `i64` и уже отработан
/// общим путём `numeric_for_next_regular`.
///
/// # Errors
///
/// Возвращает ошибку чтения регистра за границей стека значений или
/// ошибку общего пути.
#[inline(never)]
fn numeric_for_i64_start(
    stack: &mut [BslValue],
    counter_idx: usize,
    bound_idx: usize,
    pc: usize,
    frame_pc: &mut usize,
    target: i16,
) -> Result<Option<NumericForState>, RtError> {
    let counter_value = reg_load(stack, counter_idx)?;
    let bound_value = reg_load(stack, bound_idx)?;
    let pair = match (&counter_value, &bound_value) {
        (BslValue::Number(counter), BslValue::Number(bound)) => {
            counter.to_i64_exact().zip(bound.to_i64_exact())
        }
        _ => None,
    };
    let Some((current, bound)) = pair else {
        numeric_for_next_regular(stack, counter_idx, bound_idx, frame_pc, target)?;
        return Ok(None);
    };
    Ok(Some(NumericForState { pc, current, bound }))
}

/// Переполнение `i64` на инкременте счётчика: цикл дошёл до `i64::MAX` и
/// дальше считается общим путём. Вынесено по той же причине, что и
/// [`numeric_for_i64_start`], — недостижимый на практике код не должен
/// занимать место в теле горячего цикла.
///
/// # Errors
///
/// Возвращает ошибку записи регистра за границей стека значений или
/// ошибку общего пути.
#[inline(never)]
fn numeric_for_i64_overflow(
    stack: &mut [BslValue],
    counter_idx: usize,
    bound_idx: usize,
    current: i64,
    frame_pc: &mut usize,
    target: i16,
) -> Result<(), RtError> {
    reg_store(
        stack,
        counter_idx,
        BslValue::Number(bsl_number::BslNumber::from_i64(current)),
    )?;
    numeric_for_next_regular(stack, counter_idx, bound_idx, frame_pc, target)
}

#[inline]
fn numeric_for_next_regular(
    stack: &mut [BslValue],
    counter: usize,
    bound: usize,
    pc: &mut usize,
    target: i16,
) -> Result<(), RtError> {
    let (counter, bound) = reg_pair_mut(stack, counter, bound)?;
    if counter.increment_numeric_for_and_le(bound)? {
        *pc = target as usize;
    } else {
        *pc += 1;
    }
    Ok(())
}

/// Ячейка инлайн-кэша, отведённая под инструкцию на позиции `pc`.
/// `prop_cache` кодоген заводит длиной со всем `instrs`, но чанк мог
/// прийти и не от него.
#[inline]
fn prop_cache(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
) -> Result<&bsl_bytecode::PropCacheSlot, RtError> {
    at(
        &chunk.prop_cache,
        pc,
        "нет ячейки инлайн-кэша для инструкции",
    )
}

/// Ячейка инлайн-кэша `CallObjectMethod` на позиции `pc` — см.
/// [`cached_component_method`].
#[inline]
fn method_cache(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
) -> Result<&bsl_bytecode::MethodCacheSlot, RtError> {
    at(
        &chunk.method_cache,
        pc,
        "нет ячейки кэша метода для инструкции",
    )
}

/// Разрешение метода компонентного объекта с кэшем на позиции инструкции:
/// мономорфный сайт после первого вызова читает обработчик из своей ячейки
/// по одному сравнению адреса таблицы, не трогая карту мемоизации. Смена
/// типа получателя на том же сайте (полиморфизм) перечитывает карту и
/// перезаписывает ячейку; `None` кэшируется наравне с попаданием — тип без
/// имени в таблице не платит за строку и хэш на каждый вызов.
fn cached_component_method(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
    map: &ComponentMethodMap,
    table: &'static [bsl_rt::MethodDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<bsl_rt::MethodCall>, RtError> {
    let slot = method_cache(chunk, pc)?;
    let key = table.as_ptr() as usize;
    if let Some((cached_table, resolved)) = *slot.borrow()
        && cached_table == key
    {
        return Ok(resolved);
    }
    let resolved = resolve_component_method(map, table, name, program)?;
    *slot.borrow_mut() = Some((key, resolved));
    Ok(resolved)
}

/// Оригинальное написание имени поля — нужно строковому пути доступа
/// (`СтрокаТаблицыЗначений`, `КлючИЗначение`), у которого нет формы.
#[inline]
fn field_name(program: &Program, name: bsl_rt::NameId) -> Result<&str, RtError> {
    at(
        &program.names,
        name.index(),
        "идентификатор имени вне таблицы имён программы",
    )
    .map(|s| s.as_str())
}

/// Выполняет один VLIW-бандл текущего (верхнего) кадра: от одной
/// инструкции (одиночный бандл) до `Chunk::bundle_len[pc]` подряд — без
/// возврата в `drive_with` между членами. При `merge_linear` (чисто
/// интерпретаторный режим, без JIT) исполнение продолжается и через
/// границу бандла, пока `pc` идёт линейно: пробы `drive_with` имеют смысл
/// только там, куда `pc` попадает переходом, вызовом или разматыванием.
#[allow(clippy::too_many_arguments)]
fn step(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    current_exception: &mut Option<BslValue>,
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    snippets: &mut SnippetCache,
    linked: &LinkedComponents,
    host: &mut HostIo<'_>,
    merge_linear: bool,
) -> Result<Step, RtError> {
    let frame_idx = frames.len() - 1;
    let func_id = frames[frame_idx].func_id;
    let mut pc = frames[frame_idx].pc;
    let chunk = at(&program.chunks, func_id, "номер чанка вне таблицы функций")?;

    if pc >= chunk.instrs.len() {
        // Неявный возврат: тело кончилось без `Возврат` — результат
        // Неопределено, как и `Возврат;` без выражения.
        return Ok(
            match do_return_with_value(frames, stack, BslValue::Undefined)? {
                Done(v) => Step::Done(v),
                Continuing => Step::Continue,
            },
        );
    }

    // Ширина VLIW-бандла на этой позиции (см. `bsl_bytecode::bundle`).
    // Члены бандла взаимно независимы, передача управления бывает только
    // хвостовым членом, поэтому кадр и чанк между членами можно не
    // перечитывать: `Call` и `Return` дальше хвоста не встречаются, а
    // `Return` к тому же выходит из функции ранним `return`. Ноль
    // (середина бандла — сюда возвращается JIT, отказавшийся от
    // инструкции) и пустая таблица равнозначны одиночному исполнению.
    // Разметке можно верить, потому что из файла она не читается — её
    // всегда пересчитывает `bundle::compute`; ошибка члена оставляет `pc`
    // на нём самом, и `Попытка` ищется ровно как при поинструкционном
    // исполнении.
    // «Сколько членов ЗА первым»: у одиночного бандла и в середине бандла
    // ноль — путь обычной инструкции оплачивает ровно одну загрузку `u8`
    // и вычитание, вся петлевая бухгалтерия лежит после исполнения члена.
    let mut extra = chunk
        .bundle_len
        .get(pc)
        .copied()
        .unwrap_or(1)
        .saturating_sub(1);
    loop {
        // `pc < chunk.instrs.len()` проверено выше (для последующих
        // членов — перед переходом на них) — индексация здесь уже не
        // может выйти за границы.
        let instr = chunk.instrs[pc];
        match instr {
            Instr::GetModuleVar { dst, slot } => {
                // АБСОЛЮТНЫЙ индекс: модульные переменные лежат подряд,
                // начиная с `module_base`. У обычной программы база нулевая
                // — это первые слоты кадра верхнего уровня, а он стоит в
                // самом низу стека и живёт всё исполнение. Проверка границы
                // обязательна — байт-код может прийти и не от кодогена.
                if (slot as usize) >= program.module_vars.len() {
                    return Err(RtError::InvalidBytecode(
                        "номер переменной модуля вне таблицы",
                    ));
                }
                let v = reg_load(stack, program.module_base as usize + slot as usize)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetModuleVar { slot, src } => {
                if (slot as usize) >= program.module_vars.len() {
                    return Err(RtError::InvalidBytecode(
                        "номер переменной модуля вне таблицы",
                    ));
                }
                let v = reg_load(stack, frames[frame_idx].reg_index(src))?;
                reg_store(stack, program.module_base as usize + slot as usize, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Move { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = reg_load(stack, s)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadConst { dst, k } => {
                let v = at(
                    &chunk.consts,
                    k as usize,
                    "номер константы вне таблицы констант чанка",
                )?
                .clone();
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadBool { dst, val } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Boolean(val))?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadUndefined { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Undefined)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadNull { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Null)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadSkipped { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Skipped)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Add { dst, a, b } => {
                add_op(frames, stack, frame_idx, dst, a, b)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Sub { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::sub)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Mul { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::mul)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Mod { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::rem)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Div { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::div)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Neg { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = neg_op(&reg_load(stack, s)?)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Not { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = reg_load(stack, s)?.not()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Eq { dst, a, b } => {
                let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
                let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Boolean(av.eq_value(&bv)))?;
                frames[frame_idx].pc += 1;
            }
            Instr::NotEq { dst, a, b } => {
                let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
                let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Boolean(!av.eq_value(&bv)))?;
                frames[frame_idx].pc += 1;
            }
            Instr::Lt { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, "<", |o| o.is_lt())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Gt { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, ">", |o| o.is_gt())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Le { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, "<=", |o| o.is_le())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Ge { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, ">=", |o| o.is_ge())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Jump { target } => {
                frames[frame_idx].pc = target as usize;
            }
            Instr::JumpIfFalse { cond, target } => {
                let c = frames[frame_idx].reg_index(cond);
                // Строгая булевость: не-`Булево` в условии — ошибка типа,
                // а не приведение к истинности.
                if reg_load(stack, c)?.as_condition()? {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::JumpIfTrue { cond, target } => {
                let c = frames[frame_idx].reg_index(cond);
                if reg_load(stack, c)?.as_condition()? {
                    frames[frame_idx].pc = target as usize;
                } else {
                    frames[frame_idx].pc += 1;
                }
            }
            Instr::JumpIfNotSkipped { src, target } => {
                let s = frames[frame_idx].reg_index(src);
                // Не `as_condition`/строгая булевость — это не условие
                // пользовательского кода, а проверка внутреннего маркера
                // (см. пролог параметров по умолчанию в
                // `bsl-bytecode::compiler::compile_param_defaults`).
                if matches!(reg_load(stack, s)?, BslValue::Skipped) {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::NumericForNext {
                counter,
                bound,
                target,
            } => {
                let counter = frames[frame_idx].reg_index(counter);
                let bound = frames[frame_idx].reg_index(bound);
                numeric_for_next_regular(stack, counter, bound, &mut frames[frame_idx].pc, target)?;
            }
            Instr::NumericForNextI64 {
                counter,
                bound,
                target,
            } => {
                let counter_idx = frames[frame_idx].reg_index(counter);
                let bound_idx = frames[frame_idx].reg_index(bound);
                let state = match frames[frame_idx].numeric_for_state.take() {
                    Some(state) if state.pc == pc => state,
                    Some(_) => {
                        return Err(RtError::InvalidBytecode(
                            "перекрывающиеся скрытые состояния числовых циклов",
                        ));
                    }
                    None => {
                        match numeric_for_i64_start(
                            stack,
                            counter_idx,
                            bound_idx,
                            pc,
                            &mut frames[frame_idx].pc,
                            target,
                        )? {
                            Some(state) => state,
                            None => return Ok(Step::Continue),
                        }
                    }
                };

                let Some(next) = state.current.checked_add(1) else {
                    numeric_for_i64_overflow(
                        stack,
                        counter_idx,
                        bound_idx,
                        state.current,
                        &mut frames[frame_idx].pc,
                        target,
                    )?;
                    return Ok(Step::Continue);
                };
                if next <= state.bound {
                    frames[frame_idx].numeric_for_state = Some(NumericForState {
                        current: next,
                        ..state
                    });
                    frames[frame_idx].pc = target as usize;
                } else {
                    reg_store(
                        stack,
                        counter_idx,
                        BslValue::Number(bsl_number::BslNumber::from_i64(next)),
                    )?;
                    frames[frame_idx].pc += 1;
                }
            }
            Instr::Call {
                func,
                base,
                arg_modes,
                ret,
            } => {
                // Проверка глубины — ДО продвижения `pc` и до любых записей
                // в стек: в момент ошибки `pc` обязан стоять на сбойнувшей
                // инструкции, иначе `Попытка`, у которой этот `Call` —
                // последняя инструкция защищённого диапазона, его не поймает.
                if frames.len() >= MAX_CALL_DEPTH {
                    return Err(RtError::StackOverflow {
                        what: "слишком глубокая рекурсия вызовов",
                    });
                }

                // Caller продвигается ЗА инструкцию Call сейчас — так, когда
                // callee вернётся, мы продолжим ровно со следующей.
                frames[frame_idx].pc += 1;

                let modes = at(
                    &chunk.call_arg_modes,
                    arg_modes as usize,
                    "номер набора режимов аргументов вне таблицы чанка",
                )?;
                let mut param_aliases = Vec::with_capacity(modes.len());
                for (i, mode) in modes.iter().enumerate() {
                    let idx = match mode {
                        ArgMode::Value => frames[frame_idx].reg_index(base + i as u8),
                        ArgMode::ByRefLocal(slot) => frames[frame_idx].reg_index(*slot),
                    };
                    param_aliases.push(idx);
                }

                let callee_chunk = at(
                    &program.chunks,
                    func as usize,
                    "номер вызываемого чанка вне таблицы функций",
                )?;
                let call_start = stack.len();
                let own_base = stack.len();
                push_own_registers(stack, callee_chunk);

                frames.push(Frame {
                    func_id: func as usize,
                    pc: 0,
                    param_aliases,
                    own_base,
                    call_start,
                    return_reg: ret,
                    numeric_for_state: None,
                });
            }
            Instr::Return { src } => {
                let value = match src {
                    Some(r) => {
                        let idx = frames[frame_idx].reg_index(r);
                        reg_load(stack, idx)?
                    }
                    None => BslValue::Undefined,
                };
                return Ok(match do_return_with_value(frames, stack, value)? {
                    Done(v) => Step::Done(v),
                    Continuing => Step::Continue,
                });
            }
            Instr::GetIndex { dst, obj, idx } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let iv = reg_load(stack, frames[frame_idx].reg_index(idx))?;
                let v = ov.get_index(&iv, &runtime_shapes.names)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetIndex { obj, idx, src } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let iv = reg_load(stack, frames[frame_idx].reg_index(idx))?;
                let sv = reg_load(stack, frames[frame_idx].reg_index(src))?;
                ov.set_index(&iv, sv)?;
                frames[frame_idx].pc += 1;
            }
            Instr::GetProp { dst, obj, name } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                // Структура резолвится через инлайн-кэш этой ИНСТРУКЦИИ
                // (см. Chunk::prop_cache): мономорфный сайт вызова после
                // первого попадания читает слот напрямую, без HashMap-
                // поиска в Shape::index. СтрокаТаблицыЗначений заводит
                // колонки в рантайме и не могла быть интернирована на
                // этапе компиляции — для неё (и только когда кэш-путь
                // говорит "это не такой объект") VM резолвит имя в текст
                // через Program::names и идёт по строковому пути.
                let v = if let Some(object) = ov.object_ref() {
                    let mut context = bsl_rt::CallContext::new(
                        runtime_shapes,
                        &mut *host.stdout,
                        &mut *host.stderr,
                        bsl_format::format_value,
                    );
                    component_prop_get(
                        object,
                        &linked.component_properties,
                        name,
                        program,
                        &mut context,
                    )?
                } else {
                    match ov.get_field_cached(name, prop_cache(chunk, pc)?) {
                        Err(RtError::NotAnObject) => {
                            ov.get_field_by_name(field_name(program, name)?)?
                        }
                        other => other?,
                    }
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetProp { obj, name, src } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let sv = reg_load(stack, frames[frame_idx].reg_index(src))?;
                if let Some(object) = ov.object_ref() {
                    let mut context = bsl_rt::CallContext::new(
                        runtime_shapes,
                        &mut *host.stdout,
                        &mut *host.stderr,
                        bsl_format::format_value,
                    );
                    component_prop_set(
                        object,
                        &linked.component_properties,
                        name,
                        sv,
                        program,
                        &mut context,
                    )?;
                } else {
                    let имя = field_name(program, name)?;
                    match ov.set_field_cached(name, sv.clone(), prop_cache(chunk, pc)?) {
                        Err(RtError::NotAnObject) => ov.set_field_by_name(имя, sv)?,
                        other => other?,
                    }
                }
                frames[frame_idx].pc += 1;
            }
            Instr::CallBuiltin {
                dst,
                builtin,
                base,
                count,
            } => {
                let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                let v = call_builtin_with_format(builtin, args.as_slice(), runtime_shapes, host)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::CallMethod {
                dst,
                obj,
                method,
                base,
                count,
            } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                let v = if let Some(object) = ov.object_ref() {
                    let mut context = bsl_rt::CallContext::new(
                        runtime_shapes,
                        &mut *host.stdout,
                        &mut *host.stderr,
                        bsl_format::format_value,
                    );
                    object.call_method(method.primary_name(), args.as_slice(), &mut context)?
                } else {
                    bsl_rt::call_builtin_method_ctx(method, &ov, args.as_slice(), runtime_shapes)?
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::WriteText { dst, obj, src } => {
                let obj_idx = frames[frame_idx].reg_index(obj);
                let src_idx = frames[frame_idx].reg_index(src);
                let v = {
                    let ov = at(stack, obj_idx, "чтение объекта за границей стека значений")?;
                    let sv = at(
                        stack,
                        src_idx,
                        "чтение аргумента за границей стека значений",
                    )?;
                    // `Записать` полиморфен по получателю, а эта инструкция —
                    // быстрый путь `ЗаписьТекста` в обход вызова метода.
                    // Значит развести получателей надо и здесь, иначе
                    // `ТекстовыйДокумент.Записать(путь)` попадёт в чужую
                    // ветку и получит «метод не применим».
                    if let Some(object) = ov.object_ref() {
                        let mut context = bsl_rt::CallContext::new(
                            runtime_shapes,
                            &mut *host.stdout,
                            &mut *host.stderr,
                            bsl_format::format_value,
                        );
                        object.call_method("Записать", std::slice::from_ref(sv), &mut context)?
                    } else {
                        ov.text_writer_write(sv)?
                    }
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            // Холодные опкоды: конструирование объектов, возбуждение
            // исключения, закрытие файла и динамическое исполнение. Тела
            // вынесены в `step_cold`, чтобы код диспетчера не разъезжался
            // по памяти вокруг горячего пути: он живёт на грани кеша
            // микроопераций процессора, и лишние окна на пути стоят до
            // полутора раз при том же числе исполненных инструкций.
            //
            // Граница проведена по стоимости тела, а не по редкости
            // опкода: `CallBuiltin`, `CallMethod` и `WriteText`
            // остались здесь, потому что исполняются миллионами и лишний
            // вызов на каждый стоит дороже, чем занятое ими место (измерено
            // — `csv_write` теряет 7%, если унести и их). Открытый
            // `CallObjectMethod` встречается только у компонентных методов.
            //
            // Опкоды перечислены поимённо, а не `_`, чтобы `match`
            // остался исчерпывающим и новый опкод по-прежнему ломал
            // сборку, пока его не расклассифицируют.
            Instr::NewArray { .. }
            | Instr::NewStructure { .. }
            | Instr::NewTable { .. }
            | Instr::NewTypeDescription { .. }
            | Instr::NewValueComparison { .. }
            | Instr::NewMap { .. }
            | Instr::NewTextWriter { .. }
            | Instr::NewUuid { .. }
            | Instr::NewBinaryData { .. }
            | Instr::Raise { .. }
            | Instr::CloseText { .. }
            | Instr::CallObjectMethod { .. }
            | Instr::GetObjectProp { .. }
            | Instr::SetObjectProp { .. }
            | Instr::CallComponent { .. }
            | Instr::CreateObject { .. }
            | Instr::RunDynamic { .. } => {
                step_cold(
                    instr,
                    frames,
                    stack,
                    program,
                    current_exception,
                    snippets,
                    linked,
                    host,
                    runtime_shapes,
                    frame_idx,
                    func_id,
                    chunk,
                )?;
            }
            Instr::CollectionLen { dst, obj } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let len = ov.collection_len()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(
                    stack,
                    d,
                    BslValue::Number(bsl_number::BslNumber::from_i64(len as i64)),
                )?;
                frames[frame_idx].pc += 1;
            }
        }
        if extra == 0 {
            // Бандл кончился. Без JIT линейная цепочка бандлов
            // продолжается прямо здесь: смену кадра (`Call`) ловит первая
            // проверка, взятый переход — вторая, а `Return` вышел из
            // функции ранним `return` ещё в своей ветке. Всё, что
            // осталось, — обычный fallthrough, на котором пробам
            // `drive_with` делать нечего.
            if !merge_linear || frames.len() != frame_idx + 1 {
                break;
            }
            let next = frames[frame_idx].pc;
            if next != pc + 1 {
                break;
            }
            pc = next;
            if pc >= chunk.instrs.len() {
                break;
            }
            extra = chunk
                .bundle_len
                .get(pc)
                .copied()
                .unwrap_or(1)
                .saturating_sub(1);
            continue;
        }
        extra -= 1;
        // Предыдущий член сам продвинул `pc` кадра.
        pc = frames[frame_idx].pc;
        if pc >= chunk.instrs.len() {
            break;
        }
    }
    Ok(Step::Continue)
}

/// Холодная половина диспетчера: опкоды, тело которых делает настоящую
/// работу — выделяет объект, зовёт встроенную функцию или метод, пишет
/// текст, исполняет динамический фрагмент. Вынесены из [`step`] отдельной
/// функцией с `#[inline(never)]`, потому что горячий цикл
/// диспетчеризации живёт на грани кеша микроопераций: измерено, что рост
/// `step` на три байта роняет пустой цикл BSL в полтора раза при том же
/// числе исполненных инструкций. Лишний вызов на такой опкод теряется на
/// фоне его собственной работы.
///
/// # Errors
///
/// Возвращает ошибку исполнения опкода, `RtError::Raised` от
/// `ВызватьИсключение`, а на горячем опкоде, который сюда попасть не
/// может, — `RtError::InvalidBytecode`.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn step_cold(
    instr: Instr,
    frames: &mut [Frame],
    stack: &mut [BslValue],
    program: &Program,
    current_exception: &Option<BslValue>,
    snippets: &mut SnippetCache,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_>,
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    frame_idx: usize,
    func_id: usize,
    chunk: &bsl_bytecode::Chunk,
) -> Result<(), RtError> {
    match instr {
        Instr::NewArray { dst, base, count } => {
            let mut dims = Vec::with_capacity(count as usize);
            for i in 0..count {
                let v = reg_load(stack, frames[frame_idx].reg_index(base + i))?;
                dims.push(dim_to_usize(&v)?);
            }
            let arr = build_nested_array(&dims);
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, arr)?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewStructure {
            dst,
            shape,
            base,
            count,
        } => {
            let shape_rc = at(
                &program.shapes,
                shape as usize,
                "номер формы вне таблицы форм программы",
            )?
            .clone();
            let mut slots = Vec::with_capacity(count as usize);
            for i in 0..count {
                slots.push(reg_load(stack, frames[frame_idx].reg_index(base + i))?);
            }
            let v = BslValue::new_structure(shape_rc, slots);
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, v)?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewTable { dst } => {
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, BslValue::new_table())?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewTypeDescription { dst, names } => {
            let names = reg_load(stack, frames[frame_idx].reg_index(names))?;
            let value = BslValue::new_type_description(&names, runtime_shapes)?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewValueComparison { dst } => {
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, BslValue::new_value_comparison())?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewMap { dst } => {
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, BslValue::new_map())?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewTextWriter { dst, path } => {
            let path = reg_load(stack, frames[frame_idx].reg_index(path))?;
            let writer = BslValue::new_text_writer(&path)?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, writer)?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewUuid { dst, arg } => {
            let arg = reg_load(stack, frames[frame_idx].reg_index(arg))?;
            let uuid = BslValue::new_uuid(&arg)?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, uuid)?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewBinaryData { dst, path } => {
            let path = reg_load(stack, frames[frame_idx].reg_index(path))?;
            let data = BslValue::new_binary_data(&path)?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, data)?;
            frames[frame_idx].pc += 1;
        }
        Instr::Raise { src } => {
            let value = match src {
                Some(r) => reg_load(stack, frames[frame_idx].reg_index(r))?,
                // Голая форма: повторно бросаем то, что сейчас поймано
                // (или Неопределено, если бросить нечего — например,
                // `ВызватьИсключение;` вне `Исключение`).
                None => current_exception.clone().unwrap_or(BslValue::Undefined),
            };
            return Err(RtError::Raised(value));
        }
        Instr::GetObjectProp { dst, obj, name } => {
            let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
            let name_id = bsl_rt::NameId::from_index(name as u32);
            let value = if let Some(object) = ov.object_ref() {
                let mut context = bsl_rt::CallContext::new(
                    runtime_shapes,
                    &mut *host.stdout,
                    &mut *host.stderr,
                    bsl_format::format_value,
                );
                component_prop_get(
                    object,
                    &linked.component_properties,
                    name_id,
                    program,
                    &mut context,
                )?
            } else {
                match ov.get_field_cached(name_id, prop_cache(chunk, frames[frame_idx].pc)?) {
                    Err(RtError::NotAnObject) => {
                        ov.get_field_by_name(field_name(program, name_id)?)?
                    }
                    other => other?,
                }
            };
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::SetObjectProp { obj, name, src } => {
            let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
            let value = reg_load(stack, frames[frame_idx].reg_index(src))?;
            let name_id = bsl_rt::NameId::from_index(name as u32);
            if let Some(object) = ov.object_ref() {
                let mut context = bsl_rt::CallContext::new(
                    runtime_shapes,
                    &mut *host.stdout,
                    &mut *host.stderr,
                    bsl_format::format_value,
                );
                component_prop_set(
                    object,
                    &linked.component_properties,
                    name_id,
                    value,
                    program,
                    &mut context,
                )?;
            } else {
                match ov.set_field_cached(
                    name_id,
                    value.clone(),
                    prop_cache(chunk, frames[frame_idx].pc)?,
                ) {
                    Err(RtError::NotAnObject) => {
                        ov.set_field_by_name(field_name(program, name_id)?, value)?
                    }
                    other => other?,
                }
            }
            frames[frame_idx].pc += 1;
        }
        // Компонентные вызовы уехали сюда из горячего цикла: их тела —
        // самое крупное, что в нём лежало (машинерия `function_caller`
        // для обратных вызовов из компонентов), а стоимость холодного
        // перехода тонет в работе компонента. Горячему циклу важен
        // размер: он живёт на грани кеша микроопераций (см. комментарий
        // у списка холодных опкодов в `step`); вынос измерен A/B —
        // call_overhead с +12,3% до +4,3% к main, pi_leibniz с +12,1%
        // до +6,2%.
        Instr::CallComponent {
            dst, base, count, ..
        } => {
            let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
            let call = linked.function(func_id, frames[frame_idx].pc)?;
            let mut function_caller =
                |name: &str,
                 call_args: Vec<BslValue>,
                 stdout: &mut dyn Write,
                 stderr: &mut dyn Write| {
                    let mut nested_host = HostIo { stdout, stderr };
                    call_module_function_with_host(
                        program,
                        stack,
                        name,
                        call_args,
                        linked,
                        &mut nested_host,
                    )
                };
            let mut context = bsl_rt::CallContext::with_function_caller(
                runtime_shapes,
                &mut *host.stdout,
                &mut *host.stderr,
                bsl_format::format_value,
                &mut function_caller,
            );
            let value = call(&mut context, args.as_slice())?;
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::CreateObject {
            dst, base, count, ..
        } => {
            let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
            let call = linked.constructor(func_id, frames[frame_idx].pc)?;
            let mut context = bsl_rt::CallContext::new(
                runtime_shapes,
                &mut *host.stdout,
                &mut *host.stderr,
                bsl_format::format_value,
            );
            let value = call(&mut context, args.as_slice())?;
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::CallObjectMethod {
            dst,
            obj,
            method,
            base,
            count,
        } => {
            let ov = at(
                stack,
                frames[frame_idx].reg_index(obj),
                "чтение объекта за границей стека значений",
            )?;
            // Аргументы открытого вызова кодоген кладёт в свежие временные
            // регистры — в стеке значений они лежат подряд, и обработчик
            // получает их срезом стека без поштучного клонирования (это
            // измеримая часть цены вызова: Rc-инкременты и сбросы на
            // каждый аргумент). Регистры-алиасы параметров смежности не
            // гарантируют — такая база уходит запасным путём с копиями.
            // Приёмник тоже заимствуется, как у горячего `WriteText`:
            // обработчики не достают до стека VM (их `CallContext` — без
            // канала обратного вызова), поэтому заём безопасен, а
            // `reg_store` идёт уже после того, как значение вычислено.
            let contiguous_args = base as usize >= frames[frame_idx].param_aliases.len();
            let fallback_args;
            let args: &[BslValue] = if count == 0 {
                &[]
            } else if contiguous_args {
                let start = frames[frame_idx].reg_index(base);
                stack
                    .get(start..start + count as usize)
                    .ok_or(RtError::InvalidBytecode(
                        "чтение аргументов за границей стека значений",
                    ))?
            } else {
                fallback_args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                fallback_args.as_slice()
            };
            let name_id = bsl_rt::NameId::from_index(method as u32);
            let value = if let Some(object) = ov.object_ref() {
                let mut context = bsl_rt::CallContext::new(
                    runtime_shapes,
                    &mut *host.stdout,
                    &mut *host.stderr,
                    bsl_format::format_value,
                );
                // Тип со статической таблицей методов идёт кэшем ячейки
                // этой инструкции поверх мемоизированного моста «номер
                // имени → обработчик»; промах и тип без таблицы —
                // строковым `call_method`, там единственный источник
                // текста ошибки о неизвестном методе.
                match cached_component_method(
                    chunk,
                    frames[frame_idx].pc,
                    &linked.component_methods,
                    object.method_table(),
                    name_id,
                    program,
                )? {
                    Some(call) => call(object.as_dyn(), args, &mut context)?,
                    None => {
                        let method_name = field_name(program, name_id)?;
                        object.call_method(method_name, args, &mut context)?
                    }
                }
            } else {
                // Нативный получатель: обработчик по номеру имени из таблицы
                // связывания, строка нужна только тексту ошибки.
                let builtin =
                    linked
                        .builtin_method(name_id)
                        .ok_or_else(|| RtError::UnknownMethod {
                            method: field_name(program, name_id).unwrap_or("?").to_string(),
                            receiver: ov.type_name(),
                        })?;
                bsl_rt::call_builtin_method_ctx(builtin, ov, args, runtime_shapes)?
            };
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::CloseText { dst, obj } => {
            let obj_idx = frames[frame_idx].reg_index(obj);
            let object = at(stack, obj_idx, "чтение объекта за границей стека значений")?;
            let v = if let Some(extension) = object.object_ref() {
                let mut context = bsl_rt::CallContext::new(
                    runtime_shapes,
                    &mut *host.stdout,
                    &mut *host.stderr,
                    bsl_format::format_value,
                );
                extension.call_method("Закрыть", &[], &mut context)?
            } else {
                object.close_object()?
            };
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, v)?;
            frames[frame_idx].pc += 1;
        }
        Instr::RunDynamic { src, dst, is_eval } => {
            let code = reg_load(stack, frames[frame_idx].reg_index(src))?;
            let code = match code {
                BslValue::Str(s) => s.to_string(),
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: if is_eval {
                            "Вычислить"
                        } else {
                            "Выполнить"
                        },
                    });
                }
            };
            // Область видимости фрагмента — материализованная таблица
            // имён ЭТОГО кадра (`Chunk::local_names`), а не только
            // верхнего уровня: `Выполнить` внутри процедуры видит её
            // локальные. Таблица есть у всех чанков, помеченных
            // `uses_dynamic` в `bsl-sema`, а `RunDynamic` эмитится
            // только в них — так что пустой она здесь быть не может,
            // кроме как у кадра вообще без локальных переменных.
            let value = run_dynamic_snippet(
                &code,
                is_eval,
                program,
                &chunk.local_names,
                func_id,
                stack,
                &frames[frame_idx],
                snippets,
                linked,
                host,
            )?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, value)?;
            frames[frame_idx].pc += 1;
        }
        _ => {
            return Err(RtError::InvalidBytecode(
                "горячий опкод попал в холодную половину диспетчера",
            ));
        }
    }
    Ok(())
}

/// Компилирует и исполняет `code` в контексте top-level переменных
/// программы (см. `Instr::RunDynamic`). `is_eval` заворачивает `code` в
/// `Возврат (...)`, чтобы получить значение выражения тем же путём, что
/// и обычный оператор `Возврат` — общий движок для `Выполнить`/`Вычислить`
/// без раздвоения семантики.
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
fn run_dynamic_snippet(
    code: &str,
    is_eval: bool,
    program: &Program,
    scope_locals: &[String],
    scope_id: usize,
    stack: &mut [BslValue],
    frame: &Frame,
    snippets: &mut SnippetCache,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_>,
) -> Result<BslValue, RtError> {
    // Предел вложенности — на входе, до разбора: компиляция фрагмента
    // рекурсивна так же, как его исполнение, и тоже расходует стек Rust.
    let _depth = DynamicDepthGuard::enter()?;
    let compiled =
        snippets.get_or_compile(code, is_eval, scope_id, scope_locals, program, linked.env)?;

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

    let n_module = program.module_vars.len();
    // Где во фрагменте живут модульные переменные — зависит от области.
    //
    // Верхний уровень: они И ЕСТЬ первые локальные кадра (резолвер
    // объявляет их раньше прочих, слоты совпадают), и фрагмент видит их
    // через локальные слоты. Модульный блок фрагмента НАКЛАДЫВАЕТСЯ на те
    // же слоты (`module_base = 0`), а не копируется отдельно: две копии
    // одного значения — это рассинхрон. Стоило процедуре, вызванной из
    // фрагмента, записать в блок-копию, и обратный перенос локальных
    // затирал её изменение устаревшим значением слота.
    //
    // Кадр функции: модульные переменные в его локальные не входят и едут
    // отдельным блоком ЗА регистрами фрагмента. Источник — модульный блок
    // ТЕКУЩЕЙ программы по её `module_base`, а не абсолютный ноль: у
    // главной программы база нулевая, но у фрагмента, из которого нас
    // вызвали вложенным `Выполнить`, блок лежит за его собственными
    // регистрами, и чтение с нуля копировало бы чужие слоты. Вызовы внутри
    // фрагмента кладут свои кадры за блоком и обратно усекают стек только
    // до своей базы, так что блок стоит неподвижно.
    let aliased = scope_id == 0 && program.module_base == 0;
    let module_base = if aliased { 0 } else { snippet_stack.len() };
    if !aliased {
        for i in 0..n_module {
            snippet_stack.push(reg_load(stack, program.module_base as usize + i)?);
        }
    }

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
    let snippet_program = Program {
        requirements: compiled.requirements.clone(),
        chunks,
        // СОБСТВЕННАЯ таблица фрагмента, не `program.names`: она — префикс
        // (те же имена, в том же порядке, значит те же `NameId`) плюс,
        // возможно, новые поля, которых в статическом коде не было (см.
        // doc comment `CompiledSnippet`). Старые `GetProp`/`SetProp` — и
        // статического кода вокруг, и вложенных вызовов функций программы
        // (см. ниже про `chunks`) — по-прежнему резолвятся: их `NameId`
        // меньше длины `program.names` и указывают на тот же префикс.
        names: compiled.names.clone(),
        shapes: compiled.shapes.clone(),
        top_level_locals: Vec::new(),
        function_names: program.function_names.clone(),
        module_vars: program.module_vars.clone(),
        module_base: module_base as u32,
    };

    let snippet_linked = link_components(&snippet_program, linked.env)?;
    let (value, final_stack) = drive_linked(
        &snippet_program,
        0,
        snippet_stack,
        JitMode::Off,
        &snippet_linked,
        host,
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
    // Отдельный модульный блок возвращается только там, где он был
    // отдельным (область — кадр функции); на верхнем уровне модульные
    // значения лежат в локальных слотах и едут назад вместе с ними.
    if !aliased {
        for i in 0..n_module {
            reg_store(
                stack,
                program.module_base as usize + i,
                reg_load(&final_stack, module_base + i)?,
            )?;
        }
    }
    for i in 0..old_count {
        let d = frame.reg_index(i as u8);
        reg_store(stack, d, reg_load(&final_stack, i)?)?;
    }

    Ok(value)
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
/// `stack` — стек значений вызывающего: из него по [`Program::module_base`]
/// читается блок модульных переменных и в него же он возвращается после
/// вызова, поэтому запись в модульную переменную из вызванной процедуры
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
///   а не паника. Аргументов обязано быть РОВНО `n_params`: пропущенный
///   аргумент кодоген передаёт маркером `BslValue::Skipped`, а значение по
///   умолчанию вычисляет пролог самой вызванной функции — здесь это
///   работает так же.
/// - [`RtError::StackOverflow`] — превышена вложенность динамических
///   вызовов: вызов по имени — такой же вложенный `drive` на стеке Rust,
///   как `Выполнить`, и рекурсия через него не должна валить процесс мимо
///   `Попытка`.
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
    let linked = link_components(program, CompileEnv::bare())?;
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
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
pub fn call_module_function_with_registry_and_io(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
    registry: &bsl_rt::RuntimeRegistry,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let linked = link_components(program, CompileEnv::with_registry(registry))?;
    let mut host = HostIo { stdout, stderr };
    call_module_function_with_host(program, stack, name, args, &linked, &mut host)
}

fn call_module_function_with_host(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let _depth = DynamicDepthGuard::enter()?;

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

    // Модульный блок кладётся ЗА регистрами кадра — как у фрагмента
    // `Выполнить` в области функции: в локальные слоты функции модульные
    // переменные не входят, наложить их, как на верхнем уровне, не на что.
    // Источник — блок ТЕКУЩЕЙ программы по её `module_base`, а не
    // абсолютный ноль: у главной программы база нулевая, но нас могли
    // позвать изнутри фрагмента, где блок лежит за его собственными
    // регистрами. Вызовы внутри функции кладут свои кадры за блоком и
    // усекают стек только до своей базы, так что блок стоит неподвижно.
    let n_module = program.module_vars.len();
    let module_base = call_stack.len();
    for i in 0..n_module {
        call_stack.push(reg_load(stack, program.module_base as usize + i)?);
    }

    // Та же программа, но с базой модульного блока на новом стеке. Чанки,
    // имена и формы едут КАК ЕСТЬ: `Call func=N` индексирует ровно
    // `chunks[N]`, а вызванная функция может звать соседей по модулю, так
    // что нумерация обязана совпасть с исходной.
    let callee_program = Program {
        module_base: module_base as u32,
        ..program.clone()
    };

    let (value, final_stack) = drive_linked(
        &callee_program,
        func_id,
        call_stack,
        JitMode::Off,
        linked,
        host,
    )?;

    // Мутации модульных переменных обязаны пережить вызов — та же
    // дисциплина, что в `run_dynamic_snippet`.
    for i in 0..n_module {
        reg_store(
            stack,
            program.module_base as usize + i,
            reg_load(&final_stack, module_base + i)?,
        )?;
    }

    // Финальные значения слотов параметров: верхний кадр при возврате стек
    // не усекает (см. `do_return_with_value`), поэтому они всё ещё на
    // месте — в слотах `0..n_params`.
    let mut final_params = Vec::with_capacity(n_params);
    for i in 0..n_params {
        final_params.push(reg_load(&final_stack, i)?);
    }
    Ok((value, final_params))
}

/// Один скомпилированный фрагмент. `shapes` — СОБСТВЕННЫЙ список форм
/// фрагмента: индексы `shape` внутри `chunk` ссылаются именно на него, а
/// не на `program.shapes` (был баг ровно на этом — `NewStructure` попадал
/// по чужому индексу). `names` — по той же причине СОБСТВЕННАЯ (расширенная)
/// таблица имён полей: `compile_snippet` сеет свежий интернер именами
/// ОСНОВНОЙ программы, чтобы старые `NameId` совпали, но имя поля, которого
/// в статическом коде вообще не было (например, поле объекта, известное
/// только рантайму, как у `НастройкиСериализацииJSON`), получает НОВЫЙ
/// `NameId` ЗА пределами `program.names`. Раньше эта расширенная таблица
/// отбрасывалась (`let (chunk, _names, shapes) = ...`), и `GetProp`/`SetProp`
/// на такое поле падали с «идентификатор имени вне таблицы имён программы»
/// — тот же класс бага, что и с формами, просто не пойманный тогда.
struct CompiledSnippet {
    chunk: bsl_bytecode::Chunk,
    names: Vec<String>,
    shapes: Vec<std::rc::Rc<bsl_rt::Shape>>,
    requirements: Vec<bsl_bytecode::LibraryRequirement>,
}

/// Кэш «текст фрагмента -> скомпилированный чанк» на одно исполнение
/// (`drive`). Без него `Выполнить(...)` в цикле заново лексирует, парсит,
/// резолвит и компилирует одну и ту же строку на каждой итерации.
///
/// Ключ — пара (ОБЛАСТЬ ВИДИМОСТИ, текст), а не один текст: один и тот же
/// исходник, выполненный в разных кадрах, резолвится в РАЗНЫЕ номера
/// слотов, и переиспользовать чанк между ними нельзя. Область
/// идентифицируется номером чанка (`func_id`), потому что таблица имён
/// (`Chunk::local_names`) у чанка одна и та же на всех его вызовах.
/// `is_eval` тоже входит в ключ — от него зависит сам компилируемый текст
/// (`Возврат (...)` вместо операторов).
struct SnippetCache {
    entries: std::collections::HashMap<(usize, bool, String), std::rc::Rc<CompiledSnippet>>,
}

impl SnippetCache {
    fn new() -> Self {
        SnippetCache {
            entries: std::collections::HashMap::new(),
        }
    }

    fn get_or_compile(
        &mut self,
        code: &str,
        is_eval: bool,
        scope_id: usize,
        scope_locals: &[String],
        program: &Program,
        env: CompileEnv<'_>,
    ) -> Result<std::rc::Rc<CompiledSnippet>, RtError> {
        let key = (scope_id, is_eval, code.to_string());
        if let Some(hit) = self.entries.get(&key) {
            return Ok(hit.clone());
        }
        let compiled = std::rc::Rc::new(compile_dynamic_snippet(
            code,
            is_eval,
            scope_locals,
            program,
            env,
        )?);
        self.entries.insert(key, compiled.clone());
        Ok(compiled)
    }
}

/// Лексика/парсинг/резолвинг/кодоген фрагмента. Любая ошибка на этом пути
/// — `RtError::DynamicError` В МОМЕНТ ИСПОЛНЕНИЯ, а не паника и не ошибка
/// сборки: текст фрагмента становится известен только сейчас, и кривой
/// текст обязан ловиться обычной `Попытка`.
fn compile_dynamic_snippet(
    code: &str,
    is_eval: bool,
    scope_locals: &[String],
    program: &Program,
    env: CompileEnv<'_>,
) -> Result<CompiledSnippet, RtError> {
    // `is_eval` заворачивает выражение в `Возврат (...)`, чтобы получить
    // значение тем же путём, что и обычный `Возврат` — один движок на
    // `Выполнить` и `Вычислить`, без раздвоения семантики.
    let src = if is_eval {
        format!("Возврат ({code});")
    } else {
        code.to_string()
    };

    let parsed = bsl_syntax::parse_with_symbols(&src, &env.symbols)
        .map_err(|e| RtError::DynamicError(format!("{e:?}")))?;
    let mut stmts = Vec::with_capacity(parsed.items.len());
    for item in parsed.items {
        match item {
            bsl_syntax::Item::Stmt(s) => stmts.push(s),
            bsl_syntax::Item::VarDecl(vd) => stmts.push(bsl_syntax::Stmt::VarDecl(vd)),
            // НЕ ИЗМЕРЕНО(EXEC.PROC_DECLARATION): может ли фрагмент вообще
            // объявлять процедуры и функции. Взято «нет» — объявленную
            // процедуру было бы некуда деть: таблица чанков программы уже
            // скомпилирована.
            _ => {
                return Err(RtError::DynamicError(
                    "Выполнить/Вычислить не поддерживают объявление процедур/функций".to_string(),
                ));
            }
        }
    }

    // Имя + арность каждой функции модуля, в порядке `chunks[1..]`.
    let signatures: Vec<(String, usize)> = program
        .function_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let arity = program.chunks.get(i + 1).map_or(0, |c| c.n_params as usize);
            (name.clone(), arity)
        })
        .collect();
    let (all_locals, body, fragment_requirements) = match env.registry {
        Some(registry) => bsl_sema::resolve_snippet_stmts_with_registry(
            scope_locals,
            &program.module_vars,
            &stmts,
            &signatures,
            registry,
        )
        .map_err(|e| RtError::DynamicError(format!("{e:?}")))?,
        None => {
            let (locals, body) = bsl_sema::resolve_snippet_stmts(
                scope_locals,
                &program.module_vars,
                &stmts,
                &signatures,
            )
            .map_err(|e| RtError::DynamicError(format!("{e:?}")))?;
            (
                locals,
                body,
                vec![bsl_bytecode::LibraryRequirement::bsl_rt()],
            )
        }
    };
    let requirements = merge_requirements(&program.requirements, &fragment_requirements)?;
    // Режимы параметров каждой функции модуля: фрагмент может её звать, и
    // компилятору надо знать, какой аргумент идёт по ссылке.
    let callee_params: Vec<Vec<bool>> = program
        .chunks
        .iter()
        .skip(1)
        .map(|c| c.param_by_val.clone())
        .collect();
    let (chunk, names, shapes) = bsl_bytecode::compile_snippet_with_requirements(
        &all_locals,
        &body,
        &program.names,
        &callee_params,
        &requirements,
    )
    .map_err(|e| RtError::DynamicError(format!("{e:?}")))?;

    Ok(CompiledSnippet {
        chunk,
        names,
        shapes,
        requirements,
    })
}

fn merge_requirements(
    base: &[bsl_bytecode::LibraryRequirement],
    extra: &[bsl_bytecode::LibraryRequirement],
) -> Result<Vec<bsl_bytecode::LibraryRequirement>, RtError> {
    let mut merged = base.to_vec();
    for requirement in extra {
        match merged
            .iter()
            .find(|existing| existing.package == requirement.package)
        {
            Some(existing) if existing.version != requirement.version => {
                return Err(RtError::Component(format!(
                    "для {} одновременно требуются версии {} и {}",
                    requirement.package, existing.version, requirement.version
                )));
            }
            Some(_) => {}
            None => merged.push(requirement.clone()),
        }
    }
    merged[1..].sort_by(|left, right| left.package.cmp(&right.package));
    Ok(merged)
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
    chunk.bundle_len = bsl_bytecode::bundle::compute(chunk, None);
    Ok(())
}

/// Размерность в `Новый Массив(d1, d2, ...)` обязана быть целым
/// неотрицательным числом.
fn dim_to_usize(v: &BslValue) -> Result<usize, RtError> {
    match v {
        BslValue::Number(n) => {
            let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
            usize::try_from(i).map_err(|_| RtError::BadIndex)
        }
        _ => Err(RtError::TypeError {
            expected: "Число",
            op: "Новый Массив(...)",
        }),
    }
}

/// `Новый Массив(3, 4)` — массив из 3 массивов по 4: каждое измерение
/// вкладывает следующий уровень, элементы на дне — `Неопределено`. Каждый
/// вложенный массив — отдельный объект (не общий `Rc`, иначе мутация одного
/// была бы видна во всех остальных).
fn build_nested_array(dims: &[usize]) -> BslValue {
    match dims.split_first() {
        Some((&n, rest)) => {
            let items = (0..n)
                .map(|_| {
                    if rest.is_empty() {
                        BslValue::Undefined
                    } else {
                        build_nested_array(rest)
                    }
                })
                .collect();
            BslValue::new_array(items)
        }
        None => BslValue::new_array(Vec::new()),
    }
}

/// Заводит "собственные" регистры чанка (сверх параметров) в конце стека —
/// используется и для верхнего уровня (0 параметров), и для вызовов.
/// `n_regs < n_params` кодоген не порождает (пиковое число регистров
/// включает параметры), но вычитание `u8` на таком чанке паниковало бы в
/// debug и молча заворачивалось в release — берём насыщающее, а промах по
/// регистру дальше поймает `reg_load`/`reg_store` уже как
/// `InvalidBytecode`.
fn push_own_registers(stack: &mut Vec<BslValue>, chunk: &bsl_bytecode::Chunk) {
    let n_own = chunk.n_regs.saturating_sub(chunk.n_params) as usize;
    stack.resize(stack.len() + n_own, BslValue::Undefined);
}

enum ReturnOutcome {
    Done(BslValue),
    Continuing,
}
use ReturnOutcome::{Continuing, Done};

/// `frames.pop().expect(...)` ниже — ВНУТРЕННИЙ ИНВАРИАНТ VM, а не входные
/// данные (см. классификацию в шапке модуля): `step`/`drive` вызывают эту
/// функцию только пока `frames` не пуст — сам факт того, что мы исполняем
/// инструкцию, это гарантирует. Никакой байт-код, корректный или нет, сюда
/// с пустым стеком кадров не приведёт.
fn do_return_with_value(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    value: BslValue,
) -> Result<ReturnOutcome, RtError> {
    let frame = frames
        .pop()
        .expect("инвариант VM: возврат исполняется только при непустом стеке кадров");
    match frames.last() {
        None => {
            // Самый верхний кадр завершился: НЕ усекаем стек — `drive`
            // возвращает его вызывающему как есть (нужно `run_isolated`,
            // чтобы прочитать финальные значения регистров после
            // `Выполнить`/`Вычислить`, а обычному `run_program` разницы
            // нет — он этот стек всё равно не читает после возврата).
            Ok(Done(value))
        }
        Some(caller) => {
            stack.truncate(frame.call_start);
            let dst = caller.reg_index(frame.return_reg);
            reg_store(stack, dst, value)?;
            Ok(Continuing)
        }
    }
}

/// Ищет защищённый диапазон, содержащий `pc`, в данном чанке. При
/// нескольких вложенных диапазонах (`Попытка` внутри `Попытка`) выбирает
/// самый узкий — самый внутренний `Try` должен ловить раньше внешнего.
fn find_handler(chunk: &bsl_bytecode::Chunk, pc: usize) -> Option<usize> {
    chunk
        .exception_ranges
        .iter()
        .filter(|r| pc >= r.start_pc && pc < r.end_pc)
        .min_by_key(|r| r.end_pc - r.start_pc)
        .map(|r| r.handler_pc)
}

/// Разматывает кадры в поисках обработчика для только что брошенной ошибки.
/// Возвращает `true`, если нашли (кадры/pc уже поправлены — можно продолжать
/// цикл `run_program`), `false` — если исключение долетело до самого низа
/// стека кадров, не будучи пойманным нигде.
///
/// Кадр, где ошибка ПРОИЗОШЛА, проверяется по своему текущему `pc` (он ещё
/// не продвинут — инструкция вернула `Err` раньше, чем дошла до
/// инкремента). Любой кадр ВЫШЕ по стеку (куда мы попадаем, откатываясь
/// из-за того, что внутренний вызов не поймал исключение сам) проверяется
/// по `pc - 1` — позиции его собственной инструкции `Call`, а не следующей
/// за ней (которая уже была продвинута в момент самого вызова).
fn unwind_to_handler(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    err: &RtError,
    current_exception: &mut Option<BslValue>,
) -> bool {
    let mut first = true;
    loop {
        let frame_idx = frames.len() - 1;
        let chunk = match program.chunks.get(frames[frame_idx].func_id) {
            Some(c) => c,
            // Кадр с несуществующим чанком — не наше дело здесь: ошибку
            // уже несут наружу, обработчик в нём всё равно не найти.
            None => return false,
        };
        let check_pc = if first {
            frames[frame_idx].pc
        } else {
            // Кадр выше по стеку всегда стоит ЗА своей инструкцией `Call`,
            // так что `pc >= 1`; насыщение — страховка от того, что кадр
            // собрали не мы (см. классификацию паник в шапке модуля).
            frames[frame_idx].pc.saturating_sub(1)
        };
        first = false;

        if let Some(handler_pc) = find_handler(chunk, check_pc) {
            *current_exception = Some(err_to_value(err));
            frames[frame_idx].pc = handler_pc;
            return true;
        }

        if frames.len() == 1 {
            return false;
        }
        let frame = frames
            .pop()
            .expect("инвариант VM: `frames.len() >= 2` проверено строкой выше");
        stack.truncate(frame.call_start);
    }
}

/// Значение, которое видит `Исключение`-блок при повторном броске
/// (`ВызватьИсключение;` без выражения). Для `ВызватьИсключение <знач>;` это
/// само `<знач>`; для внутренних ошибок VM (деление на ноль, обращение к
/// несуществующему полю, ...) — их текстовое описание, потому что
/// полноценного объекта информации об ошибке (`ИнформацияОбОшибке()`) пока
/// нет, это отдельная задача поверх механизма builtin-функций.
fn err_to_value(err: &RtError) -> BslValue {
    match err {
        RtError::Raised(v) => v.clone(),
        other => BslValue::Str(bsl_rt::BslString::from_str(&other.to_string())),
    }
}

/// `Строка`/`Формат`/`Число`/`Message` перехватываются здесь, а не в
/// `bsl_rt::call_builtin_fn`: форматирование живёт в `bsl-format`, которое
/// зависит от `bsl-rt` (не наоборот) — `bsl-rt` физически не может
/// отформатировать число сам. Всё остальное уходит в `bsl-rt` как обычно.
///
/// `ПрочитатьJSON`/`ЗаписатьJSON` перехватываются здесь по той же причине,
/// но с другого конца: их функции восстановления и преобразования — это
/// вызов пользовательской функции ПО ИМЕНИ, то есть [`call_module_function`],
/// которой в `bsl-rt` быть не может (зависимость идёт в обратную сторону).
/// Поэтому `program` и `stack` доходят сюда: из них строится замыкание,
/// которое рантайм зовёт как обычную функцию.
///
/// Арность проверена в `bsl-sema` — но проверена для того байт-кода,
/// который родился из резолвинга. Здесь она перепроверяется один раз на
/// вызов, чтобы ни эта функция, ни `bsl_rt::call_builtin_fn` (тоже
/// индексирующая `args` напрямую) не паниковали на чанке, собранном мимо
/// резолвера.
fn call_builtin_with_format(
    builtin: bsl_rt::BuiltinFn,
    args: &[BslValue],
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    host: &mut HostIo<'_>,
) -> Result<BslValue, RtError> {
    use bsl_rt::BuiltinFn;
    // Проверка по МАКСИМУМУ, а не по минимуму: резолвер добивает
    // необязательные позиции `Неопределено` (см.
    // `BuiltinFn::arity_range`), так что корректный байт-код всегда
    // приносит полный набор — и `call_builtin_fn` может индексировать
    // `args[1]`/`args[2]` без проверок на каждой ветке.
    if args.len() < builtin.arity_range().1 {
        return Err(RtError::InvalidBytecode(
            "встроенной функции передано меньше аргументов, чем требует её арность",
        ));
    }
    match builtin {
        BuiltinFn::Message => {
            writeln!(host.stdout, "{}", bsl_format::format_value(&args[0], None)?)
                .map_err(|error| RtError::IoError(error.to_string()))?;
            Ok(BslValue::Undefined)
        }
        BuiltinFn::ToString => {
            let s = bsl_format::format_value(&args[0], None)?;
            Ok(BslValue::Str(bsl_rt::BslString::from_str(&s)))
        }
        BuiltinFn::Format => {
            let spec = match &args[1] {
                BslValue::Str(s) => s.to_string(),
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: "Формат(..., СтрокаФормата)",
                    });
                }
            };
            let s = bsl_format::format_value(&args[0], Some(&spec))?;
            Ok(BslValue::Str(bsl_rt::BslString::from_str(&s)))
        }
        BuiltinFn::ToNumber => {
            let s = match &args[0] {
                BslValue::Str(s) => s,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: "Число(...)",
                    });
                }
            };
            let n = bsl_format::parse_number(&s.to_string(), &bsl_format::NumberFormat::default())?;
            Ok(BslValue::Number(n))
        }
        // Не `call_builtin_fn`: `ЗаполнитьЗначенияСвойств` читает таблицу
        // имён, и путь без контекста для неё кончается ошибкой.
        other => bsl_rt::call_builtin_fn_ctx(other, args, runtime_shapes),
    }
}

/// Тело инструкции `Add`.
///
/// Отдельной функцией, потому что её зовут ДВОЕ: ветка `step`
/// интерпретатора и шим JIT-а. Второй реализации сложения строк в
/// проекте быть не должно — при расхождении режимов `--jit` и обычного
/// не сработал бы ни один существующий тест.
fn add_op(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    b: u8,
) -> Result<(), RtError> {
    // Накопление строки в саму себя (`Текст = Текст + Кусок`
    // — приёмник и левый операнд один регистр) идёт особым
    // путём: значение ЗАБИРАЕТСЯ из регистра, а не копируется.
    // Регистр всё равно будет перезаписан результатом, зато
    // счётчик ссылок падает до единицы, и буфер дописывается
    // на месте вместо копирования всего накопленного.
    //
    // Условие на ОБЕ строки проверяется ДО того, как регистр
    // опустошён: иначе ошибка типа оставила бы переменную
    // затёртой, а её мог бы поймать `Попытка` и поехать
    // дальше с потерянным значением.
    let d = frames[frame_idx].reg_index(dst);
    let ia = frames[frame_idx].reg_index(a);
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    let both_strings = matches!(
        (stack.get(ia), &bv),
        (Some(BslValue::Str(_)), BslValue::Str(_))
    );
    if d == ia && both_strings {
        let av = std::mem::replace(&mut stack[ia], BslValue::Undefined);
        let (BslValue::Str(left), BslValue::Str(right)) = (av, &bv) else {
            unreachable!("типы проверены выше")
        };
        stack[d] = BslValue::Str(left.append(right));
    } else if matches!(stack.get(ia), Some(BslValue::Str(_))) {
        // Строка СЛЕВА решает исход: правый операнд приводится к строке
        // и приклеивается, каким бы он ни был. Измерено на 8.3.27 по
        // всем типам сразу — и приведение оказалось ровно `Строка()`,
        // вместе с разделителями групп («Сумма: » + 1000.5 даёт
        // «Сумма: 1 000,5» с НЕРАЗРЫВНЫМ пробелом внутри). Поэтому
        // здесь именно `format_value`, а не своё представление числа.
        let av = reg_load(stack, ia)?;
        let BslValue::Str(left) = av else {
            unreachable!("тип проверен выше")
        };
        let right = bsl_format::format_value(&bv, None)?;
        let joined = left.append(&bsl_rt::BslString::from_str(&right));
        reg_store(stack, d, BslValue::Str(joined))?;
    } else {
        // Тот же порядок, что и в `binop`: сначала как есть, приведение
        // — только после отказа. Строка слева сюда уже не попадает (её
        // разобрала ветка выше), поэтому подменить склейку арифметикой
        // этот повтор не может.
        let av = reg_load(stack, ia)?;
        let sum = match av.add(&bv) {
            Ok(v) => v,
            Err(first) => {
                if needs_arith_coercion(&av) || needs_arith_coercion(&bv) {
                    arith(&av)?.add(arith(&bv)?.as_ref())?
                } else {
                    return Err(first);
                }
            }
        };
        reg_store(stack, d, sum)?;
    }
    Ok(())
}

/// Операнд арифметики после приведения.
///
/// Платформа тянет к числу и строку, и булево: `5 + "3"` даёт 8,
/// `Истина + 1` — 2, `-Истина` — минус единицу (всё измерено). Разбор
/// строки — тот же, что у `Число()`: с обрезкой пробелов, точкой ИЛИ
/// запятой и разделителями групп, так что `("" + 1000.5) - 0.5` честно
/// возвращает 1000.
///
/// `Cow` здесь не украшение: приведение нужно РЕДКО, а сложение чисел
/// лежит на самом горячем пути, и лишней копии значения на нём быть не
/// должно.
#[inline]
fn arith(v: &BslValue) -> Result<std::borrow::Cow<'_, BslValue>, RtError> {
    match v {
        BslValue::Str(s) => {
            let n = bsl_format::parse_number(&s.to_string(), &bsl_format::NumberFormat::default())
                .map_err(RtError::Num)?;
            Ok(std::borrow::Cow::Owned(BslValue::Number(n)))
        }
        // Истина — единица, Ложь — ноль.
        BslValue::Boolean(b) => Ok(std::borrow::Cow::Owned(BslValue::Number(
            bsl_number::BslNumber::from_i64(i64::from(*b)),
        ))),
        _ => Ok(std::borrow::Cow::Borrowed(v)),
    }
}

/// Тело инструкции `Neg`. Отдельной функцией по той же причине, что и
/// [`add_op`]: её зовут и интерпретатор, и шим JIT-а.
fn neg_op(v: &BslValue) -> Result<BslValue, RtError> {
    // Тот же порядок, что в `binop`: сначала как есть. Унарный минус лежит
    // на горячем пути не меньше сложения (`flip = -flip` в цикле), и
    // строить `Cow` ради заведомого числа он не должен.
    match v.neg() {
        Ok(r) => Ok(r),
        Err(first) => {
            if needs_arith_coercion(v) {
                arith(v)?.neg()
            } else {
                Err(first)
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn binop(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    b: u8,
    f: impl Fn(&BslValue, &BslValue) -> Result<BslValue, RtError>,
) -> Result<(), RtError> {
    let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    // Приведение строк и булевых к числу — свойство ВСЕЙ арифметики, а не
    // одного сложения: измерены `"5" - 1`, `"5" * 2`, `"5" / 2`,
    // `Истина - 1` и `Истина * 2`.
    //
    // Порядок здесь ради ЦЕНЫ: сначала операция пробуется как есть, и
    // только её отказ включает приведение. Пара чисел — подавляющее
    // большинство вызовов — не платит за приведение вообще ничего, даже
    // проверки тега. Повтор безопасен, потому что операции над значениями
    // чистые: неудачная попытка ничего не меняет.
    let result = match f(&av, &bv) {
        Ok(v) => v,
        Err(first) => {
            if needs_arith_coercion(&av) || needs_arith_coercion(&bv) {
                f(arith(&av)?.as_ref(), arith(&bv)?.as_ref())?
            } else {
                return Err(first);
            }
        }
    };
    let d = frames[frame_idx].reg_index(dst);
    reg_store(stack, d, result)?;
    Ok(())
}

/// Нужно ли значению приведение перед арифметикой. Вынесено, чтобы горячий
/// путь не звал [`arith`] ради заведомо чисел.
#[inline(always)]
fn needs_arith_coercion(v: &BslValue) -> bool {
    matches!(v, BslValue::Str(_) | BslValue::Boolean(_))
}

#[allow(clippy::too_many_arguments)]
fn cmp(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    b: u8,
    op: &'static str,
    f: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<(), RtError> {
    let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    let ord = av.compare(&bv, op)?;
    let d = frames[frame_idx].reg_index(dst);
    reg_store(stack, d, BslValue::Boolean(f(ord)))?;
    Ok(())
}

#[cfg(test)]
mod tests;
