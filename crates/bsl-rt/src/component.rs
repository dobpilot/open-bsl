use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::rc::Rc;

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

/// Сервисы конкретного состояния исполнения, доступные компоненту.
///
/// Контекст не раскрывает стек и регистры VM. Вывод и таблицы форм
/// принадлежат сессии, поэтому две VM в одном процессе не разделяют
/// изменяемое состояние.
pub struct CallContext<'a> {
    runtime_shapes: &'a mut RuntimeShapes,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    formatter: ValueFormatter,
    function_caller: Option<&'a mut FunctionCaller<'a>>,
    /// Часовой пояс ПРОГОНА либо `None` — «этот путь о зоне не знает».
    ///
    /// Компоненту зона нужна там, где записанный момент переводится в
    /// местное время (даты JSON, лексические формы XDTO), а `HostEnv`
    /// целиком сюда не дать: он занят вызывающим.
    ///
    /// `None` приходит с НАТИВНОГО пути: у JIT-шимов контекст —
    /// сток (см. `bsl-vm/src/jit`), и зоны там нет по той же причине, что
    /// и вывода. Это не заглушка, а проверяемая запись контракта: зону
    /// читают только ГЛОБАЛЬНЫЕ функции и конструкторы компонентов
    /// (`ЗаписатьДатуJSON`, `ПрочитатьДатуJSON`, `ПрочитатьJSON`,
    /// `ЗаписатьJSON`, построение фабрики XDTO), а JIT компилирует из
    /// компонентного только `CallMethod`/`CallObjectMethod` — методы
    /// объектов, ни один из которых зону не спрашивает. Протухни этот
    /// разбор, компонент получит ошибку, а не тихо другое время.
    zone: Option<&'a Rc<dyn crate::TimeZone>>,
}

impl<'a> CallContext<'a> {
    pub fn new(
        runtime_shapes: &'a mut RuntimeShapes,
        stdout: &'a mut dyn Write,
        stderr: &'a mut dyn Write,
        formatter: ValueFormatter,
        zone: Option<&'a Rc<dyn crate::TimeZone>>,
    ) -> Self {
        Self {
            runtime_shapes,
            stdout,
            stderr,
            formatter,
            function_caller: None,
            zone,
        }
    }

    /// Создаёт контекст, в котором компонент может вызвать функцию
    /// текущего BSL-модуля по имени.
    pub fn with_function_caller(
        runtime_shapes: &'a mut RuntimeShapes,
        stdout: &'a mut dyn Write,
        stderr: &'a mut dyn Write,
        formatter: ValueFormatter,
        zone: Option<&'a Rc<dyn crate::TimeZone>>,
        function_caller: &'a mut FunctionCaller<'a>,
    ) -> Self {
        Self {
            runtime_shapes,
            stdout,
            stderr,
            formatter,
            function_caller: Some(function_caller),
            zone,
        }
    }

    pub fn runtime_shapes(&mut self) -> &mut RuntimeShapes {
        self.runtime_shapes
    }

    pub fn stdout(&mut self) -> &mut dyn Write {
        self.stdout
    }

    pub fn stderr(&mut self) -> &mut dyn Write {
        self.stderr
    }

    pub fn format_value(&self, value: &BslValue, spec: Option<&str>) -> RtResult<String> {
        (self.formatter)(value, spec)
    }

    /// Часовой пояс прогона — для перевода записанного момента в местное
    /// время. Отдаётся ссылкой, а не значением смещения: смещение зависит
    /// от МОМЕНТА, и какой момент интересует, знает только компонент.
    /// # Errors
    ///
    /// [`RtError::InvalidBytecode`], если контекст построен без зоны (см.
    /// поле `zone`): значит зону спросил путь, который её не получает, и
    /// честнее ошибка, чем чужое время.
    pub fn zone(&self) -> RtResult<&dyn crate::TimeZone> {
        self.zone.map(Rc::as_ref).ok_or(RtError::InvalidBytecode(
            "часовой пояс запрошен там, где окружения прогона нет",
        ))
    }

    /// Зона В СОБСТВЕННОСТЬ — для компонента, который её ЗАПОМИНАЕТ, а не
    /// читает по ходу вызова (фабрика XDTO хранит зону своего построения).
    ///
    /// # Errors
    ///
    /// То же, что у [`CallContext::zone`].
    pub fn zone_rc(&self) -> RtResult<Rc<dyn crate::TimeZone>> {
        self.zone.map(Rc::clone).ok_or(RtError::InvalidBytecode(
            "часовой пояс запрошен там, где окружения прогона нет",
        ))
    }

    /// Таблица форм и зона ОДНОВРЕМЕННО: `runtime_shapes` берёт `self`
    /// изменяемо, поэтому после него `zone()` уже не позвать, а нужны они
    /// вместе на каждом чтении и записи JSON.
    pub fn shapes_and_zone(&mut self) -> (&mut RuntimeShapes, Option<&dyn crate::TimeZone>) {
        (self.runtime_shapes, self.zone.map(Rc::as_ref))
    }

    /// Разделяет изменяемые сервисы для операции, которая одновременно
    /// меняет таблицу форм и может вызвать BSL-функцию.
    pub fn execution_parts(
        &mut self,
    ) -> (
        &mut RuntimeShapes,
        &mut dyn Write,
        &mut dyn Write,
        Option<&mut FunctionCaller<'a>>,
        Option<&dyn crate::TimeZone>,
    ) {
        (
            self.runtime_shapes,
            self.stdout,
            self.stderr,
            self.function_caller.as_deref_mut(),
            self.zone.map(Rc::as_ref),
        )
    }
}

/// Единый ABI статически зарегистрированной функции или конструктора.
pub type ComponentCall = for<'a> fn(&mut CallContext<'a>, &[BslValue]) -> RtResult<BslValue>;

/// Код метода внутри одного типа компонента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MethodCode(u16);

impl MethodCode {
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

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
/// Арность здесь сознательно не объявляется: у методов нет точки
/// статической проверки — получатель известен только в рантайме, — а
/// тексты ошибок о числе аргументов уже живут в самих обработчиках.
#[derive(Debug, Clone, Copy)]
pub struct MethodDescriptor {
    pub code: MethodCode,
    pub names: &'static [&'static str],
    pub call: MethodCall,
}

/// Код свойства внутри одного типа компонента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyCode(u16);

impl PropertyCode {
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    pub const fn get(self) -> u16 {
        self.0
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
    pub code: PropertyCode,
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
    // Посимвольное сравнение без аллокаций: этим путём ходят и вызовы из
    // JIT-шимов, где строка приходит на каждый вызов, — собирать `String`
    // на каждую строку таблицы недопустимо.
    let equals_ci = |candidate: &str| {
        candidate
            .chars()
            .flat_map(char::to_uppercase)
            .eq(name.chars().flat_map(char::to_uppercase))
    };
    for descriptor in table {
        if descriptor
            .names
            .iter()
            .any(|candidate| equals_ci(candidate))
        {
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
pub struct LibraryDescriptor {
    pub package: &'static str,
    pub version: &'static str,
    pub dependencies: &'static [LibraryDependency],
    pub functions: &'static [FunctionDescriptor],
    pub constructors: &'static [ConstructorDescriptor],
    /// Типы объектов, которые компонент вводит в язык. По ним `Тип("Имя")`
    /// находит тип: закрытый реестр `TypeId` ядра компонентных типов
    /// больше не знает. Не объявленный здесь тип остаётся доступен через
    /// `ТипЗнч(объект)`, но по имени не ищется.
    pub types: &'static [&'static crate::TypeDescriptor],
}

/// Дескриптор базового компонента. На переходном этапе встроенные функции
/// и конструкторы ещё обслуживаются старыми таблицами, поэтому каталоги
/// пусты; по мере миграции они будут перенесены сюда без изменения API
/// фасада.
pub const fn core_library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: crate::PACKAGE_NAME,
        version: crate::PACKAGE_VERSION,
        dependencies: &[],
        functions: &[],
        constructors: &[],
        types: &[],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    MissingCore,
    TooManyLibraries,
    EmptyIdentity,
    DuplicatePackage(String),
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
        }
    }
}

impl std::error::Error for RegistryError {}

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

        Ok(RuntimeRegistry {
            libraries: self.libraries,
            function_names,
            constructor_names,
        })
    }
}

/// Неизменяемый каталог собранного `Engine`.
pub struct RuntimeRegistry {
    libraries: Vec<LibraryDescriptor>,
    function_names: HashMap<String, (u8, FunctionCode)>,
    constructor_names: HashMap<String, (u8, ConstructorCode)>,
}

impl RuntimeRegistry {
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
            .flat_map(|library| library.types.iter().copied())
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
        LibraryDescriptor {
            package: crate::PACKAGE_NAME,
            version: crate::PACKAGE_VERSION,
            dependencies: &[],
            functions: CORE_FUNCTIONS,
            constructors: &[],
            types: &[],
        }
    }

    fn json() -> LibraryDescriptor {
        LibraryDescriptor {
            package: "bsl-json",
            version: "0.1.0",
            dependencies: &[LibraryDependency {
                package: crate::PACKAGE_NAME,
                version: crate::PACKAGE_VERSION,
            }],
            functions: &[],
            constructors: JSON_CONSTRUCTORS,
            types: &[],
        }
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
        builder.register(core()).register(LibraryDescriptor {
            package: "other",
            version: "1.0.0",
            dependencies: &[],
            functions: DUPLICATE,
            constructors: &[],
            types: &[],
        });

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
}
