use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use crate::{BslValue, RtError, RtResult, RuntimeShapes};

/// Стабильный ключ библиотеки между семантическим анализом и компилятором.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LibraryKey(String);

impl LibraryKey {
    pub fn new(package: impl Into<String>) -> Self {
        Self(package.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Точная версия runtime-компонента, необходимого программе.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRequirement {
    pub package: String,
    pub version: String,
}

impl LibraryRequirement {
    pub fn new(package: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            version: version.into(),
        }
    }

    pub fn bsl_rt() -> Self {
        Self::new(crate::PACKAGE_NAME, crate::PACKAGE_VERSION)
    }
}

/// Код конструктора внутри одного runtime-компонента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstructorCode(u16);

impl ConstructorCode {
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Код глобальной функции внутри одного runtime-компонента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionCode(u16);

impl FunctionCode {
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Допустимое число аргументов функции или конструктора.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arity {
    min: u8,
    max: u8,
}

impl Arity {
    pub const fn exact(count: u8) -> Self {
        Self {
            min: count,
            max: count,
        }
    }

    pub const fn range(min: u8, max: u8) -> Self {
        assert!(min <= max, "минимальная арность больше максимальной");
        Self { min, max }
    }

    pub const fn min(self) -> u8 {
        self.min
    }

    pub const fn max(self) -> u8 {
        self.max
    }

    pub const fn accepts(self, count: u8) -> bool {
        self.min <= count && count <= self.max
    }
}

/// Форматирование значения предоставляется верхним слоем, поскольку
/// `bsl-format` зависит от `bsl-rt`, а обратная зависимость запрещена.
pub type ValueFormatter = fn(&BslValue, Option<&str>) -> RtResult<String>;

/// Вызов экспортной функции исполняемого модуля по имени.
///
/// Потоки передаются в сам вызов, чтобы замыканию не приходилось
/// владеть `HostIo` VM и одновременно заимствовать его для
/// [`CallContext::stdout`] и [`CallContext::stderr`].
pub type FunctionCaller<'a> = dyn FnMut(
        &str,
        Vec<BslValue>,
        &mut dyn Write,
        &mut dyn Write,
    ) -> RtResult<(BslValue, Vec<BslValue>)>
    + 'a;

/// Возможность прогона, которой может не быть на конкретном пути
/// исполнения. Наличие возможности — это данные контекста, а не отдельный
/// тип контекста: сокращённый контекст просто не несёт внешних возможностей.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Capability {
    Stdout,
    Stderr,
    Zone,
    FileSystem,
    FunctionCaller,
    Random,
}

/// Каким контекстом прогона располагал вызов, у которого спросили
/// возможность, — для текста ошибки и только.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextKind {
    /// Полный контекст с внешними возможностями прогона.
    Full,
    /// Сокращённый контекст без внешних возможностей.
    Reduced,
}

/// Именованная запись сервисов интерпретаторного пути — вместо позиционных
/// аргументов конструктора. Внешние возможности здесь есть всегда
/// (интерпретатор их несёт); `function_caller` — `None`, если из этого
/// вызова BSL-функцию модуля звать нельзя.
pub struct InterpreterServices<'a> {
    pub runtime_shapes: &'a mut RuntimeShapes,
    pub formatter: ValueFormatter,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    pub zone: &'a Rc<dyn crate::TimeZone>,
    pub files: &'a Rc<dyn crate::FileSystem>,
    pub random: &'a crate::RandomHandle,
    pub function_caller: Option<&'a mut FunctionCaller<'a>>,
}

/// Сервисы конкретного состояния исполнения, доступные компоненту.
///
/// Контекст не раскрывает стек и регистры VM. Вывод и таблицы форм
/// принадлежат сессии, поэтому две VM в одном процессе не разделяют
/// изменяемое состояние.
///
/// Возможности прогона (`stdout`, `stderr`, `zone`, `files`, `random`, `function_caller`) —
/// это `Option` В ПОЛЯХ: на НАТИВНОМ пути (JIT-шимы) их нет, и обращение к
/// отсутствующей отвечает одним типизированным отказом
/// [`RtError::CapabilityMissing`], а не молчаливым стоком или чужим
/// временем. Путь (`path`) — факт о вызывающем, он попадает в текст отказа.
pub struct CallContext<'a> {
    // Всегда есть — не зависят от пути исполнения.
    runtime_shapes: &'a mut RuntimeShapes,
    formatter: ValueFormatter,
    /// Каким путём построен контекст: факт о вызывающем, попадает в текст
    /// [`RtError::CapabilityMissing`].
    path: ContextKind,
    // Возможности: `None` на нативном пути (JIT-шимы).
    stdout: Option<&'a mut dyn Write>,
    stderr: Option<&'a mut dyn Write>,
    /// Часовой пояс ПРОГОНА либо `None` — «этот путь о зоне не знает».
    /// Компоненту зона нужна там, где записанный момент переводится в
    /// местное время (даты JSON, лексические формы XDTO). `None` приходит с
    /// нативного пути: у JIT-шимов зоны нет по той же причине, что и вывода
    /// — реестр компонентов ОТКРЫТ, и метод стороннего типа под `--jit`
    /// получит `CapabilityMissing` там, где интерпретатор отвечает значением
    /// (обе стороны закреплены тестами `crates/open-bsl/tests/embedding.rs`).
    zone: Option<&'a Rc<dyn crate::TimeZone>>,
    /// Файловая система ПРОГОНА либо `None` — «этот путь о ней не знает».
    /// `None` приходит с нативного пути (JIT-шимы), как и у зоны: объект,
    /// которому нужна ФС, забирает её у своего КОНСТРУКТОРА (тот идёт
    /// интерпретатором, `CreateObject` не шимится) и хранит сам, а не
    /// спрашивает контекст на каждом вызове метода.
    files: Option<&'a Rc<dyn crate::FileSystem>>,
    random: Option<&'a crate::RandomHandle>,
    function_caller: Option<&'a mut FunctionCaller<'a>>,
}

impl<'a> CallContext<'a> {
    /// Контекст ИНТЕРПРЕТАТОРНОГО пути: потоки и зона есть всегда.
    pub fn interpreter(services: InterpreterServices<'a>) -> Self {
        Self {
            runtime_shapes: services.runtime_shapes,
            formatter: services.formatter,
            path: ContextKind::Full,
            stdout: Some(services.stdout),
            stderr: Some(services.stderr),
            zone: Some(services.zone),
            files: Some(services.files),
            random: Some(services.random),
            function_caller: services.function_caller,
        }
    }

    /// Контекст НАТИВНОГО пути (JIT-шимы): ни потоков, ни зоны, ни вызова
    /// функции модуля — только таблица форм и форматтер. Обращение к
    /// отсутствующей возможности отвечает [`RtError::CapabilityMissing`],
    /// а не молчаливым стоком.
    pub fn native(runtime_shapes: &'a mut RuntimeShapes, formatter: ValueFormatter) -> Self {
        Self {
            runtime_shapes,
            formatter,
            path: ContextKind::Reduced,
            stdout: None,
            stderr: None,
            zone: None,
            files: None,
            random: None,
            function_caller: None,
        }
    }

    pub fn runtime_shapes(&mut self) -> &mut RuntimeShapes {
        self.runtime_shapes
    }

    /// Поток вывода прогона.
    ///
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если контекст построен без вывода
    /// (нативный путь): прежде JIT-шимы подставляли молчаливый сток, теперь
    /// компонент, пишущий под `--jit`, получает явный отказ.
    pub fn stdout(&mut self) -> RtResult<&mut (dyn Write + 'a)> {
        let path = self.path;
        self.stdout
            .as_deref_mut()
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::Stdout,
                path,
            })
    }

    /// Поток ошибок прогона.
    ///
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если контекст построен без потока
    /// ошибок (нативный путь).
    pub fn stderr(&mut self) -> RtResult<&mut (dyn Write + 'a)> {
        let path = self.path;
        self.stderr
            .as_deref_mut()
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::Stderr,
                path,
            })
    }

    pub fn format_value(&self, value: &BslValue, spec: Option<&str>) -> RtResult<String> {
        (self.formatter)(value, spec)
    }

    /// Часовой пояс прогона — для перевода записанного момента в местное
    /// время. Отдаётся ссылкой, а не значением смещения: смещение зависит
    /// от МОМЕНТА, и какой момент интересует, знает только компонент.
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если контекст построен без зоны (см.
    /// поле `zone`): значит зону спросил путь, который её не получает, и
    /// честнее ошибка, чем чужое время.
    pub fn zone(&self) -> RtResult<&dyn crate::TimeZone> {
        self.zone.map(Rc::as_ref).ok_or(RtError::CapabilityMissing {
            capability: Capability::Zone,
            path: self.path,
        })
    }

    /// Зона В СОБСТВЕННОСТЬ — для компонента, который её ЗАПОМИНАЕТ, а не
    /// читает по ходу вызова (фабрика XDTO хранит зону своего построения).
    ///
    /// # Errors
    ///
    /// То же, что у [`CallContext::zone`].
    pub fn zone_rc(&self) -> RtResult<Rc<dyn crate::TimeZone>> {
        self.zone.map(Rc::clone).ok_or(RtError::CapabilityMissing {
            capability: Capability::Zone,
            path: self.path,
        })
    }

    /// Файловая система прогона ссылкой — для компонента, который открывает
    /// файл ПРЯМО В КОНСТРУКТОРЕ и дальше держит только дескриптор
    /// (`ЗаписьТекста`, `ФайловыйПоток`).
    ///
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если контекст без файловой системы
    /// (нативный путь).
    pub fn files(&self) -> RtResult<&dyn crate::FileSystem> {
        let path = self.path;
        self.files
            .map(Rc::as_ref)
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::FileSystem,
                path,
            })
    }

    /// Файловая система прогона В СОБСТВЕННОСТЬ — для компонента, который её
    /// ЗАПОМИНАЕТ и обращается к путям в своих методах (`ТекстовыйДокумент`,
    /// читатель/писатель архива, менеджер файловых потоков): метод может
    /// пойти нативным путём под JIT, где контекста с ФС уже нет.
    ///
    /// # Errors
    ///
    /// То же, что у [`CallContext::files`].
    pub fn files_rc(&self) -> RtResult<Rc<dyn crate::FileSystem>> {
        self.files.map(Rc::clone).ok_or(RtError::CapabilityMissing {
            capability: Capability::FileSystem,
            path: self.path,
        })
    }

    /// Источник случайности прогона. Расстановку битов версии UUID делает
    /// конструктор значения, а не источник.
    ///
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если контекст сокращённый.
    pub fn random(&self) -> RtResult<&crate::RandomHandle> {
        self.random.ok_or(RtError::CapabilityMissing {
            capability: Capability::Random,
            path: self.path,
        })
    }

    /// Таблица форм и зона ОДНОВРЕМЕННО: `runtime_shapes` берёт `self`
    /// изменяемо, поэтому после него `zone()` уже не позвать, а нужны они
    /// вместе на каждом чтении и записи JSON-значения.
    ///
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если зоны нет (нативный путь).
    pub fn shapes_and_zone(&mut self) -> RtResult<(&mut RuntimeShapes, &dyn crate::TimeZone)> {
        let zone = self
            .zone
            .map(Rc::as_ref)
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::Zone,
                path: self.path,
            })?;
        Ok((self.runtime_shapes, zone))
    }

    /// Выдаёт на время одного замыкания разделённые изменяемые сервисы для
    /// операции, которая одновременно меняет таблицу форм, пишет в потоки и
    /// может вызвать BSL-функцию (чтение и запись JSON с обратными
    /// вызовами). Замыкание, а не набор аксессоров: у полей РАЗНЫЕ
    /// изменяемые заимствования одного контекста, и вернуть их по отдельности
    /// нельзя.
    ///
    /// Набор ФИКСИРОВАН: `runtime_shapes`, `stdout`, `stderr`, `zone` и
    /// необязательный `function_caller`. Потоки и зона обязательны — на
    /// нативном пути их нет, и построить `ExecutionParts` не выйдет.
    ///
    /// # Errors
    ///
    /// [`RtError::CapabilityMissing`], если вывод, поток ошибок или зона
    /// отсутствуют (нативный путь), — плюс любая ошибка самого замыкания.
    pub fn with_execution_parts<R>(
        &mut self,
        body: impl FnOnce(ExecutionParts<'_, 'a>) -> RtResult<R>,
    ) -> RtResult<R> {
        let path = self.path;
        let stdout = self
            .stdout
            .as_deref_mut()
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::Stdout,
                path,
            })?;
        let stderr = self
            .stderr
            .as_deref_mut()
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::Stderr,
                path,
            })?;
        let zone = self
            .zone
            .map(Rc::as_ref)
            .ok_or(RtError::CapabilityMissing {
                capability: Capability::Zone,
                path,
            })?;
        let function_caller = self.function_caller.as_deref_mut();
        body(ExecutionParts {
            runtime_shapes: &mut *self.runtime_shapes,
            stdout,
            stderr,
            zone,
            function_caller,
        })
    }
}

/// Разделённые изменяемые сервисы, выданные [`CallContext::with_execution_parts`]
/// на время одного замыкания. Набор фиксирован; `files` и `random` здесь нет намеренно:
/// конструкторы получают их отдельными аксессорами и не делят изменяемые заимствования с этой
/// составной операцией.
pub struct ExecutionParts<'ctx, 'a> {
    pub runtime_shapes: &'ctx mut RuntimeShapes,
    pub stdout: &'ctx mut dyn Write,
    pub stderr: &'ctx mut dyn Write,
    pub zone: &'ctx dyn crate::TimeZone,
    pub function_caller: Option<&'ctx mut FunctionCaller<'a>>,
}

/// Единый ABI статически зарегистрированной функции или конструктора.
pub type ComponentCall = for<'a> fn(&mut CallContext<'a>, &[BslValue]) -> RtResult<BslValue>;

/// Вызов метода объекта компонента. Получатель — сам объект за
/// трейт-объектом: VM отдаёт его без пересборки обёртки значения, а
/// строковый вход по умолчанию приводит `self` через `as_dyn`.
/// Обработчик возвращается к конкретному типу даункастом
/// `receiver.as_any().downcast_ref::<T>()`.
pub type MethodCall =
    for<'a> fn(&dyn crate::ObjectProtocol, &[BslValue], &mut CallContext<'a>) -> RtResult<BslValue>;

/// Статический дескриптор метода объекта компонента — как
/// [`FunctionDescriptor`] для глобальных функций. Непустая таблица методов
/// типа (см. `ObjectProtocol::method_table`) включает быстрый путь VM
/// «номер имени → обработчик» без строковых операций на вызове.
///
/// Арность у метода — рантаймная, а не статическая. Статической точки нет:
/// получатель известен только в исполнении, а имена делятся между типами
/// (`Открыть` есть и у менеджера потоков, и у читателя архива), поэтому
/// код вызова не знает, чью арность проверять. Зато рантаймная точка есть
/// и одна — там, где получатель уже выбран: [`crate::call_method_from_table`]
/// и арм `CallObjectMethod` в VM сверяют число аргументов с полем `arity`
/// до вызова обработчика. Измерено (`OBJ.METHOD.EXTRA_ARGS`): платформа
/// отвечает ошибкой и на лишний, и на недостающий аргумент — прежнее
/// допущение «тексты ошибок о числе аргументов уже живут в самих
/// обработчиках» опровергнуто (`ТабличныйДокумент.НачатьГруппуСтрок` молча
/// принимал пять аргументов при двух объявленных).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MethodDescriptor {
    names: &'static [&'static str],
    arity: Arity,
    call: MethodCall,
}

impl MethodDescriptor {
    /// Дескриптор метода. Кода здесь нет (см. `MethodCode`, удалённый в
    /// ABI-F): связать байт-код по коду метода нельзя, а `arity` проверяется
    /// в рантайме, где получатель уже известен.
    pub const fn new(names: &'static [&'static str], arity: Arity, call: MethodCall) -> Self {
        Self { names, arity, call }
    }

    /// Написания метода — первое каноническое (русское).
    pub const fn names(&self) -> &'static [&'static str] {
        self.names
    }

    /// Допустимое число аргументов.
    pub const fn arity(&self) -> Arity {
        self.arity
    }

    /// Обработчик вызова. Нужен VM: кэш `CallObjectMethod` держит дескриптор
    /// и на попадании берёт из него обработчик, не разбирая имя (см.
    /// `resolve_component_method` в `bsl-vm`).
    pub const fn call(&self) -> MethodCall {
        self.call
    }

    /// Рантаймная проверка арности перед вызовом обработчика — единый
    /// источник для всех путей диспетчеризации: `call_method_from_table`
    /// (строковый путь и откат шимов JIT), арм `CallObjectMethod` VM и его
    /// шим JIT. Живёт здесь, а не в `bsl-vm`: горячий цикл VM на грани кеша
    /// микроопераций, и лишняя функция в его `lib.rs` сдвигает укладку кода
    /// (измерено на `call_overhead`). Измерено `OBJ.METHOD.EXTRA_ARGS`:
    /// платформа отвечает ошибкой и на лишний, и на недостающий аргумент;
    /// ошибка — [`RtError::MethodNotApplicable`], ловимая `Попыткой`.
    ///
    /// # Errors
    ///
    /// [`RtError::MethodNotApplicable`], если `count` не в диапазоне `arity`.
    #[inline]
    pub fn check_arity(&self, count: u8, receiver: &'static str) -> RtResult<()> {
        if self.arity.accepts(count) {
            Ok(())
        } else {
            Err(crate::RtError::MethodNotApplicable {
                method: self.names.first().copied().unwrap_or("метод"),
                receiver,
            })
        }
    }
}

/// Чтение свойства объекта компонента. Получатель — как у
/// [`MethodCall`]: сам объект, без обёртки значения.
pub type PropertyGet =
    for<'a> fn(&dyn crate::ObjectProtocol, &mut CallContext<'a>) -> RtResult<BslValue>;

/// Запись свойства объекта компонента.
pub type PropertySet =
    for<'a> fn(&dyn crate::ObjectProtocol, BslValue, &mut CallContext<'a>) -> RtResult<()>;

/// Статический дескриптор свойства — как [`MethodDescriptor`] для методов.
/// Непустая таблица свойств типа (см. `ObjectProtocol::property_table`)
/// включает быстрый путь VM «номер имени → обработчик»: строка разбирается
/// один раз на пару «тип, имя», дальше доступ идёт без строковых операций.
#[derive(Debug, Clone, Copy)]
pub struct PropertyDescriptor {
    pub names: &'static [&'static str],
    pub get: PropertyGet,
    /// `None` — свойство только для чтения: запись отвечает
    /// [`crate::RtError::PropertyReadOnly`].
    pub set: Option<PropertySet>,
}

/// Поиск свойства в таблице по имени. Сворачивание — [`crate::folded_eq`],
/// единственный судья равенства имён в рантайме.
fn find_property(
    table: &'static [PropertyDescriptor],
    name: &str,
) -> Option<&'static PropertyDescriptor> {
    table.iter().find(|descriptor| {
        descriptor
            .names
            .iter()
            .any(|candidate| crate::folded_eq(candidate, name))
    })
}

/// Чтение свойства по статической таблице — реализация `get_property` по
/// умолчанию и вход для доступа с именем-строкой.
///
/// # Errors
///
/// [`crate::RtError::UnknownProperty`], если имени нет в таблице; ошибки
/// самого обработчика — как есть.
pub fn get_property_from_table(
    table: &'static [PropertyDescriptor],
    type_name: &'static str,
    receiver: &dyn crate::ObjectProtocol,
    name: &str,
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match find_property(table, name) {
        Some(descriptor) => (descriptor.get)(receiver, context),
        None => {
            let _ = type_name;
            Err(crate::RtError::UnknownProperty(name.to_string()))
        }
    }
}

/// Запись свойства по статической таблице.
///
/// # Errors
///
/// [`crate::RtError::UnknownProperty`], если имени нет в таблице;
/// [`crate::RtError::PropertyReadOnly`], если у свойства нет обработчика
/// записи.
pub fn set_property_from_table(
    table: &'static [PropertyDescriptor],
    type_name: &'static str,
    receiver: &dyn crate::ObjectProtocol,
    name: &str,
    value: BslValue,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    match find_property(table, name) {
        Some(descriptor) => match descriptor.set {
            Some(set) => set(receiver, value, context),
            None => Err(crate::RtError::PropertyReadOnly {
                property: name.to_string(),
                receiver: type_name,
            }),
        },
        None => Err(crate::RtError::UnknownProperty(name.to_string())),
    }
}

/// Диспетчеризация вызова по статической таблице методов для входов с
/// именем-строкой: реализация `call_method` конвертированного типа. Имя
/// сравнивается без учёта регистра, как в остальных таблицах имён.
///
/// # Errors
///
/// [`crate::RtError::UnknownMethod`], если имени нет в таблице; ошибки самого
/// метода — как есть.
pub fn call_method_from_table(
    table: &'static [MethodDescriptor],
    type_name: &'static str,
    receiver: &dyn crate::ObjectProtocol,
    name: &str,
    arguments: &[BslValue],
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    // Судья равенства имён один — [`crate::folded_eq`]. Он и без аллокаций
    // на общем пути (побайтовое сравнение, свёрнутая строка лишь на входе
    // вне быстрых алфавитов — см. `fold.rs`), поэтому путь JIT-шимов, где
    // имя приходит на каждый вызов, платит не больше прежнего посимвольного
    // сравнения через `to_uppercase`, которому он к тому же тождествен.
    for descriptor in table {
        if descriptor
            .names
            .iter()
            .any(|candidate| crate::folded_eq(candidate, name))
        {
            // Рантаймная проверка арности до вызова обработчика — та же
            // [`MethodDescriptor::check_arity`], что в арме `CallObjectMethod`
            // у VM. Этот путь проходят строковый `call_method` и его откат в
            // шимах JIT.
            let count = u8::try_from(arguments.len()).unwrap_or(u8::MAX);
            descriptor.check_arity(count, type_name)?;
            return (descriptor.call)(receiver, arguments, context);
        }
    }
    Err(crate::RtError::UnknownMethod {
        method: name.to_string(),
        receiver: type_name,
    })
}

/// Как функцию компонента разрешено звать.
///
/// `Intrinsic` — форма, которую платформа отвергает В ПОЗИЦИИ ОПЕРАТОРА
/// (резолвер проверяет это в `AStmt::ExprStmt`). Ни один дескриптор в
/// дереве её сегодня не выставляет, и это ИЗМЕРЕНО, а не забыто:
/// `measure-stmtcall.platform.txt` показывает, что 8.3.27 отвергает
/// `Acos(1);`, но принимает `СтрНайтиПоРегулярномуВыражению("аб", "а");`
/// — то есть правило распространяется не на все встроенные функции, и
/// нашим компонентным как раз подходит `Function`. Вариант остаётся
/// точкой расширения для функции, которую платформа оператором не
/// пропустит.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Function,
    Procedure,
    Intrinsic,
}

#[derive(Debug, Clone, Copy)]
pub struct FunctionDescriptor {
    pub code: FunctionCode,
    pub names: &'static [&'static str],
    pub arity: Arity,
    pub kind: FunctionKind,
    pub call: ComponentCall,
}

#[derive(Debug, Clone, Copy)]
pub struct ConstructorDescriptor {
    pub code: ConstructorCode,
    pub names: &'static [&'static str],
    pub arity: Arity,
    pub call: ComponentCall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryDependency {
    pub package: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct LibraryDescriptor {
    package: &'static str,
    /// Годятся ли объекты этой библиотеки НАТИВНОМУ пути исполнения.
    ///
    /// JIT обслуживает обращения к объектам сокращённым [`CallContext`]:
    /// потоки вывода в нём — стоки, зоны прогона нет. Официальным
    /// компонентам этого хватает — их методы и свойства ни в вывод, ни в
    /// зону не ходят, — но реестр открыт, и у стороннего типа обработчик
    /// вправе делать и то и другое. Библиотека объявляет это сама, а
    /// связывание сводит объявления реестра к одному признаку
    /// ([`RuntimeRegistry::has_full_context_objects`]): при нём чанк,
    /// обращающийся к ОБЪЕКТАМ, целиком минует нативный путь. Различать
    /// получателей по типу было бы точнее, но цена этого измерена и
    /// велика — см. `LinkedComponents` в `bsl-vm`.
    object_context: ObjectContextNeed,
    version: &'static str,
    dependencies: &'static [LibraryDependency],
    functions: &'static [FunctionDescriptor],
    constructors: &'static [ConstructorDescriptor],
    /// Типы объектов, которые компонент вводит в язык. По ним `Тип("Имя")`
    /// находит тип: закрытый реестр `TypeId` ядра компонентных типов
    /// больше не знает. Не объявленный здесь тип остаётся доступен через
    /// `ТипЗнч(объект)`, но по имени не ищется.
    types: &'static [&'static crate::TypeDescriptor],
    /// Написания, на которые откликается больше одного типа: пара
    /// «псевдоним → тип-владелец» (см. ABI-D). Пусто, пока конфликта имён
    /// нет; каталог типов реестра разрешает неоднозначность по этому списку,
    /// а не по порядку `types`.
    type_aliases: &'static [(&'static str, &'static crate::TypeDescriptor)],
}

impl LibraryDescriptor {
    /// Обязательный минимум библиотеки. Остальные таблицы добавляются
    /// `with_*`; `object_context` входит сюда, а не в умолчание, — потребность
    /// объектных обработчиков в полном контексте каждая библиотека объявляет
    /// явно.
    pub const fn new(
        package: &'static str,
        version: &'static str,
        object_context: ObjectContextNeed,
    ) -> Self {
        Self {
            package,
            object_context,
            version,
            dependencies: &[],
            functions: &[],
            constructors: &[],
            types: &[],
            type_aliases: &[],
        }
    }

    /// Зависимости от других библиотек (ядро не объявляется — реестр
    /// включает его в требования любой программы).
    pub const fn with_dependencies(mut self, d: &'static [LibraryDependency]) -> Self {
        self.dependencies = d;
        self
    }

    /// Глобальные функции библиотеки.
    pub const fn with_functions(mut self, f: &'static [FunctionDescriptor]) -> Self {
        self.functions = f;
        self
    }

    /// Конструкторы (`Новый ...`) библиотеки.
    pub const fn with_constructors(mut self, c: &'static [ConstructorDescriptor]) -> Self {
        self.constructors = c;
        self
    }

    /// Типы объектов, которые библиотека вводит в язык.
    pub const fn with_types(mut self, t: &'static [&'static crate::TypeDescriptor]) -> Self {
        self.types = t;
        self
    }

    /// Написания, на которые откликается больше одного типа (см. ABI-D).
    pub const fn with_type_aliases(
        mut self,
        a: &'static [(&'static str, &'static crate::TypeDescriptor)],
    ) -> Self {
        self.type_aliases = a;
        self
    }

    /// Имя пакета библиотеки.
    pub const fn package(&self) -> &'static str {
        self.package
    }

    /// Версия библиотеки.
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// Какой контекст прогона нужен объектным обработчикам библиотеки.
    pub const fn object_context(&self) -> ObjectContextNeed {
        self.object_context
    }

    /// Зависимости библиотеки.
    pub const fn dependencies(&self) -> &'static [LibraryDependency] {
        self.dependencies
    }

    /// Глобальные функции библиотеки.
    pub const fn functions(&self) -> &'static [FunctionDescriptor] {
        self.functions
    }

    /// Конструкторы библиотеки.
    pub const fn constructors(&self) -> &'static [ConstructorDescriptor] {
        self.constructors
    }

    /// Типы, которые библиотека вводит в язык.
    pub const fn types(&self) -> &'static [&'static crate::TypeDescriptor] {
        self.types
    }

    /// Псевдонимы имён типов, владельцев которых объявила библиотека.
    pub const fn type_aliases(&self) -> &'static [(&'static str, &'static crate::TypeDescriptor)] {
        self.type_aliases
    }
}

/// Какой контекст прогона нужен обработчикам объектов этой библиотеки.
///
/// Компонент объявляет СВОЮ потребность, а не устройство движка: как
/// именно тот исполнит обращение — его дело, и в этом ABI ему нет имени.
///
/// Умолчания у признака нет намеренно: поле обязательное, и автор
/// компонента отвечает на вопрос осознанно. Ошибиться в безопасную сторону
/// всегда можно — [`ObjectContextNeed::Full`] стоит скорости, но не
/// корректности.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectContextNeed {
    /// Обработчикам нужен полный контекст: они читают хотя бы одну его внешнюю
    /// возможность. Движок обязан дать такой контекст или отступить на путь, где он есть.
    Full,
    /// Обработчики обходятся сокращённым контекстом и не читают его внешние
    /// возможности.
    Reduced,
}

fn construct_binary_data(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let path = arguments.first().ok_or(RtError::InvalidBytecode(
        "конструктор ДвоичныеДанные вызван без аргумента",
    ))?;
    BslValue::new_binary_data(path, context.files()?)
}

fn construct_uuid(context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    let undefined = BslValue::Undefined;
    let argument = arguments.first().unwrap_or(&undefined);
    BslValue::new_uuid(argument, context.random()?)
}

const CORE_CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ДвоичныеДанные", "BinaryData"],
        arity: Arity::exact(1),
        call: construct_binary_data,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["УникальныйИдентификатор", "UUID"],
        arity: Arity::range(0, 1),
        call: construct_uuid,
    },
];

/// Дескриптор базового компонента. На переходном этапе встроенные функции
/// ещё обслуживаются старой таблицей; конструкторы, которым нужны
/// возможности прогона, проходят обычную компонентную границу.
pub const fn core_library() -> LibraryDescriptor {
    LibraryDescriptor::new(
        crate::PACKAGE_NAME,
        crate::PACKAGE_VERSION,
        ObjectContextNeed::Reduced,
    )
    .with_constructors(CORE_CONSTRUCTORS)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    MissingCore,
    TooManyLibraries,
    EmptyIdentity,
    DuplicatePackage(String),
    /// Два компонента объявили одно каноническое `name` типа.
    DuplicateTypeName(String),
    /// Компонент объявил каноническим именем типа то, которым назван тип
    /// ядра: имя ядра нельзя тихо перекрыть регистрацией библиотеки.
    TypeShadowsCore(String),
    /// На написание откликается больше одного типа, а владельца никто не
    /// объявил, — прежде это разрешалось порядком списка, то есть молча.
    AmbiguousTypeAlias(String),
    /// Владелец псевдонима объявлен, но его дескриптора нет в
    /// `LibraryDescriptor::types` этой же библиотеки. Проверяется
    /// `std::ptr::eq`, а не структурным равенством: у `TypeDescriptor`
    /// структурный `PartialEq`, и вторая статика с теми же полями прошла
    /// бы проверку, не будучи зарегистрированной.
    AliasOwnerNotDeclared {
        alias: String,
        package: String,
    },
    /// Одно написание объявлено собственным больше одного раза.
    DuplicateAliasOwner(String),
    /// Владелец объявлен, но сам на это написание не откликается
    /// (`TypeDescriptor::answers_to` ложно): запись не разрешает
    /// неоднозначность, а вводит имя, которого у типа нет.
    AliasOwnerDoesNotAnswer {
        alias: String,
        type_name: String,
    },
    /// Написание объявлено в `type_aliases`, хотя неоднозначным не
    /// является: таблица — разрешение конфликта, а не второй источник имён
    /// (в том числе для написания, которым владеет тип ядра).
    AliasIsNotAmbiguous(String),
    DuplicateFunctionCode {
        package: String,
        code: FunctionCode,
    },
    DuplicateConstructorCode {
        package: String,
        code: ConstructorCode,
    },
    DuplicateFunctionName(String),
    DuplicateConstructorName(String),
    EmptyNames {
        package: String,
        code: u16,
    },
    MissingDependency {
        package: String,
        dependency: String,
    },
    DependencyVersion {
        package: String,
        dependency: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCore => write!(f, "компонент bsl-rt не зарегистрирован"),
            Self::TooManyLibraries => write!(f, "число компонентов не помещается в индекс u8"),
            Self::EmptyIdentity => write!(f, "имя пакета и версия не могут быть пустыми"),
            Self::DuplicatePackage(package) => {
                write!(f, "компонент {package} зарегистрирован дважды")
            }
            Self::DuplicateFunctionCode { package, code } => write!(
                f,
                "код функции {} повторён в компоненте {package}",
                code.get()
            ),
            Self::DuplicateConstructorCode { package, code } => write!(
                f,
                "код конструктора {} повторён в компоненте {package}",
                code.get()
            ),
            Self::DuplicateFunctionName(name) => {
                write!(f, "имя глобальной функции {name} зарегистрировано дважды")
            }
            Self::DuplicateConstructorName(name) => {
                write!(f, "имя конструктора {name} зарегистрировано дважды")
            }
            Self::EmptyNames { package, code } => {
                write!(f, "запись {package}/{code} не содержит ни одного имени")
            }
            Self::MissingDependency {
                package,
                dependency,
            } => write!(f, "компоненту {package} необходим {dependency}"),
            Self::DependencyVersion {
                package,
                dependency,
                expected,
                actual,
            } => write!(
                f,
                "компоненту {package} необходим {dependency}={expected}, зарегистрирован {actual}"
            ),
            Self::DuplicateTypeName(name) => {
                write!(f, "имя типа {name} объявлено дважды")
            }
            Self::TypeShadowsCore(name) => {
                write!(f, "имя типа {name} уже принадлежит типу ядра")
            }
            Self::AmbiguousTypeAlias(name) => write!(
                f,
                "на написание {name} откликается больше одного типа, владелец не объявлен"
            ),
            Self::AliasOwnerNotDeclared { alias, package } => write!(
                f,
                "владелец псевдонима {alias} не объявлен в types компонента {package}"
            ),
            Self::DuplicateAliasOwner(name) => {
                write!(f, "владелец написания {name} объявлен дважды")
            }
            Self::AliasOwnerDoesNotAnswer { alias, type_name } => write!(
                f,
                "тип {type_name} объявлен владельцем {alias}, но на это написание не откликается"
            ),
            Self::AliasIsNotAmbiguous(name) => {
                write!(
                    f,
                    "написание {name} объявлено псевдонимом, но неоднозначным не является"
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// Проверенный каталог типов компонентов: свёрнутое написание → дескриптор,
/// каждое написание разрешено ровно в один тип. Строится один раз в
/// [`RuntimeBuilder::build`] и дальше неизменяем; [`RuntimeShapes`] несёт
/// его `Arc` весь прогон (именно `Arc`, а не `Rc`: реестр обязан оставаться
/// `Send`/`Sync` — движок фасада держит его за `Arc`, хотя сам прогон
/// однопоточный). Приоритет разрешения зашит в ПОСТРОЕНИЕ, а не в
/// вызывающих: тип ядра (через `TypeId::lookup` в `resolve_type`) → затем
/// этот каталог, где каноническое имя и объявленный владелец псевдонима
/// уже разведены.
#[derive(Debug, Default)]
pub(crate) struct TypeCatalog {
    by_spelling: HashMap<String, &'static crate::TypeDescriptor>,
}

impl TypeCatalog {
    /// Тип по написанию — регистронезависимо и без учёта пробелов, тем же
    /// судьёй [`crate::types::squash`], что и `TypeDescriptor::answers_to`,
    /// иначе проверка разошлась бы с поиском на «ЧтениеXML» против
    /// «Чтение XML».
    pub(crate) fn resolve(&self, name: &str) -> Option<&'static crate::TypeDescriptor> {
        self.by_spelling.get(&crate::types::squash(name)).copied()
    }
}

/// Спелинги, на которые откликается тип: каноническое имя, представление и
/// дополнительные написания. Тот же набор, что проверяет `answers_to`.
fn type_spellings(ty: &'static crate::TypeDescriptor) -> impl Iterator<Item = &'static str> {
    [ty.name, ty.type_display]
        .into_iter()
        .chain(ty.type_names.iter().copied())
}

/// Строит проверенный каталог типов из объявлений библиотек. Порядок
/// проверок — модель псевдонимов ABI-D: сначала запрет затенения ядра и
/// повтора канонических имён, потом объявленные владельцы, потом разрешение
/// написаний; каждое правило даёт свой [`RegistryError`].
///
/// # Errors
///
/// Любой из вариантов [`RegistryError`], относящихся к типам и псевдонимам.
fn build_type_catalog(libraries: &[LibraryDescriptor]) -> Result<TypeCatalog, RegistryError> {
    // Написание (свёрнутое) -> типы, которые на него откликаются, и одно
    // исходное написание для текста ошибки.
    let mut answered: HashMap<String, (Vec<&'static crate::TypeDescriptor>, String)> =
        HashMap::new();
    // Свёрнутое каноническое имя -> тип: ловит повтор канонического имени.
    let mut canonical: HashMap<String, &'static crate::TypeDescriptor> = HashMap::new();

    for library in libraries {
        for &ty in library.types() {
            // Правило 1: каноническое имя, совпавшее с ЛЮБЫМ написанием типа
            // ядра, — отказ. Тихо перекрыть имя ядра регистрацией нельзя.
            if crate::TypeId::lookup(ty.name).is_some() {
                return Err(RegistryError::TypeShadowsCore(ty.name.to_string()));
            }
            let canon = crate::types::squash(ty.name);
            match canonical.get(&canon) {
                Some(prev) if !std::ptr::eq(*prev, ty) => {
                    return Err(RegistryError::DuplicateTypeName(ty.name.to_string()));
                }
                Some(_) => {}
                None => {
                    canonical.insert(canon, ty);
                }
            }
            for spelling in type_spellings(ty) {
                let entry = answered
                    .entry(crate::types::squash(spelling))
                    .or_insert_with(|| (Vec::new(), spelling.to_string()));
                if !entry.0.iter().any(|t| std::ptr::eq(*t, ty)) {
                    entry.0.push(ty);
                }
            }
        }
    }

    // Объявленные владельцы псевдонимов.
    let mut declared: HashMap<String, (&'static crate::TypeDescriptor, String)> = HashMap::new();
    for library in libraries {
        for &(alias, owner) in library.type_aliases() {
            if !library.types().iter().any(|t| std::ptr::eq(*t, owner)) {
                return Err(RegistryError::AliasOwnerNotDeclared {
                    alias: alias.to_string(),
                    package: library.package.to_string(),
                });
            }
            if !owner.answers_to(alias) {
                return Err(RegistryError::AliasOwnerDoesNotAnswer {
                    alias: alias.to_string(),
                    type_name: owner.name.to_string(),
                });
            }
            if declared
                .insert(crate::types::squash(alias), (owner, alias.to_string()))
                .is_some()
            {
                return Err(RegistryError::DuplicateAliasOwner(alias.to_string()));
            }
        }
    }

    // Псевдоним обязан разрешать РЕАЛЬНУЮ неоднозначность: не написание ядра
    // (там владелец — само ядро, порядок в `resolve_type`) и не написание,
    // на которое откликается один тип.
    for (sq, (_owner, original)) in &declared {
        let ambiguous = answered.get(sq).map(|(t, _)| t.len()).unwrap_or(0) > 1;
        if crate::TypeId::lookup(sq).is_some() || !ambiguous {
            return Err(RegistryError::AliasIsNotAmbiguous(original.clone()));
        }
    }

    // Разрешение написаний в дескрипторы.
    let mut by_spelling = HashMap::new();
    for (sq, (types, original)) in &answered {
        // Правило 2: написание, которым владеет тип ядра, в компонентный
        // каталог не попадает — молча, ядро выигрывает в `resolve_type`.
        if crate::TypeId::lookup(sq).is_some() {
            continue;
        }
        if types.len() == 1 {
            by_spelling.insert(sq.clone(), types[0]);
        } else {
            match declared.get(sq) {
                Some((owner, _)) => {
                    by_spelling.insert(sq.clone(), *owner);
                }
                None => return Err(RegistryError::AmbiguousTypeAlias(original.clone())),
            }
        }
    }

    Ok(TypeCatalog { by_spelling })
}

/// Изменяемая стадия композиции runtime-компонентов.
#[derive(Default)]
pub struct RuntimeBuilder {
    libraries: Vec<LibraryDescriptor>,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, library: LibraryDescriptor) -> &mut Self {
        self.libraries.push(library);
        self
    }

    /// Проверяет композицию, назначает детерминированные локальные индексы
    /// библиотек и замораживает реестр.
    ///
    /// # Errors
    ///
    /// Возвращает [`RegistryError`] при конфликте пакетов, кодов или имён,
    /// а также при отсутствующей либо несовместимой зависимости.
    pub fn build(mut self) -> Result<RuntimeRegistry, RegistryError> {
        if self.libraries.len() > u8::MAX as usize + 1 {
            return Err(RegistryError::TooManyLibraries);
        }
        if self
            .libraries
            .iter()
            .all(|library| library.package != crate::PACKAGE_NAME)
        {
            return Err(RegistryError::MissingCore);
        }
        if self
            .libraries
            .iter()
            .any(|library| library.package.is_empty() || library.version.is_empty())
        {
            return Err(RegistryError::EmptyIdentity);
        }
        self.libraries.sort_by(|a, b| {
            match (
                a.package == crate::PACKAGE_NAME,
                b.package == crate::PACKAGE_NAME,
            ) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.package.cmp(b.package),
            }
        });

        for pair in self.libraries.windows(2) {
            if pair[0].package == pair[1].package {
                return Err(RegistryError::DuplicatePackage(pair[0].package.to_string()));
            }
        }

        let versions: HashMap<&str, &str> = self
            .libraries
            .iter()
            .map(|library| (library.package, library.version))
            .collect();
        for library in &self.libraries {
            for dependency in library.dependencies {
                match versions.get(dependency.package) {
                    None => {
                        return Err(RegistryError::MissingDependency {
                            package: library.package.to_string(),
                            dependency: dependency.package.to_string(),
                        });
                    }
                    Some(actual) if *actual != dependency.version => {
                        return Err(RegistryError::DependencyVersion {
                            package: library.package.to_string(),
                            dependency: dependency.package.to_string(),
                            expected: dependency.version.to_string(),
                            actual: (*actual).to_string(),
                        });
                    }
                    Some(_) => {}
                }
            }
        }

        let mut function_names = HashMap::new();
        let mut constructor_names = HashMap::new();
        for (library_index, library) in self.libraries.iter().enumerate() {
            let mut function_codes = HashMap::new();
            for function in library.functions {
                if function.names.is_empty() {
                    return Err(RegistryError::EmptyNames {
                        package: library.package.to_string(),
                        code: function.code.get(),
                    });
                }
                if function_codes.insert(function.code, ()).is_some() {
                    return Err(RegistryError::DuplicateFunctionCode {
                        package: library.package.to_string(),
                        code: function.code,
                    });
                }
                for name in function.names {
                    let key = name.to_uppercase();
                    if function_names
                        .insert(key.clone(), (library_index as u8, function.code))
                        .is_some()
                    {
                        return Err(RegistryError::DuplicateFunctionName(name.to_string()));
                    }
                }
            }

            let mut constructor_codes = HashMap::new();
            for constructor in library.constructors {
                if constructor.names.is_empty() {
                    return Err(RegistryError::EmptyNames {
                        package: library.package.to_string(),
                        code: constructor.code.get(),
                    });
                }
                if constructor_codes.insert(constructor.code, ()).is_some() {
                    return Err(RegistryError::DuplicateConstructorCode {
                        package: library.package.to_string(),
                        code: constructor.code,
                    });
                }
                for name in constructor.names {
                    let key = name.to_uppercase();
                    if constructor_names
                        .insert(key.clone(), (library_index as u8, constructor.code))
                        .is_some()
                    {
                        return Err(RegistryError::DuplicateConstructorName(name.to_string()));
                    }
                }
            }
        }

        let type_catalog = Arc::new(build_type_catalog(&self.libraries)?);

        Ok(RuntimeRegistry {
            libraries: self.libraries,
            function_names,
            constructor_names,
            type_catalog,
        })
    }
}

/// Неизменяемый каталог собранного `Engine`.
pub struct RuntimeRegistry {
    libraries: Vec<LibraryDescriptor>,
    function_names: HashMap<String, (u8, FunctionCode)>,
    constructor_names: HashMap<String, (u8, ConstructorCode)>,
    type_catalog: Arc<TypeCatalog>,
}

impl RuntimeRegistry {
    /// Есть ли библиотека, объявившая
    /// [`ObjectContextNeed::Full`]: её обработчикам нужен полный
    /// контекст исполнения.
    #[must_use]
    pub fn has_full_context_objects(&self) -> bool {
        self.libraries
            .iter()
            .any(|library| library.object_context == ObjectContextNeed::Full)
    }

    pub fn libraries(&self) -> &[LibraryDescriptor] {
        &self.libraries
    }

    pub fn library(&self, index: u8) -> Option<&LibraryDescriptor> {
        self.libraries.get(index as usize)
    }

    pub fn library_by_package(&self, package: &str) -> Option<&LibraryDescriptor> {
        self.libraries
            .iter()
            .find(|library| library.package == package)
    }

    pub fn lookup_function(&self, name: &str) -> Option<(u8, FunctionCode)> {
        self.function_names.get(&name.to_uppercase()).copied()
    }

    pub fn lookup_constructor(&self, name: &str) -> Option<(u8, ConstructorCode)> {
        self.constructor_names.get(&name.to_uppercase()).copied()
    }

    /// Все типы, объявленные библиотеками реестра.
    pub fn types(&self) -> impl Iterator<Item = &'static crate::TypeDescriptor> + '_ {
        self.libraries
            .iter()
            .flat_map(|library| library.types().iter().copied())
    }

    /// Проверенный каталог типов — мост внутри крейта: поля реестра закрыты,
    /// а каталог нужен [`RuntimeShapes::seeded`] из соседнего модуля.
    pub(crate) fn type_catalog(&self) -> Arc<TypeCatalog> {
        Arc::clone(&self.type_catalog)
    }

    pub fn requirements_for(
        &self,
        used: impl IntoIterator<Item = LibraryKey>,
    ) -> Vec<LibraryRequirement> {
        let mut required = std::collections::HashSet::new();
        required.insert(crate::PACKAGE_NAME);
        for key in used {
            required.insert(
                self.libraries
                    .iter()
                    .find(|library| library.package == key.as_str())
                    .expect("ключ получен из этого реестра")
                    .package,
            );
        }

        loop {
            let before = required.len();
            for library in &self.libraries {
                if required.contains(library.package) {
                    for dependency in library.dependencies {
                        required.insert(dependency.package);
                    }
                }
            }
            if required.len() == before {
                break;
            }
        }

        self.libraries
            .iter()
            .filter(|library| required.contains(library.package))
            .map(|library| LibraryRequirement::new(library.package, library.version))
            .collect()
    }

    pub fn function(&self, library: u8, code: FunctionCode) -> Option<&FunctionDescriptor> {
        self.library(library)?
            .functions
            .iter()
            .find(|function| function.code == code)
    }

    pub fn constructor(
        &self,
        library: u8,
        code: ConstructorCode,
    ) -> Option<&ConstructorDescriptor> {
        self.library(library)?
            .constructors
            .iter()
            .find(|constructor| constructor.code == code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RtError;

    fn no_call(_ctx: &mut CallContext<'_>, _args: &[BslValue]) -> RtResult<BslValue> {
        Err(RtError::DynamicError(
            "не вызывается в тесте реестра".to_string(),
        ))
    }

    const CORE_FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["Сообщить", "Message"],
        arity: Arity::exact(1),
        kind: FunctionKind::Procedure,
        call: no_call,
    }];
    const JSON_CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ЧтениеJSON", "JSONReader"],
        arity: Arity::exact(0),
        call: no_call,
    }];

    fn core() -> LibraryDescriptor {
        LibraryDescriptor::new(
            crate::PACKAGE_NAME,
            crate::PACKAGE_VERSION,
            ObjectContextNeed::Reduced,
        )
        .with_functions(CORE_FUNCTIONS)
    }

    fn json() -> LibraryDescriptor {
        LibraryDescriptor::new("bsl-json", "0.1.0", ObjectContextNeed::Reduced)
            .with_dependencies(&[LibraryDependency {
                package: crate::PACKAGE_NAME,
                version: crate::PACKAGE_VERSION,
            }])
            .with_constructors(JSON_CONSTRUCTORS)
    }

    #[test]
    fn registry_is_sorted_and_aliases_resolve_to_one_code() {
        let mut builder = RuntimeBuilder::new();
        builder.register(json()).register(core());
        let registry = builder.build().unwrap();

        assert_eq!(registry.library(0).unwrap().package, crate::PACKAGE_NAME);
        assert_eq!(registry.library(1).unwrap().package, "bsl-json");
        assert_eq!(
            registry.lookup_function("message"),
            Some((0, FunctionCode::new(1)))
        );
        assert_eq!(
            registry.lookup_constructor("чтениеjson"),
            Some((1, ConstructorCode::new(1)))
        );
    }

    #[test]
    fn duplicate_function_alias_is_rejected_case_insensitively() {
        const DUPLICATE: &[FunctionDescriptor] = &[FunctionDescriptor {
            code: FunctionCode::new(2),
            names: &["message"],
            arity: Arity::exact(1),
            kind: FunctionKind::Function,
            call: no_call,
        }];
        let mut builder = RuntimeBuilder::new();
        builder.register(core()).register(
            LibraryDescriptor::new("other", "1.0.0", ObjectContextNeed::Reduced)
                .with_functions(DUPLICATE),
        );

        assert!(matches!(
            builder.build(),
            Err(RegistryError::DuplicateFunctionName(_))
        ));
    }

    #[test]
    fn dependency_version_is_exact() {
        let mut bad_json = json();
        bad_json.dependencies = &[LibraryDependency {
            package: crate::PACKAGE_NAME,
            version: "9.9.9",
        }];
        let mut builder = RuntimeBuilder::new();
        builder.register(core()).register(bad_json);

        assert!(matches!(
            builder.build(),
            Err(RegistryError::DependencyVersion { .. })
        ));
    }

    /// Нативный путь (JIT-шимы) не несёт возможностей: обращение к выводу
    /// или зоне отвечает ОДНОЙ формой отказа `CapabilityMissing` с пометкой
    /// пути — а не молчаливым стоком (прежнее поведение) и не чужим
    /// временем. Это наблюдаемая цель ABI-A.
    #[test]
    fn a_native_context_reports_capability_missing() {
        let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut context = CallContext::native(&mut shapes, |_v, _s| unreachable!());
        assert!(matches!(
            context.stdout(),
            Err(RtError::CapabilityMissing {
                capability: Capability::Stdout,
                path: ContextKind::Reduced,
            })
        ));
        assert!(matches!(
            context.zone(),
            Err(RtError::CapabilityMissing {
                capability: Capability::Zone,
                path: ContextKind::Reduced,
            })
        ));
        assert!(matches!(
            context.random(),
            Err(RtError::CapabilityMissing {
                capability: Capability::Random,
                path: ContextKind::Reduced,
            })
        ));
        assert!(matches!(
            context.with_execution_parts(|_parts| Ok(())),
            Err(RtError::CapabilityMissing {
                path: ContextKind::Reduced,
                ..
            })
        ));
    }
}

#[cfg(test)]
mod descriptor_sizes {
    use super::{LibraryDescriptor, MethodDescriptor};

    /// Размеры дескрипторов зафиксированы намеренно (ABI-E плана
    /// abi-refactor-f). `LibraryDescriptor` вырос ровно на толстый указатель
    /// (16 байт на x86-64) — поле `type_aliases`, добавленное под каталог
    /// типов ABI-D; записи статические, так что рост платится один раз на
    /// библиотеку, а не на объект. `MethodDescriptor` — 32 байта (написания,
    /// `arity`, обработчик), закрытие полей его не изменило. Тест ловит
    /// незамеченный рост дескриптора.
    #[test]
    fn descriptors_have_the_expected_size() {
        assert_eq!(std::mem::size_of::<LibraryDescriptor>(), 120);
        assert_eq!(std::mem::size_of::<MethodDescriptor>(), 32);
    }
}
