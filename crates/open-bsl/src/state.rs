//! Сессия исполнения: host-сервисы и запуск модулей.

use std::io::Write;

use bsl_rt::{Clock, FileSystem, HostEnv, HttpClientFactory, RandomSource, TimeZone};

use crate::Value;
use crate::dynamic::DynamicCode;
use crate::engine::{Engine, Module};
use crate::error::Error;

/// Настройки сервисов одной сессии исполнения.
pub struct StateBuilder {
    engine: Engine,
    host: HostServices,
    jit: bool,
    scheduler: bsl_vm::SchedulerConfig,
    /// Host-профиль фоновых заданий этого сеанса: 0 — системный.
    #[cfg(not(target_arch = "wasm32"))]
    job_profile_index: u32,
}

impl StateBuilder {
    pub(crate) fn new(engine: Engine) -> Self {
        Self {
            engine,
            host: HostServices::process(),
            jit: false,
            scheduler: bsl_vm::SchedulerConfig::default(),
            #[cfg(not(target_arch = "wasm32"))]
            job_profile_index: 0,
        }
    }

    pub fn stdout(mut self, writer: impl Write + 'static) -> Self {
        self.host.stdout = Box::new(writer);
        self
    }

    pub fn stderr(mut self, writer: impl Write + 'static) -> Self {
        self.host.stderr = Box::new(writer);
        self
    }

    pub fn jit(mut self, enabled: bool) -> Self {
        self.jit = enabled;
        self
    }

    /// Задаёт число безопасных точек на квант кооперативной BSL-задачи.
    /// Значение применяется только когда в запуске живы несколько задач.
    #[must_use]
    pub fn safe_points_per_quantum(mut self, value: usize) -> Self {
        self.scheduler.safe_points_per_quantum = value;
        self
    }

    /// Аргументы запуска, которые скрипт увидит в
    /// `АргументыКоманднойСтроки`.
    ///
    /// Принадлежат ЭТОЙ сессии, а не процессу: два `State` одного `Engine`
    /// видят каждый свой набор, в каком угодно порядке запусков.
    #[must_use]
    pub fn arguments(mut self, arguments: Vec<String>) -> Self {
        self.host.env = self.host.env.with_arguments(arguments);
        self
    }

    /// Часы сессии: `ТекущаяДата` и
    /// `ТекущаяУниверсальнаяДатаВМиллисекундах` отвечают из них.
    /// Неподвижные часы делают вывод скрипта побайтово воспроизводимым.
    #[must_use]
    pub fn clock(mut self, clock: impl Clock + 'static) -> Self {
        self.host.env = self.host.env.with_clock(clock);
        self
    }

    /// Источник байтов для `Новый УникальныйИдентификатор()`. Расстановку
    /// битов версии и варианта он не контролирует — она остаётся за
    /// рантаймом, поэтому заданная последовательность даёт настоящий UUID
    /// версии 4, а не произвольные шестнадцать байтов.
    #[must_use]
    pub fn random(mut self, random: impl RandomSource + 'static) -> Self {
        self.host.env = self.host.env.with_random(random);
        self
    }

    /// Часовой пояс сессии: в нём толкуются даты со смещением при чтении
    /// и записи JSON и лексические формы XDTO.
    ///
    /// Он же и ОГРАНИЧЕН этим: `ТекущаяДата` считает от Unix-эпохи и
    /// смещения не применяет, а фабрика XDTO запоминает зону того прогона,
    /// в котором построена, — то есть смена зоны у сессии на уже
    /// построенную фабрику не действует.
    #[must_use]
    pub fn zone(mut self, zone: impl TimeZone + 'static) -> Self {
        self.host.env = self.host.env.with_zone(zone);
        self
    }

    /// Файловая система сессии: `ЗначениеВФайл`, `ЗначениеИзФайла`,
    /// `Новый ДвоичныеДанные(путь)` и компонентные объекты
    /// (`ТекстовыйДокумент`, `ЧтениеJSON`, `ЧтениеZIP`, …) читают и пишут
    /// через неё. Компонентам файловая возможность приходит в контексте
    /// вызова (ABI-G), а не прямым `std::fs`.
    #[must_use]
    pub fn files(mut self, files: impl FileSystem + 'static) -> Self {
        self.host.env = self.host.env.with_files(files);
        self
    }

    /// HTTP-транспорт сессии. Фабрика получает чистую конфигурацию
    /// `HTTPСоединение`; Tokio и конкретный клиент в этот интерфейс не входят.
    #[must_use]
    pub fn network(mut self, factory: impl HttpClientFactory + 'static) -> Self {
        self.host.env = self.host.env.with_network(factory);
        self
    }

    /// Явно запрещает сеть, даже если feature `http` подключил системный
    /// адаптер по умолчанию.
    #[must_use]
    pub fn deny_network(mut self) -> Self {
        self.host.env = self.host.env.without_network();
        self
    }

    /// Выбирает host-профиль фоновых заданий этого сеанса. Foreground
    /// `files`, `network`, часы и вывод сеанса в worker не копируются —
    /// профиль и есть их именованная замена, зарегистрированная в
    /// [`crate::EngineBuilder::register_host_profile`]. Задания сеанса и
    /// их потомки строят host-окружение по этому профилю; повысить его
    /// вложенное задание не может.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Configuration`] для идентификатора, не
    /// зарегистрированного в этом движке, — без fallback на
    /// process-профиль. `Engine::new_state` остаётся инфаллибельным за
    /// счёт системного профиля по умолчанию.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn host_profile(mut self, id: crate::jobs::HostProfileId) -> Result<Self, Error> {
        self.job_profile_index = self.engine.validate_host_profile(id)?;
        Ok(self)
    }

    /// Неблокирующий приёмник сообщений сеанса: `Сообщить` и
    /// `СообщениеПользователю.Сообщить()` отдают ему владеющий DTO вместо
    /// строки в stdout.
    #[must_use]
    pub fn message_sink(mut self, sink: std::rc::Rc<dyn bsl_rt::UserMessageSink>) -> Self {
        self.host.env.set_message_sink(sink);
        self
    }

    pub fn build(self) -> State {
        // На wasm32 ни хранилище, ни сервис заданий не внедряются — `mut`
        // нужен только нативным веткам ниже.
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut host = self.host;
        // Каждый сеанс получает своё временное хранилище; его mailbox
        // регистрируется в реестре движка — задания публикуют write-set'ы
        // по token'у вызывателя.
        #[cfg(not(target_arch = "wasm32"))]
        let session_token = {
            let token = crate::jobs::random_uuid();
            let session = std::rc::Rc::new(std::cell::RefCell::new(
                bsl_rt::TempStorageSession::new(token, host.env.random()),
            ));
            self.engine
                .temp_hub()
                .register(token, session.borrow().mailbox());
            host.env.set_temp_storage(session);
            token
        };
        // Движок с каталогом внедряет сервис фоновых заданий в каждый
        // сеанс: клоны движка разделяют один runtime. Ошибка сборки
        // runtime не валит сеанс — без сервиса `ФоновыеЗадания` отвечает
        // ловимой ошибкой возможности.
        #[cfg(not(target_arch = "wasm32"))]
        if self.engine.catalog().is_some()
            && let Ok(runtime) = self.engine.job_runtime()
        {
            host.env
                .set_background_jobs(std::rc::Rc::new(crate::jobs::EngineJobService {
                    runtime,
                    caller_token: session_token,
                    profile_index: self.job_profile_index,
                }));
        }
        State {
            dynamic: self.engine.dynamic_code(),
            engine: self.engine,
            host,
            jit: self.jit,
            scheduler: self.scheduler,
        }
    }
}

/// Изменяемые возможности host-приложения, принадлежащие одной сессии.
/// Они не входят в реестр компонентов и не сериализуются в байт-код.
///
/// Вывод и окружение запуска лежат вместе, потому что это одно и то же по
/// сути: то, что BSL-код берёт не из своих аргументов, а из мира вокруг.
///
/// Не часть публичной поверхности: ни одного публичного метода и ни одной
/// публичной сигнатуры, где он встречался бы, — поэтому `pub(crate)`, а не
/// `pub` (см. C.2, «замкнутость»).
pub(crate) struct HostServices {
    pub(crate) stdout: Box<dyn Write>,
    pub(crate) stderr: Box<dyn Write>,
    pub(crate) env: HostEnv,
}

impl HostServices {
    fn process() -> Self {
        let env = HostEnv::process();
        #[cfg(all(feature = "http", not(target_arch = "wasm32")))]
        let env = env.with_network(bsl_http::system_factory());
        Self {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            env,
        }
    }
}

/// Изолированные изменяемые host-сервисы одной BSL-сессии.
pub struct State {
    pub(crate) engine: Engine,
    pub(crate) host: HostServices,
    /// Компилятор `Выполнить`/`Вычислить` этой сессии. Лежит рядом с
    /// потоками и окружением, потому что это такой же сервис прогона: VM
    /// динамический код только исполняет, а компилирует — он. Свой у
    /// каждой сессии, поэтому и кэш фрагментов у сессий раздельный.
    pub(crate) dynamic: DynamicCode,
    jit: bool,
    pub(crate) scheduler: bsl_vm::SchedulerConfig,
}

/// Результат одного шага pollable-исполнения.
#[derive(Debug, PartialEq)]
pub enum ExecutionPoll {
    /// Корневой BSL-код и все порождённые им async-задачи завершились.
    Complete(Value),
    /// Есть готовая BSL-задача; host вправе вызвать [`Execution::poll`]
    /// снова сразу же.
    Runnable,
    /// Готовых BSL-задач нет; продолжение зависит от host-completion.
    Waiting,
}

/// Один запуск модуля, заимствующий изменяемые сервисы [`State`].
///
/// Объект намеренно нельзя перенести в другой `State`: обещания и
/// completion-сообщения принадлежат ровно одному запуску.
pub struct Execution<'state, 'module> {
    state: &'state mut State,
    module: &'module Module,
    vm: bsl_vm::ProgramExecution,
}

impl Execution<'_, '_> {
    /// Продвигает запуск, разрешая обработать не более `host_slice`
    /// завершений внешних операций.
    ///
    /// Нулевой слайс не делает работы и возвращает
    /// [`ExecutionPoll::Runnable`]. До появления внешних операций чистый
    /// BSL завершается за первый ненулевой вызов; это тот же драйвер, что
    /// использует [`State::run`].
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания или исполнения. Повторный `poll` после
    /// завершения также является ошибкой контракта host-приложения.
    pub fn poll(&mut self, host_slice: usize) -> Result<ExecutionPoll, Error> {
        let registry = self.state.engine.registry();
        let result = match self.state.engine.catalog() {
            Some(catalog) => self.vm.poll_configuration_with_registry_and_io(
                &self.module.program,
                catalog,
                registry,
                &mut self.state.host.stdout,
                &mut self.state.host.stderr,
                &mut self.state.dynamic,
                &mut self.state.host.env,
                host_slice,
            )?,
            None => self.vm.poll_with_registry_and_io(
                &self.module.program,
                registry,
                &mut self.state.host.stdout,
                &mut self.state.host.stderr,
                &mut self.state.dynamic,
                &mut self.state.host.env,
                host_slice,
            )?,
        };
        Ok(match result {
            bsl_vm::ProgramPoll::Complete(value, _) => ExecutionPoll::Complete(value),
            bsl_vm::ProgramPoll::Runnable => ExecutionPoll::Runnable,
            bsl_vm::ProgramPoll::Waiting => ExecutionPoll::Waiting,
        })
    }
}

impl State {
    /// Создаёт состояние с базовым рантаймом и потоками процесса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку сборки базового реестра компонентов.
    pub fn new() -> Result<Self, Error> {
        Ok(Engine::builder().build()?.new_state())
    }

    /// Компилирует и исполняет исходный модуль.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку любой фазы компиляции или исполнения.
    pub fn exec(&mut self, source: &str) -> Result<Value, Error> {
        let module = self.engine.compile(source)?;
        self.run(&module)
    }

    /// Вычисляет BSL-выражение в отдельном модуле.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку любой фазы компиляции или исполнения.
    pub fn eval(&mut self, expression: &str) -> Result<Value, Error> {
        self.exec(&format!("Возврат ({expression});"))
    }

    /// Создаёт отдельный запуск с собственным token, задачами, обещаниями
    /// и модульным состоянием.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания компонентов до первой инструкции.
    pub fn start<'state, 'module>(
        &'state mut self,
        module: &'module Module,
    ) -> Result<Execution<'state, 'module>, Error> {
        self.dynamic.bind_module(module.id);
        let jit = if self.jit {
            bsl_vm::JitMode::On
        } else {
            bsl_vm::JitMode::Off
        };
        let mut vm = bsl_vm::ProgramExecution::start_with_registry_and_scheduler(
            &module.program,
            self.engine.registry(),
            jit,
            &self.host.env,
            self.scheduler,
        )?;
        // У движка с конфигурацией каждый запуск получает свои сессионные
        // экземпляры общих модулей: `ModuleState` между запусками и
        // сеансами не разделяется.
        if let Some(catalog) = self.engine.catalog() {
            vm.attach_catalog(catalog);
            // Расширение CLI `//@используй`: тела модулей выполняются до
            // entry в post-order файлового графа. Обычная политика —
            // ленивая инициализация при первом касании.
            if let Some(order) = self.engine.eager_init_order() {
                vm.schedule_eager_init(catalog, order)?;
            }
        }
        Ok(Execution {
            state: self,
            module,
            vm,
        })
    }

    /// Исполняет заранее скомпилированный модуль.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания компонентов или исполнения.
    pub fn run(&mut self, module: &Module) -> Result<Value, Error> {
        let mut execution = self.start(module)?;
        loop {
            match execution.poll(usize::MAX)? {
                ExecutionPoll::Complete(value) => return Ok(value),
                ExecutionPoll::Runnable | ExecutionPoll::Waiting => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pollable_execution_matches_run_for_pure_bsl() {
        let engine = Engine::builder().build().expect("сборка движка");
        let module = engine
            .compile(
                "Асинх Функция Ф() Возврат 42; КонецФункции\n\
                 Асинх Процедура П() Если Ждать Ф() <> 42 Тогда ВызватьИсключение; КонецЕсли; КонецПроцедуры\n\
                 П(); Возврат 7;",
            )
            .expect("компиляция");
        let mut state = engine.new_state();
        let mut execution = state.start(&module).unwrap();

        assert_eq!(execution.poll(0).unwrap(), ExecutionPoll::Runnable);
        assert_eq!(
            execution.poll(1).unwrap(),
            ExecutionPoll::Complete(Value::Number(bsl_rt::BslNumber::from_i64(7)))
        );
        assert!(execution.poll(1).is_err());

        let mut state = engine.new_state();
        assert_eq!(
            state.run(&module).unwrap(),
            Value::Number(bsl_rt::BslNumber::from_i64(7))
        );
    }

    #[test]
    fn standalone_metadata_has_no_configuration_common_modules() {
        let engine = Engine::builder().build().expect("сборка движка");
        let module = engine
            .compile(
                "Возврат Метаданные.ОбщиеМодули.Найти(\"ПолучениеФайловИзИнтернета\") = Неопределено;",
            )
            .expect("компиляция");
        for jit in [false, true] {
            assert_eq!(
                engine
                    .state_builder()
                    .jit(jit)
                    .build()
                    .run(&module)
                    .unwrap(),
                Value::Boolean(true)
            );
        }
    }

    /// Кэш фрагментов переживает запуск, поэтому обязан быть УСТОЙЧИВЫМ:
    /// повторный `run` того же модуля не компилирует ничего заново и не
    /// добавляет записей. Иначе долгоживущая сессия копила бы недостижимые
    /// чанки на каждый запуск.
    #[test]
    fn repeated_runs_of_one_module_reuse_the_cached_fragments() {
        let engine = Engine::builder().build().expect("сборка движка");
        let module = engine
            .compile(
                "сумма = 0;\n\
                 Для ном = 1 По 3 Цикл\n\
                 Выполнить(\"сумма = сумма + ном\");\n\
                 КонецЦикла;\n\
                 Возврат сумма + Вычислить(\"1\");",
            )
            .expect("компиляция модуля");
        let mut state = engine.new_state();

        assert_eq!(state.run(&module).unwrap().to_string(), "7");
        let after_first = (state.dynamic.cached(), state.dynamic.compiles());
        // Два места `Выполнить`/`Вычислить` — две записи, и цикл из трёх
        // итераций их не размножил.
        assert_eq!(after_first, (2, 2));

        for _ in 0..5 {
            assert_eq!(state.run(&module).unwrap().to_string(), "7");
        }
        assert_eq!(
            (state.dynamic.cached(), state.dynamic.compiles()),
            after_first,
            "повторные запуски не должны ни компилировать, ни копить"
        );
    }

    /// То же для ВЛОЖЕННОГО динамического кода: фрагмент внутри фрагмента
    /// получает область по номеру внешнего фрагмента, а тот приезжает из
    /// кэша — значит и вложенный на повторе в кэш попадает.
    #[test]
    fn nested_dynamic_code_is_not_recompiled_on_a_repeat_run() {
        let engine = Engine::builder().build().expect("сборка движка");
        let module = engine
            .compile("х = 1;\nВыполнить(\"Выполнить(\"\"х = х + 41\"\")\");\nВозврат х;")
            .expect("компиляция модуля");
        let mut state = engine.new_state();

        assert_eq!(state.run(&module).unwrap().to_string(), "42");
        // Внешний фрагмент и вложенный — две компиляции, не больше.
        assert_eq!(state.dynamic.compiles(), 2);
        assert_eq!(state.dynamic.cached(), 2);

        assert_eq!(state.run(&module).unwrap().to_string(), "42");
        assert_eq!(
            (state.dynamic.cached(), state.dynamic.compiles()),
            (2, 2),
            "вложенный фрагмент обязан попадать в кэш на повторном запуске"
        );
    }

    /// Кэш принадлежит СЕССИИ: у второй `State` того же движка он свой.
    #[test]
    fn two_states_do_not_share_the_fragment_cache() {
        let engine = Engine::builder().build().expect("сборка движка");
        let module = engine
            .compile("Возврат Вычислить(\"2 + 2\");")
            .expect("компиляция модуля");
        let mut first = engine.new_state();
        let mut second = engine.new_state();

        assert_eq!(first.run(&module).unwrap().to_string(), "4");
        assert_eq!(first.dynamic.compiles(), 1);
        assert_eq!(second.dynamic.compiles(), 0);

        assert_eq!(second.run(&module).unwrap().to_string(), "4");
        assert_eq!(second.dynamic.compiles(), 1);
    }

    /// Два РАЗНЫХ модуля в одной сессии не делят чанк: нулевой чанк есть у
    /// каждого, а таблицы локальных у них свои.
    #[test]
    fn two_modules_in_one_state_get_their_own_fragments() {
        let engine = Engine::builder().build().expect("сборка движка");
        let first = engine
            .compile("х = 40;\nВозврат Вычислить(\"х + 2\");")
            .expect("первый модуль");
        let second = engine
            .compile("у = 1;\nх = 5;\nВозврат Вычислить(\"х + 2\");")
            .expect("второй модуль");
        let mut state = engine.new_state();

        assert_eq!(state.run(&first).unwrap().to_string(), "42");
        assert_eq!(state.run(&second).unwrap().to_string(), "7");
        assert_eq!(state.dynamic.compiles(), 2);
    }
}
