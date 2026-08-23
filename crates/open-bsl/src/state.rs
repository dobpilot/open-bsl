//! Сессия исполнения: host-сервисы и запуск модулей.

use std::io::Write;

use bsl_rt::{Clock, FileSystem, HostEnv, RandomSource, TimeZone};

use crate::Value;
use crate::dynamic::DynamicCode;
use crate::engine::{Engine, Module};
use crate::error::Error;

/// Настройки сервисов одной сессии исполнения.
pub struct StateBuilder {
    engine: Engine,
    host: HostServices,
    jit: bool,
}

impl StateBuilder {
    pub(crate) fn new(engine: Engine) -> Self {
        Self {
            engine,
            host: HostServices::process(),
            jit: false,
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

    /// Файловая система сессии: `ЗначениеВФайл`, `ЗначениеИзФайла` и
    /// `Новый ДвоичныеДанные(путь)` читают и пишут через неё.
    ///
    /// Пока это ПЕРВАЯ волна возможности — файл целиком. Компонентные
    /// объекты (`ТекстовыйДокумент`, `ЧтениеJSON`, `ЧтениеZIP`, …) ходят
    /// в `std::fs` напрямую и заданную здесь систему не видят; список
    /// оставшихся мест — в обзоре `bsl_rt::FileSystem`.
    #[must_use]
    pub fn files(mut self, files: impl FileSystem + 'static) -> Self {
        self.host.env = self.host.env.with_files(files);
        self
    }

    pub fn build(self) -> State {
        State {
            dynamic: self.engine.dynamic_code(),
            engine: self.engine,
            host: self.host,
            jit: self.jit,
        }
    }
}

/// Изменяемые возможности host-приложения, принадлежащие одной сессии.
/// Они не входят в реестр компонентов и не сериализуются в байт-код.
///
/// Вывод и окружение запуска лежат вместе, потому что это одно и то же по
/// сути: то, что BSL-код берёт не из своих аргументов, а из мира вокруг.
pub struct HostServices {
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
    env: HostEnv,
}

impl HostServices {
    fn process() -> Self {
        Self {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            env: HostEnv::process(),
        }
    }
}

/// Изолированные изменяемые host-сервисы одной BSL-сессии.
pub struct State {
    engine: Engine,
    host: HostServices,
    /// Компилятор `Выполнить`/`Вычислить` этой сессии. Лежит рядом с
    /// потоками и окружением, потому что это такой же сервис прогона: VM
    /// динамический код только исполняет, а компилирует — он. Свой у
    /// каждой сессии, поэтому и кэш фрагментов у сессий раздельный.
    dynamic: DynamicCode,
    jit: bool,
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

    /// Исполняет заранее скомпилированный модуль.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания компонентов или исполнения.
    pub fn run(&mut self, module: &Module) -> Result<Value, Error> {
        let registry = self.engine.registry();
        // Компилятор фрагментов едет в VM отдельным сервисом прогона:
        // реестр и символы условной компиляции нужны ЕМУ, а не VM. Перед
        // прогоном он узнаёт, чей модуль пойдёт, — иначе нулевой чанк
        // одного модуля столкнулся бы в его кэше с нулевым чанком другого.
        self.dynamic.bind_module(module.id);
        let result = if self.jit {
            bsl_vm::run_program_jit_with_registry_and_io(
                &module.program,
                registry,
                &mut self.host.stdout,
                &mut self.host.stderr,
                &mut self.dynamic,
                &mut self.host.env,
            )
        } else {
            bsl_vm::run_program_with_registry_and_io(
                &module.program,
                registry,
                &mut self.host.stdout,
                &mut self.host.stderr,
                &mut self.dynamic,
                &mut self.host.env,
            )
        }?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
