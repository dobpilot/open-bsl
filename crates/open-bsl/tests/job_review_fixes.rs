//! Регрессионные проверки исправлений по ревью плана фоновых заданий:
//! host-профили без повышения возможностей, реальные лимиты
//! `BackgroundJobConfig`, вложенное временное хранилище, полная модель
//! сообщений, supervision worker и типизированные host-ошибки.
#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use open_bsl::jobs::{BackgroundJobConfig, BackgroundStateFactory, JobRuntime};
use open_bsl::{Engine, HostError, HostErrorCode, JobStateDto, StateBuilder, UserMessageDto};

const WAIT: Option<Duration> = Some(Duration::from_secs(30));

/// Общий буфер stdout сеанса — как в `embedding.rs`.
#[derive(Default, Clone)]
struct SharedWriter(Rc<RefCell<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SharedWriter {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).unwrap()
    }
}

/// Тексты сообщений задания — самый частый разбор в проверках.
fn message_texts(runtime: &JobRuntime, id: open_bsl::JobId) -> Vec<String> {
    runtime
        .take_messages(id, false)
        .expect("сообщения читаются")
        .into_iter()
        .map(|message| message.text)
        .collect()
}

/// Запускает цель без параметров и дожидается terminal-состояния.
fn run_job(runtime: &JobRuntime, target: &str) -> Arc<open_bsl::JobSnapshotDto> {
    let params = empty_params();
    let snapshot = runtime
        .submit_by_name(target, params, None, None)
        .expect("задание принято");
    assert!(
        runtime
            .wait_terminal(&[snapshot.id], WAIT)
            .expect("ожидание без ошибок")
    );
    runtime
        .snapshot(snapshot.id)
        .expect("снимок после terminal")
}

fn empty_params() -> Arc<open_bsl::SerializedValueGraph> {
    let shapes = open_bsl::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    Arc::new(
        open_bsl::SerializedValueGraph::capture(&[], &shapes, &open_bsl::GraphLimits::default())
            .expect("пустой снимок"),
    )
}

// --- 1. Host-профили ----------------------------------------------------

/// Профиль без сети: `deny_network` сеанса задания.
struct DenyNetworkProfile;

impl BackgroundStateFactory for DenyNetworkProfile {
    fn configure(&self, builder: StateBuilder) -> Result<StateBuilder, String> {
        Ok(builder.deny_network())
    }
}

const NETWORK_MODULE: &str = "Процедура ПроверитьСеть() Экспорт\n\
     Попытка\n\
         Соединение = Новый HTTPСоединение(\"127.0.0.1\", 1);\n\
         Ответ = Соединение.Получить(Новый HTTPЗапрос(\"/нет\"));\n\
         Сообщить(\"сеть доступна\");\n\
     Исключение\n\
         Сообщить(\"сеть: \" + ИнформацияОбОшибке().Описание);\n\
     КонецПопытки;\n\
 КонецПроцедуры\n\
 Процедура РодительСети() Экспорт\n\
     Задание = ФоновыеЗадания.Выполнить(\"Служебный.ПроверитьСеть\");\n\
     Задание.ОжидатьЗавершенияВыполнения();\n\
     Сообщения = Задание.ПолучитьСообщенияПользователю();\n\
     Для Каждого Сообщение Из Сообщения Цикл\n\
         Сообщить(\"ребёнок: \" + Сообщение.Текст);\n\
     КонецЦикла;\n\
 КонецПроцедуры";

/// Профиль `deny_network` действует в задании: сетевой вызов отвечает
/// ошибкой возможности, а не соединением; системный профиль сети не
/// лишён (его отказ — сетевой, без слова «возможность»).
#[cfg(feature = "http")]
#[test]
fn a_deny_network_profile_removes_network_from_the_job() {
    let mut builder = Engine::builder().common_module("Служебный", NETWORK_MODULE);
    let profile = builder.register_host_profile(Arc::new(DenyNetworkProfile));
    let engine = builder.build().expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");

    // Системный профиль: сеть есть, отказ — сетевой (порт 1 закрыт).
    let system = run_job(&runtime, "Служебный.ПроверитьСеть");
    assert_eq!(system.state, JobStateDto::Completed);
    let system_text = message_texts(&runtime, system.id).join("\n");
    assert!(
        !system_text.contains("возможность"),
        "системный профиль не лишён сети: {system_text}"
    );

    // Профиль deny_network: задание из ЭТОГО сеанса сети не видит.
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль зарегистрирован в этом движке")
        .build();
    let out = SharedWriter::default();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.ПроверитьСеть\");\n\
             Задание.ОжидатьЗавершенияВыполнения();",
        )
        .expect("entry компилируется");
    drop(out);
    state.run(&module).expect("прогон");
    let jobs = runtime.snapshots();
    let denied = jobs
        .iter()
        .filter(|snapshot| {
            message_texts(&runtime, snapshot.id)
                .join("\n")
                .contains("возможность")
        })
        .count();
    assert_eq!(denied, 1, "ровно одно задание осталось без сети");
}

/// Вложенное задание наследует профиль родителя: ребёнок родительского
/// задания под `deny_network` тоже не видит сети — повышения нет.
#[cfg(feature = "http")]
#[test]
fn a_nested_job_inherits_the_deny_network_profile() {
    let mut builder = Engine::builder().common_module("Служебный", NETWORK_MODULE);
    let profile = builder.register_host_profile(Arc::new(DenyNetworkProfile));
    let engine = builder.build().expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль валиден")
        .build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.РодительСети\");\n\
             Задание.ОжидатьЗавершенияВыполнения();",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let parent = runtime
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.method_name == "Служебный.РодительСети")
        .expect("родительское задание в реестре");
    let text = message_texts(&runtime, parent.id).join("\n");
    assert!(
        text.contains("ребёнок: сеть: ") && text.contains("возможность"),
        "ребёнок обязан унаследовать deny_network: {text}"
    );
}

/// Изолированная файловая система профиля: запись задания попадает в
/// память фабрики, а не на диск процесса.
#[derive(Debug, Clone, Default)]
struct SharedStore(Arc<Mutex<HashMap<String, Vec<u8>>>>);

#[derive(Debug)]
struct MemoryFs(SharedStore);

impl open_bsl::FileSystem for MemoryFs {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        self.0.0.lock().unwrap().get(path).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("нет файла {path}"))
        })
    }

    fn write(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        self.0
            .0
            .lock()
            .unwrap()
            .insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn metadata(&self, path: &str) -> std::io::Result<open_bsl::FileMetadata> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("метаданные не поддержаны: {path}"),
        ))
    }

    fn read_dir<'fs>(
        &'fs self,
        path: &str,
    ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<open_bsl::DirEntry>> + 'fs>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("каталог не поддержан: {path}"),
        ))
    }

    fn create_dir_all(&self, _path: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn open(
        &self,
        path: &str,
        _options: open_bsl::FileOpenOptions,
    ) -> std::io::Result<Box<dyn open_bsl::FileHandle>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("дескрипторы не поддержаны: {path}"),
        ))
    }
}

struct MemoryFsProfile(SharedStore);

impl BackgroundStateFactory for MemoryFsProfile {
    fn configure(&self, builder: StateBuilder) -> Result<StateBuilder, String> {
        Ok(builder.files(MemoryFs(self.0.clone())))
    }
}

#[test]
fn an_isolated_file_system_profile_confines_job_writes() {
    let store = SharedStore::default();
    let mut builder = Engine::builder().common_module(
        "Служебный",
        "Процедура Записать() Экспорт\n\
             ЗначениеВФайл(\"/изолировано/значение\", 42);\n\
         КонецПроцедуры",
    );
    let profile = builder.register_host_profile(Arc::new(MemoryFsProfile(store.clone())));
    let engine = builder.build().expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль валиден")
        .build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Записать\");\n\
             Задание.ОжидатьЗавершенияВыполнения();",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let done = runtime
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.method_name == "Служебный.Записать")
        .expect("задание в реестре");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );
    assert!(
        store
            .0
            .lock()
            .unwrap()
            .contains_key("/изолировано/значение"),
        "запись обязана попасть в изолированную ФС профиля"
    );
}

/// Пользовательская библиотека родительского движка доступна цели
/// задания: worker регистрирует те же `LibraryDescriptor`.
fn host_answer(
    _context: &mut open_bsl::CallContext<'_>,
    _arguments: &[open_bsl::Value],
) -> open_bsl::RtResult<open_bsl::Value> {
    Ok(open_bsl::Value::Boolean(true))
}

const HOST_FUNCTIONS: &[open_bsl::FunctionDescriptor] = &[open_bsl::FunctionDescriptor {
    code: open_bsl::FunctionCode::new(1),
    names: &["ОтветХоста", "HostAnswer"],
    arity: open_bsl::Arity::exact(0),
    kind: open_bsl::FunctionKind::Function,
    call: host_answer,
}];

fn host_library() -> open_bsl::LibraryDescriptor {
    open_bsl::LibraryDescriptor::new(
        "example-host",
        "1.0.0",
        open_bsl::ObjectContextNeed::Reduced,
    )
    .with_dependencies(&[open_bsl::LibraryDependency {
        package: bsl_rt::PACKAGE_NAME,
        version: bsl_rt::PACKAGE_VERSION,
    }])
    .with_functions(HOST_FUNCTIONS)
}

#[test]
fn a_job_target_calls_a_user_registered_library() {
    let engine = Engine::builder()
        .register_library(host_library())
        .common_module(
            "Служебный",
            "Процедура Спросить() Экспорт\n\
                 Сообщить(ОтветХоста());\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Спросить");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );
    assert_eq!(message_texts(&runtime, done.id), ["Да"]);
}

/// Идентификатор чужого движка отвергается при выборе профиля — без
/// fallback на process-профиль.
#[test]
fn a_foreign_host_profile_id_is_rejected() {
    let mut builder_a = Engine::builder().common_module("М", "Перем Х Экспорт;");
    let foreign = builder_a.register_host_profile(Arc::new(DenyNetworkProfile));
    let _engine_a = builder_a.build().expect("движок A");
    let engine_b = Engine::builder()
        .common_module("М", "Перем Х Экспорт;")
        .build()
        .expect("движок B");
    let error = match engine_b.state_builder().host_profile(foreign) {
        Ok(_) => panic!("чужой профиль обязан быть отвергнут"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("не зарегистрирован"),
        "не тот текст: {error}"
    );
}

/// Ошибка фабрики без паники завершает ТОЛЬКО этот job как `Failed` с
/// текстом host-профиля; worker остаётся жив и доводит следующее задание.
struct FailingProfile;

impl BackgroundStateFactory for FailingProfile {
    fn configure(&self, _builder: StateBuilder) -> Result<StateBuilder, String> {
        Err("нет прав на песочницу".to_string())
    }
}

#[test]
fn a_factory_error_fails_only_its_job() {
    let mut builder = Engine::builder().common_module(
        "Служебный",
        "Процедура Пусто() Экспорт\n\
         КонецПроцедуры",
    );
    let profile = builder.register_host_profile(Arc::new(FailingProfile));
    let engine = builder.build().expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль валиден")
        .build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Задание.ОжидатьЗавершенияВыполнения();",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let failed = runtime
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.method_name == "Служебный.Пусто")
        .expect("задание в реестре");
    assert_eq!(failed.state, JobStateDto::Failed);
    let error = failed.error.as_ref().expect("ошибка задания");
    assert!(
        error.brief.contains("host-профиль недоступен")
            && error.brief.contains("нет прав на песочницу"),
        "не тот текст: {}",
        error.brief
    );
    // Worker жив: системное задание после отказа фабрики завершается.
    let done = run_job(&runtime, "Служебный.Пусто");
    assert_eq!(done.state, JobStateDto::Completed);
}

// --- 5. Supervision worker ---------------------------------------------

/// Паника фабрики — внешняя паника worker (вне границы задания).
struct PanickingProfile;

impl BackgroundStateFactory for PanickingProfile {
    fn configure(&self, _builder: StateBuilder) -> Result<StateBuilder, String> {
        panic!("фабрика профиля упала");
    }
}

/// Внешняя паника worker переводит закреплённое задание в `Failed` (не
/// вечное `Running`), worker заменяется и доводит следующее задание.
#[test]
fn a_worker_panic_fails_the_job_and_the_worker_is_replaced() {
    let mut builder = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            workers: Some(1),
            ..BackgroundJobConfig::default()
        });
    let profile = builder.register_host_profile(Arc::new(PanickingProfile));
    let engine = builder.build().expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль валиден")
        .build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Задание.ОжидатьЗавершенияВыполнения();",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let failed = runtime
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.state == JobStateDto::Failed)
        .expect("упавшее задание в реестре");
    assert!(
        failed
            .error
            .as_ref()
            .expect("ошибка задания")
            .brief
            .contains("сбоем worker"),
        "паника worker обязана завершить задание Failed"
    );
    // Замена worker: системное задание доводится новым потоком.
    let done = run_job(&runtime, "Служебный.Пусто");
    assert_eq!(done.state, JobStateDto::Completed);
}

/// Три ПОСЛЕДОВАТЕЛЬНЫЕ паники запуска переводят runtime в `Broken`:
/// новые submissions получают ловимую ошибку с кодом `RuntimeBroken`.
#[test]
fn three_consecutive_startup_panics_break_the_runtime() {
    let mut builder = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            workers: Some(1),
            ..BackgroundJobConfig::default()
        });
    let profile = builder.register_host_profile(Arc::new(PanickingProfile));
    let engine = builder.build().expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль валиден")
        .build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Попытка\n\
                 Задание.ОжидатьЗавершенияВыполнения();\n\
             Исключение\n\
             КонецПопытки;",
        )
        .expect("entry компилируется");
    for _ in 0..3 {
        state.run(&module).expect("прогон");
    }
    // После третьей паники runtime сломан; каждое из трёх заданий при
    // этом получило terminal-состояние, а не вечное Running. Ожидание
    // просыпается от terminal-события Drop-гарда РАНЬШЕ, чем супервизор
    // досчитает паники и выставит Broken, поэтому зазор пережидается
    // повторными submissions: принятые в него задания сломанный runtime
    // сам доведёт до Failed.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let error = loop {
        match runtime.submit_by_name("Служебный.Пусто", empty_params(), None, None) {
            Err(error) => break error,
            Ok(extra) => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "runtime так и не стал Broken"
                );
                let _ = runtime.wait_terminal(&[extra.id], Some(Duration::from_secs(10)));
            }
        }
    };
    assert_eq!(error.code, HostErrorCode::RuntimeBroken);
    for snapshot in runtime.snapshots() {
        assert!(
            snapshot.state.is_terminal(),
            "задание {} застряло в {:?}",
            snapshot.id.to_uuid_string(),
            snapshot.state
        );
    }
}

// --- 2. Лимиты ----------------------------------------------------------

/// Payload больше предела одной записи отвергается ловимой ошибкой ДО
/// запуска, а сериализатор останавливается на том же пределе.
#[test]
fn an_oversized_payload_is_rejected_catchably() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Эхо(Знач х) Экспорт\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            max_single_job_record_bytes: 64 << 10,
            max_error_bytes_per_job: 8 << 10,
            max_message_bytes_per_job: 8 << 10,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "Параметры = Новый Массив;\n\
             Строка1 = \"х\";\n\
             Для Н = 1 По 17 Цикл\n\
                 Строка1 = Строка1 + Строка1;\n\
             КонецЦикла;\n\
             Параметры.Добавить(Строка1);\n\
             Попытка\n\
                 Задание = ФоновыеЗадания.Выполнить(\"Служебный.Эхо\", Параметры);\n\
                 Сообщить(\"принято\");\n\
             Исключение\n\
                 Сообщить(\"отказ: \" + ИнформацияОбОшибке().Описание);\n\
             КонецПопытки;",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let text = out.text();
    assert!(
        text.starts_with("отказ: "),
        "крупный payload обязан быть отвергнут: {text}"
    );
}

/// Явный предел одновременных заданий и освобождение слота на terminal.
#[test]
fn the_inflight_limit_is_enforced_and_released() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры\n\
             Процедура Вечно() Экспорт\n\
                 Пока Истина Цикл\n\
                 КонецЦикла;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            workers: Some(1),
            max_inflight_jobs: 1,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let blocker = runtime
        .submit_by_name("Служебный.Вечно", empty_params(), None, None)
        .expect("первое задание принято");
    let error = runtime
        .submit_by_name("Служебный.Пусто", empty_params(), None, None)
        .expect_err("предел одновременных заданий");
    assert_eq!(error.code, HostErrorCode::ResourceLimit);
    runtime.cancel(blocker.id).expect("отмена");
    assert!(
        runtime
            .wait_terminal(&[blocker.id], WAIT)
            .expect("ожидание")
    );
    // Слот освобождён terminal transition.
    let done = run_job(&runtime, "Служебный.Пусто");
    assert_eq!(done.state, JobStateDto::Completed);
}

/// Бюджет ошибки: огромная диагностика усечена до
/// `max_error_bytes_per_job` с признаком `DiagnosticResourceLimit`.
#[test]
fn a_huge_error_is_bounded_by_the_error_budget() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Упасть() Экспорт\n\
                 Текст = \"ошибка!\";\n\
                 Для Н = 1 По 12 Цикл\n\
                     Текст = Текст + Текст;\n\
                 КонецЦикла;\n\
                 ВызватьИсключение Текст;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            max_error_bytes_per_job: 2 << 10,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Упасть");
    assert_eq!(done.state, JobStateDto::Failed);
    let error = done.error.as_ref().expect("ошибка задания");
    assert!(
        error.diagnostic_truncated,
        "диагностика обязана быть усечена"
    );
    assert!(
        error.byte_size() <= 2 << 10,
        "бюджет ошибки нарушен: {}",
        error.byte_size()
    );
    assert!(
        error.brief.contains("DiagnosticResourceLimit"),
        "нет признака усечения: {}",
        error.brief
    );
}

/// Бюджет сообщений: превышение — ловимая ошибка в задании, уже принятые
/// сообщения сохраняются.
#[test]
fn the_message_budget_is_enforced_catchably() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Болтать() Экспорт\n\
                 Сообщить(\"первое\");\n\
                 Большое = \"м\";\n\
                 Для Н = 1 По 10 Цикл\n\
                     Большое = Большое + Большое;\n\
                 КонецЦикла;\n\
                 Попытка\n\
                     Сообщить(Большое);\n\
                     Сообщить(\"прошло\");\n\
                 Исключение\n\
                     Сообщить(\"отказ\");\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            // 1 КиБ: «первое» и «отказ» помещаются, килобайтная строка — нет.
            max_message_bytes_per_job: 1 << 10,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Болтать");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );
    assert_eq!(message_texts(&runtime, done.id), ["первое", "отказ"]);
}

/// Бюджет сообщений ограничивает сериализацию ГРАФА в `КлючДанных` ДО
/// крупной аллокации: за большими строками в массиве лежит вовсе не
/// сериализуемый компонентный объект, и ограниченный бюджетом обходчик
/// до него не доходит — отказ приходит текстом бюджета, а не ошибкой
/// непереносимого типа, как было бы при сериализации «сначала целиком».
#[test]
fn a_huge_graph_in_a_message_field_is_refused_by_the_budget() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура ГрафВКлюче() Экспорт\n\
                 Сообщить(\"первое\");\n\
                 Строка1 = \"м\";\n\
                 Для Н = 1 По 11 Цикл\n\
                     Строка1 = Строка1 + Строка1;\n\
                 КонецЦикла;\n\
                 Большое = Новый Массив;\n\
                 Большое.Добавить(Строка1);\n\
                 Большое.Добавить(Новый ЗаписьJSON);\n\
                 Сообщение = Новый СообщениеПользователю;\n\
                 Сообщение.Текст = \"короткий\";\n\
                 Сообщение.КлючДанных = Большое;\n\
                 Попытка\n\
                     Сообщение.Сообщить();\n\
                     Сообщить(\"прошло\");\n\
                 Исключение\n\
                     Сообщить(\"отказ: \" + ИнформацияОбОшибке().Описание);\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            // 1 КиБ: тексты проб помещаются, двухкилобайтная строка графа — нет.
            max_message_bytes_per_job: 1 << 10,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.ГрафВКлюче");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );
    let texts = message_texts(&runtime, done.id);
    assert_eq!(texts.first().map(String::as_str), Some("первое"));
    let refusal = texts.get(1).expect("отказ обязан быть сообщён");
    assert!(
        refusal.contains("бюджет"),
        "отказ обязан прийти от бюджета сообщений, а не от типа: {refusal}"
    );
    assert!(
        !texts.iter().any(|text| text == "прошло"),
        "сообщение с графом больше бюджета не имеет права пройти"
    );
}

/// Per-job staging-кредит временного хранилища: превышение — ловимая
/// ошибка, уже собранный write-set цел и публикуется на terminal.
#[test]
fn the_per_job_staging_budget_is_enforced() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Писать(Знач Адрес) Экспорт\n\
                 ПоместитьВоВременноеХранилище(\"малое\", Адрес);\n\
                 Большое = \"х\";\n\
                 Для Н = 1 По 12 Цикл\n\
                     Большое = Большое + Большое;\n\
                 КонецЦикла;\n\
                 Попытка\n\
                     ПоместитьВоВременноеХранилище(Большое, Адрес);\n\
                     Сообщить(\"большое прошло\");\n\
                 Исключение\n\
                     Сообщить(\"staging: \" + ИнформацияОбОшибке().Описание);\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            max_staged_temp_bytes_per_job: 1 << 10,
            max_live_staged_temp_bytes: 1 << 10,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "Адрес = ПоместитьВоВременноеХранилище(\"старое\");\n\
             Параметры = Новый Массив;\n\
             Параметры.Добавить(Адрес);\n\
             Задание = ФоновыеЗадания.Выполнить(\"Служебный.Писать\", Параметры);\n\
             Задание.ОжидатьЗавершенияВыполнения();\n\
             Сообщить(\"после: \" + ПолучитьИзВременногоХранилища(Адрес));",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let runtime = engine.job_runtime().expect("runtime");
    let job = runtime
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.method_name == "Служебный.Писать")
        .expect("задание в реестре");
    let text = message_texts(&runtime, job.id).join("\n");
    assert!(
        text.contains("staging: "),
        "лимит staging обязан сработать: {text}"
    );
    assert!(
        out.text().contains("после: малое"),
        "малый write-set обязан пережить отказ и опубликоваться: {}",
        out.text()
    );
}

/// Глобальный staging-кредит освобождается на terminal: последовательные
/// задания с полным кредитом каждый раз получают его заново.
#[test]
fn the_global_staging_budget_is_released_on_terminal() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Писать(Знач Адрес) Экспорт\n\
                 Большое = \"х\";\n\
                 Для Н = 1 По 8 Цикл\n\
                     Большое = Большое + Большое;\n\
                 КонецЦикла;\n\
                 ПоместитьВоВременноеХранилище(Большое, Адрес);\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            // 512 байт полезной строки + служебные байты графа — впритык
            // к 1 КиБ: два незакрытых кредита не поместились бы.
            max_staged_temp_bytes_per_job: 1 << 10,
            max_live_staged_temp_bytes: 1 << 10,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "Адрес = ПоместитьВоВременноеХранилище(\"старое\");\n\
             Параметры = Новый Массив;\n\
             Параметры.Добавить(Адрес);\n\
             Для Н = 1 По 3 Цикл\n\
                 Задание = ФоновыеЗадания.Выполнить(\"Служебный.Писать\", Параметры);\n\
                 Задание.ОжидатьЗавершенияВыполнения();\n\
             КонецЦикла;",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let runtime = engine.job_runtime().expect("runtime");
    for snapshot in runtime.snapshots() {
        assert_eq!(
            snapshot.state,
            JobStateDto::Completed,
            "кредит обязан освобождаться на terminal: {:?}",
            snapshot.error
        );
    }
}

// --- 3. Вложенное временное хранилище -----------------------------------

/// Дочернее задание публикует write-set в mailbox НЕПОСРЕДСТВЕННОГО
/// родителя (job-сеанса), а адрес исходного foreground-сеанса для него
/// чужой — ловимая ошибка, транзитивного повышения нет.
#[test]
fn a_child_job_publishes_to_its_immediate_parent_only() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Родитель(Знач АдресДеда) Экспорт\n\
                 МойАдрес = ПоместитьВоВременноеХранилище(\"родительское\");\n\
                 Параметры = Новый Массив;\n\
                 Параметры.Добавить(МойАдрес);\n\
                 Параметры.Добавить(АдресДеда);\n\
                 Задание = ФоновыеЗадания.Выполнить(\"Служебный.Ребёнок\", Параметры);\n\
                 Задание.ОжидатьЗавершенияВыполнения();\n\
                 Сообщить(\"родитель видит: \" + ПолучитьИзВременногоХранилища(МойАдрес));\n\
                 Сообщения = Задание.ПолучитьСообщенияПользователю();\n\
                 Для Каждого Сообщение Из Сообщения Цикл\n\
                     Сообщить(\"ребёнок: \" + Сообщение.Текст);\n\
                 КонецЦикла;\n\
             КонецПроцедуры\n\
             Процедура Ребёнок(Знач АдресРодителя, Знач АдресДеда) Экспорт\n\
                 ПоместитьВоВременноеХранилище(\"от ребёнка\", АдресРодителя);\n\
                 Попытка\n\
                     ПоместитьВоВременноеХранилище(\"взлом\", АдресДеда);\n\
                     Сообщить(\"дед доступен\");\n\
                 Исключение\n\
                     Сообщить(\"дед: \" + ИнформацияОбОшибке().Описание);\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "АдресДеда = ПоместитьВоВременноеХранилище(\"дедово\");\n\
             Параметры = Новый Массив;\n\
             Параметры.Добавить(АдресДеда);\n\
             Задание = ФоновыеЗадания.Выполнить(\"Служебный.Родитель\", Параметры);\n\
             Задание.ОжидатьЗавершенияВыполнения();\n\
             Сообщить(\"дед видит: \" + ПолучитьИзВременногоХранилища(АдресДеда));",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let parent = runtime
        .snapshots()
        .into_iter()
        .find(|snapshot| snapshot.method_name == "Служебный.Родитель")
        .expect("родительское задание");
    assert_eq!(
        parent.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        parent.error
    );
    let text = message_texts(&runtime, parent.id).join("\n");
    assert!(
        text.contains("родитель видит: от ребёнка"),
        "публикация ребёнка обязана дойти до mailbox родителя: {text}"
    );
    assert!(
        text.contains("ребёнок: дед: ") && !text.contains("дед доступен"),
        "адрес foreground-сеанса обязан быть чужим для внука: {text}"
    );
    assert!(
        out.text().contains("дед видит: дедово"),
        "значение деда обязано остаться нетронутым: {}",
        out.text()
    );
}

/// Закрытие сеанса-получателя до публикации: успешный BSL-job становится
/// `Failed` — частичной публикации нет.
#[test]
fn a_closed_caller_session_fails_the_publishing_job() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Вечно() Экспорт\n\
                 Пока Истина Цикл\n\
                 КонецЦикла;\n\
             КонецПроцедуры\n\
             Процедура Писать(Знач Адрес) Экспорт\n\
                 ПоместитьВоВременноеХранилище(\"позднее\", Адрес);\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            workers: Some(1),
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    // Блокирующее задание занимает единственный worker, писатель ждёт в
    // FIFO; сеанс-получатель закрывается до его старта.
    let blocker = runtime
        .submit_by_name("Служебный.Вечно", empty_params(), None, None)
        .expect("блокирующее задание принято");
    let writer = {
        let mut state = engine.new_state();
        let module = engine
            .compile_entry(
                "Адрес = ПоместитьВоВременноеХранилище(\"старое\");\n\
                 Параметры = Новый Массив;\n\
                 Параметры.Добавить(Адрес);\n\
                 Задание = ФоновыеЗадания.Выполнить(\"Служебный.Писать\", Параметры);\n\
                 Возврат Задание.УникальныйИдентификатор;",
            )
            .expect("entry компилируется");
        let value = state.run(&module).expect("прогон");
        let text = open_bsl::format_value(&value, None).expect("идентификатор");
        open_bsl::JobId(bsl_rt::uuid::parse(&text).expect("uuid"))
        // Сеанс-получатель закрывается здесь.
    };
    runtime.cancel(blocker.id).expect("отмена блокирующего");
    assert!(runtime.wait_terminal(&[writer], WAIT).expect("ожидание"));
    let done = runtime.snapshot(writer).expect("снимок");
    assert_eq!(
        done.state,
        JobStateDto::Failed,
        "публикация в закрытый сеанс обязана завершить задание Failed"
    );
    assert!(
        done.error
            .as_ref()
            .expect("ошибка задания")
            .brief
            .contains("закрыт"),
        "не тот текст: {:?}",
        done.error
    );
}

// --- 4. Сообщения --------------------------------------------------------

/// Все поля `СообщениеПользователю` переживают границу сеансов, а
/// `ПолучитьСообщенияПользователю(Истина)` атомарно забирает сообщения у
/// live и у terminal задания; повторное чтение пусто.
#[test]
fn message_fields_survive_and_drain_is_atomic() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Сказать() Экспорт\n\
                 Сообщение = Новый СообщениеПользователю;\n\
                 Сообщение.Текст = \"текст\";\n\
                 Сообщение.Поле = \"Объект.Реквизит\";\n\
                 Сообщение.ПутьКДанным = \"Данные\";\n\
                 Сообщение.КлючДанных = 42;\n\
                 Сообщение.ИдентификаторНазначения = \"форма\";\n\
                 Сообщение.Сообщить();\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Сказать");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );

    // Rust-уровень: DTO несёт все поля.
    let messages = runtime.take_messages(done.id, false).expect("сообщения");
    assert_eq!(messages.len(), 1);
    let dto = &messages[0];
    assert_eq!(dto.text, "текст");
    assert_eq!(dto.field, "Объект.Реквизит");
    assert_eq!(dto.data_path, "Данные");
    assert!(dto.data_key.is_some(), "КлючДанных обязан сериализоваться");
    assert!(
        dto.target_id.is_some(),
        "ИдентификаторНазначения обязан сериализоваться"
    );

    // BSL-уровень: свойства читаются у terminal-задания, drain атомарен.
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(&format!(
            "Задание = ФоновыеЗадания.НайтиПоУникальномуИдентификатору(\n\
                 Новый УникальныйИдентификатор(\"{}\"));\n\
             Сообщения = Задание.ПолучитьСообщенияПользователю(Истина);\n\
             С = Сообщения[0];\n\
             Сообщить(С.Текст + \"|\" + С.Поле + \"|\" + С.ПутьКДанным + \"|\"\n\
                 + Строка(С.КлючДанных) + \"|\" + Строка(С.ИдентификаторНазначения));\n\
             Повтор = Задание.ПолучитьСообщенияПользователю();\n\
             Сообщить(\"после drain: \" + Повтор.Количество());",
            done.id.to_uuid_string()
        ))
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let text = out.text();
    assert!(
        text.contains("текст|Объект.Реквизит|Данные|42|форма"),
        "поля обязаны пережить границу: {text}"
    );
    assert!(
        text.contains("после drain: 0"),
        "drain у terminal-задания обязан быть атомарным: {text}"
    );
}

/// Backpressure внешнего sink: `Сообщить` возвращает ловимую ошибку, но
/// история сообщения задания не теряется.
#[derive(Debug)]
struct RefusingSink;

impl open_bsl::UserMessageSink for RefusingSink {
    fn enqueue(&self, _message: &UserMessageDto) -> Result<(), HostError> {
        Err(HostError::new(
            HostErrorCode::HostBackpressure,
            "очередь представления полна",
        ))
    }
}

#[test]
fn display_backpressure_is_catchable_and_history_survives() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Сказать() Экспорт\n\
                 Попытка\n\
                     Сообщить(\"важное\");\n\
                 Исключение\n\
                     // Ловимая ошибка backpressure; сообщение уже в истории.\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .job_message_sink(Arc::new(RefusingSink))
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Сказать");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );
    assert_eq!(
        message_texts(&runtime, done.id),
        ["важное"],
        "история не теряется при отказе внешнего sink"
    );
}

/// Foreground-сеанс с внедрённым sink: глобальный `Сообщить` отдаёт DTO
/// в sink, а не строку в stdout.
#[derive(Default)]
struct CollectingSink(RefCell<Vec<UserMessageDto>>);

impl open_bsl::UserMessageSink for CollectingSink {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        self.0.borrow_mut().push(message.clone());
        Ok(())
    }
}

#[test]
fn a_session_sink_receives_the_global_message_dto() {
    let engine = Engine::builder().build().expect("движок");
    let sink = Rc::new(CollectingSink::default());
    let out = SharedWriter::default();
    let mut state = engine
        .state_builder()
        .stdout(out.clone())
        .message_sink(sink.clone())
        .build();
    state.exec("Сообщить(\"в sink\");").expect("прогон");
    assert_eq!(out.text(), "", "stdout обязан остаться пустым");
    let collected = sink.0.borrow();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].text, "в sink");
}

// --- 6. Типизированные ошибки и вытеснение -------------------------------

/// Вытеснение задания во время ожидания (история нулевой длины):
/// `ОжидатьЗавершенияВыполнения` задания отвечает ловимой `JobExpired`,
/// а не устаревшим снимком «Активно» — свежего terminal-снимка больше
/// нет, и прежний за него не выдаётся.
#[test]
fn eviction_during_the_job_wait_raises_job_expired() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            max_history_jobs: 0,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Попытка\n\
                 Задание = Задание.ОжидатьЗавершенияВыполнения();\n\
                 Сообщить(\"ожидание вернуло: \" + Задание.Состояние);\n\
             Исключение\n\
                 Сообщить(\"ожидание: \" + ИнформацияОбОшибке().Описание);\n\
             КонецПопытки;",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let text = out.text();
    assert!(
        text.contains("ожидание: ") && text.contains("вытесн"),
        "вытеснение во время ожидания обязано отвечать JobExpired: {text}"
    );
    assert!(
        !text.contains("ожидание вернуло"),
        "устаревший снимок не имеет права выдаваться за свежий: {text}"
    );
}

/// Вытеснение во время менеджерного ожидания: результат не сжимается
/// молча — вытесненное задание отвечает ловимой `JobExpired`.
#[test]
fn eviction_during_the_manager_wait_raises_job_expired() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            max_history_jobs: 0,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "Задание = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Массив = Новый Массив;\n\
             Массив.Добавить(Задание);\n\
             Попытка\n\
                 Обновлённые = ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Массив);\n\
                 Сообщить(\"размер: \" + Обновлённые.Количество());\n\
             Исключение\n\
                 Сообщить(\"ожидание: \" + ИнформацияОбОшибке().Описание);\n\
             КонецПопытки;",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let text = out.text();
    assert!(
        text.contains("ожидание: ") && text.contains("вытесн"),
        "вытеснение во время менеджерного ожидания обязано отвечать JobExpired: {text}"
    );
    assert!(
        !text.contains("размер:"),
        "результат менеджерного ожидания не имеет права менять размер: {text}"
    );
}

/// Live-методы вытесненного снимка — ловимая `JobExpired`; свойства уже
/// материализованного снимка читаются.
#[test]
fn evicted_snapshot_live_methods_raise_job_expired() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            max_history_jobs: 1,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let out = SharedWriter::default();
    let mut state = engine.state_builder().stdout(out.clone()).build();
    let module = engine
        .compile_entry(
            "Первое = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Первое.ОжидатьЗавершенияВыполнения();\n\
             Второе = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
             Второе.ОжидатьЗавершенияВыполнения();\n\
             // История хранит одно задание: Первое вытеснено.\n\
             Сообщить(\"имя: \" + Первое.ИмяМетода);\n\
             Попытка\n\
                 Первое.Отменить();\n\
                 Сообщить(\"отмена прошла\");\n\
             Исключение\n\
                 Сообщить(\"отмена: \" + ИнформацияОбОшибке().Описание);\n\
             КонецПопытки;\n\
             Попытка\n\
                 Сообщения = Первое.ПолучитьСообщенияПользователю();\n\
                 Сообщить(\"сообщения: \" + Сообщения.Количество());\n\
             Исключение\n\
                 Сообщить(\"сообщения: \" + ИнформацияОбОшибке().Описание);\n\
             КонецПопытки;\n\
             Попытка\n\
                 Первое.ОжидатьЗавершенияВыполнения();\n\
                 Сообщить(\"ожидание прошло\");\n\
             Исключение\n\
                 Сообщить(\"ожидание: \" + ИнформацияОбОшибке().Описание);\n\
             КонецПопытки;",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    let text = out.text();
    assert!(
        text.contains("имя: Служебный.Пусто"),
        "свойства снимка обязаны читаться: {text}"
    );
    for (label, forbidden) in [
        ("отмена:", "отмена прошла"),
        ("сообщения:", "сообщения: 0"),
        ("ожидание:", "ожидание прошло"),
    ] {
        assert!(
            text.contains(&format!("{label} задание неизвестно")) && !text.contains(forbidden),
            "live-метод вытесненного снимка обязан отвечать JobExpired: {text}"
        );
    }
}

/// После закрытия runtime live-методы отвечают ловимой `RuntimeClosed`,
/// а свойства уже материализованного снимка остаются читаемыми.
#[test]
fn a_closed_runtime_raises_runtime_closed_but_properties_survive() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Пусто() Экспорт\n\
             КонецПроцедуры",
        )
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Пусто");
    runtime.shutdown(Duration::from_secs(5));

    // Rust-уровень: типизированные коды.
    let error = runtime
        .cancel(done.id)
        .expect_err("закрытый runtime отвечает ошибкой");
    assert_eq!(error.code, HostErrorCode::RuntimeClosed);
    let error = runtime
        .take_messages(done.id, false)
        .expect_err("сообщения закрытого runtime");
    assert_eq!(error.code, HostErrorCode::RuntimeClosed);
    let error = runtime
        .submit_by_name("Служебный.Пусто", empty_params(), None, None)
        .expect_err("submit после закрытия");
    assert_eq!(error.code, HostErrorCode::RuntimeClosed);

    // Свойства уже материализованного снимка читаемы.
    assert_eq!(done.method_name, "Служебный.Пусто");
    assert_eq!(done.state, JobStateDto::Completed);
}

/// Дренаж сообщений у ЖИВОГО задания атомарен: повторное чтение пусто,
/// задание продолжает работать.
#[test]
fn live_message_drain_is_atomic() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Болтун() Экспорт\n\
                 Сообщить(\"старт\");\n\
                 Пока Истина Цикл\n\
                 КонецЦикла;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            workers: Some(1),
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let snapshot = runtime
        .submit_by_name("Служебный.Болтун", empty_params(), None, None)
        .expect("задание принято");
    // Дождаться первого сообщения работающего задания.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let messages = runtime.take_messages(snapshot.id, false).expect("чтение");
        if !messages.is_empty() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "задание не сообщило за 30 секунд"
        );
        std::thread::yield_now();
    }
    let drained = runtime.take_messages(snapshot.id, true).expect("дренаж");
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].text, "старт");
    assert!(
        runtime
            .take_messages(snapshot.id, false)
            .expect("повторное чтение")
            .is_empty(),
        "повторное чтение после drain обязано быть пустым"
    );
    runtime.cancel(snapshot.id).expect("отмена");
    assert!(
        runtime
            .wait_terminal(&[snapshot.id], WAIT)
            .expect("ожидание")
    );
}

/// Счётчик глобального staging-бюджета и кумулятивный бюджет сообщений
/// не зависят от порядка чтения: дренаж не возвращает байты сообщений.
#[test]
fn message_budget_is_cumulative_across_drains() {
    let engine = Engine::builder()
        .common_module(
            "Служебный",
            "Процедура Дважды() Экспорт\n\
                 Большое = \"м\";\n\
                 Для Н = 1 По 9 Цикл\n\
                     Большое = Большое + Большое;\n\
                 КонецЦикла;\n\
                 Сообщить(Большое);\n\
                 Попытка\n\
                     Сообщить(Большое);\n\
                     Сообщить(Большое);\n\
                 Исключение\n\
                     Сообщить(\"отказ\");\n\
                 КонецПопытки;\n\
             КонецПроцедуры",
        )
        .background_jobs(BackgroundJobConfig {
            // Две строки по ~1 КиБ помещаются, третья — нет.
            max_message_bytes_per_job: 2200,
            ..BackgroundJobConfig::default()
        })
        .build()
        .expect("движок собирается");
    let runtime = engine.job_runtime().expect("runtime");
    let done = run_job(&runtime, "Служебный.Дважды");
    assert_eq!(
        done.state,
        JobStateDto::Completed,
        "ошибка: {:?}",
        done.error
    );
    let texts = message_texts(&runtime, done.id);
    assert_eq!(
        texts.last().map(String::as_str),
        Some("отказ"),
        "третье сообщение обязано упереться в кумулятивный бюджет: {texts:?}"
    );
}

/// Учебный счётчик вызовов профиля: фабрика вызывается в потоке worker
/// по разу на задание своего профиля.
struct CountingProfile(Arc<AtomicUsize>);

impl BackgroundStateFactory for CountingProfile {
    fn configure(&self, builder: StateBuilder) -> Result<StateBuilder, String> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(builder)
    }
}

#[test]
fn the_profile_factory_runs_once_per_job() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut builder = Engine::builder().common_module(
        "Служебный",
        "Процедура Пусто() Экспорт\n\
         КонецПроцедуры",
    );
    let profile = builder.register_host_profile(Arc::new(CountingProfile(Arc::clone(&calls))));
    let engine = builder.build().expect("движок собирается");
    let mut state = engine
        .state_builder()
        .host_profile(profile)
        .expect("профиль валиден")
        .build();
    let module = engine
        .compile_entry(
            "Для Н = 1 По 3 Цикл\n\
                 Задание = ФоновыеЗадания.Выполнить(\"Служебный.Пусто\");\n\
                 Задание.ОжидатьЗавершенияВыполнения();\n\
             КонецЦикла;",
        )
        .expect("entry компилируется");
    state.run(&module).expect("прогон");
    assert_eq!(calls.load(Ordering::Relaxed), 3);
}
