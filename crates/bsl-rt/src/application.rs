//! Нейтральная host-граница запуска приложений без командного интерпретатора.

use std::{fmt, io};

use crate::SecretString;

/// Выполняет нормализованный RunApp либо возвращает операцию ожидания VM.
/// Четыре аргумента подготовлены резолвером; внутренний результат — новое
/// значение выходного аргумента, а не разрешение процедурного вызова в выражении.
///
/// # Errors
/// Неверные типы/число аргументов, отсутствующий launcher, неразбираемая
/// Unix-команда или отказ принятия host. Ошибки завершения приходят через mapper.
pub fn call_run_app(
    args: &[crate::BslValue],
    env: &crate::HostEnv,
) -> crate::RtResult<crate::CallOutcome> {
    use crate::{BslValue, CallOutcome, RtError};
    let [command, directory, wait, _output] = args else {
        return Err(RtError::InvalidBytecode(
            "RunApp требует четыре нормализованных аргумента",
        ));
    };
    // НЕ ИЗМЕРЕНО(RUNAPP.EDGE_ARGUMENTS)
    // Строгие строки не подменяют отсутствующие замеры иных преобразований.
    let command = command.as_str("ЗапуститьПриложение")?.to_string();
    let directory = directory.as_str("ЗапуститьПриложение")?.to_string();
    let BslValue::Boolean(wait) = wait else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ЗапуститьПриложение",
        });
    };
    let launcher = env
        .application_launcher()
        .ok_or_else(|| RtError::IoError("host не предоставляет запуск приложений".into()))?;
    if command.is_empty() {
        return Ok(CallOutcome::Ready(if *wait {
            BslValue::number_from_i64(0)
        } else {
            BslValue::Undefined
        }));
    }
    if !cfg!(unix) {
        return Err(RtError::IoError(
            "разбор строки запуска реализован только для Unix".into(),
        ));
    }
    let request = ApplicationRequest {
        target: ApplicationTarget::from_unix_command_line(&command).map_err(application_error)?,
        wait_for_exit: *wait,
        working_directory: if directory.is_empty() {
            None
        } else {
            Some(directory)
        },
    };
    Ok(CallOutcome::Pending(
        crate::PendingHostCall::ApplicationSync {
            launcher,
            request,
            mapper: if *wait {
                application_result
            } else {
                application_started
            },
            error_mapper: application_error,
        },
    ))
}

/// Готовит измеренный async-вызов; VM превращает исход в обещание.
/// Синхронный вход разделяет запуск, но не преобразование NotFound.
///
/// # Errors
/// Неверные аргументы, отсутствие возможности host либо отказ подготовки.
/// Поддерживающий исполнитель обязан доставить эти ошибки через обещание.
pub fn call_run_app_async(
    args: &[crate::BslValue],
    env: &crate::HostEnv,
) -> crate::RtResult<crate::CallOutcome> {
    use crate::{BslString, BslValue, CallOutcome, PendingHostCall, RtError};
    let [command, directory, wait] = args else {
        return Err(RtError::InvalidBytecode(
            "RunAppAsync требует три нормализованных аргумента",
        ));
    };
    let command = command.as_str("ЗапуститьПриложениеАсинх")?.to_string();
    let command = command.split('\0').next().unwrap_or_default();
    let outcome = call_run_app(
        &[
            BslValue::Str(BslString::from_str(command)),
            directory.clone(),
            wait.clone(),
            BslValue::Undefined,
        ],
        env,
    )?;
    Ok(match outcome {
        CallOutcome::Pending(PendingHostCall::ApplicationSync {
            launcher,
            request,
            error_mapper,
            ..
        }) => CallOutcome::Pending(PendingHostCall::ApplicationSync {
            launcher,
            mapper: if request.wait_for_exit {
                application_async_result
            } else {
                application_started
            },
            request,
            error_mapper,
        }),
        other => other,
    })
}

fn application_async_result(
    result: io::Result<ApplicationResult>,
    shapes: &mut crate::RuntimeShapes,
) -> crate::RtResult<crate::BslValue> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(crate::BslValue::Undefined),
        other => application_result(other, shapes),
    }
}

fn application_error(error: io::Error) -> crate::RtError {
    // Host может вложить команду/секреты в текст ошибки: наружу идёт только класс.
    crate::RtError::IoError(format!("ошибка запуска приложения: {:?}", error.kind()))
}

fn application_result(
    result: io::Result<ApplicationResult>,
    _: &mut crate::RuntimeShapes,
) -> crate::RtResult<crate::BslValue> {
    match result {
        Ok(ApplicationResult::Exited(ApplicationExit { code: Some(code) })) => {
            Ok(crate::BslValue::number_from_i64(i64::from(code)))
        }
        Ok(ApplicationResult::Exited(ApplicationExit { code: None })) => Err(
            crate::RtError::IoError("приложение завершилось без числового кода".into()),
        ),
        Ok(ApplicationResult::Started | ApplicationResult::DocumentOpened) => Err(
            crate::RtError::IoError("host не предоставил завершение приложения".into()),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(crate::BslValue::number_from_i64(127))
        }
        Err(error) => Err(application_error(error)),
    }
}

fn application_started(
    result: io::Result<ApplicationResult>,
    _: &mut crate::RuntimeShapes,
) -> crate::RtResult<crate::BslValue> {
    match result {
        Ok(_) => Ok(crate::BslValue::Undefined),
        // Прежний контракт RunApp без ожидания не сообщает отсутствующий executable.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(crate::BslValue::Undefined),
        Err(error) => Err(application_error(error)),
    }
}

/// Явный вид запуска; открытие документа не маскируется под исполняемый файл.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationTarget {
    /// Аргументы уже разделены вызывающим; host не разбирает их как shell-код.
    Executable {
        program: String,
        arguments: Vec<SecretString>,
    },
    /// Host выбирает зарегистрированную ассоциацию для документа.
    Document { path: String },
}

impl ApplicationTarget {
    /// Разбирает Unix-кавычки и экранирование в имя программы и argv.
    ///
    /// Это не shell: переменные, команды, wildcard и операторы не
    /// интерпретируются. Пробел, табуляция и LF вне кавычек разделяют слова;
    /// пустые quoted-аргументы сохраняются. Метод не выполняет I/O и не
    /// определяет синтаксис командной строки Windows.
    ///
    /// # Errors
    ///
    /// `InvalidInput` при NUL, незакрытой кавычке, оборванном экранировании
    /// или пустом имени программы. Диагностика не раскрывает входную строку.
    pub fn from_unix_command_line(command: &str) -> io::Result<Self> {
        enum Quote {
            None,
            Single,
            Double,
        }
        let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "некорректная Unix-команда");
        if command.contains('\0') {
            return Err(invalid());
        }
        let mut quote = Quote::None;
        let mut words = Vec::new();
        let mut word = String::new();
        let mut started = false;
        let mut chars = command.chars();
        while let Some(ch) = chars.next() {
            match quote {
                Quote::Single => {
                    if ch == '\'' {
                        quote = Quote::None;
                    } else {
                        word.push(ch);
                    }
                }
                Quote::Double => match ch {
                    '"' => quote = Quote::None,
                    '\\' => {
                        let next = chars.next().ok_or_else(invalid)?;
                        if next != '\n' {
                            if !matches!(next, '$' | '`' | '"' | '\\') {
                                word.push('\\');
                            }
                            word.push(next);
                        }
                    }
                    _ => word.push(ch),
                },
                Quote::None => match ch {
                    ' ' | '\t' | '\n' => {
                        if started {
                            words.push(std::mem::take(&mut word));
                            started = false;
                        }
                    }
                    '\'' | '"' => {
                        started = true;
                        quote = if ch == '\'' {
                            Quote::Single
                        } else {
                            Quote::Double
                        };
                    }
                    '\\' => {
                        let next = chars.next().ok_or_else(invalid)?;
                        if next != '\n' {
                            started = true;
                            word.push(next);
                        }
                    }
                    _ => {
                        started = true;
                        word.push(ch);
                    }
                },
            }
        }
        if !matches!(quote, Quote::None) {
            return Err(invalid());
        }
        if started {
            words.push(word);
        }
        let mut words = words.into_iter();
        let program = words
            .next()
            .filter(|word| !word.is_empty())
            .ok_or_else(invalid)?;
        Ok(Self::Executable {
            program,
            arguments: words.map(SecretString::new).collect(),
        })
    }
}

/// Запрос запуска в пространстве и с полномочиями host, не виртуальной ФС BSL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationRequest {
    pub target: ApplicationTarget,
    /// Ожидать завершения приложения, а не только запуска/передачи документа.
    pub wait_for_exit: bool,
    pub working_directory: Option<String>,
}

/// Нативный результат завершения. Отсутствие кода, например при сигнале ОС,
/// не подменяется нулём; преобразование в BSL-код — отдельный контракт.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationExit {
    pub code: Option<i32>,
}

/// Подтверждение host: запуск и передача документа не являются exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationResult {
    /// Процесс создан; host независимо продолжает получать его статус.
    Started,
    /// Получен настоящий статус завершения процесса.
    Exited(ApplicationExit),
    /// Документ передан обработчику ассоциации; его завершение не наблюдается.
    DocumentOpened,
}

/// Материализация завершения приложения в вызывающем BSL-потоке.
/// Наблюдатель процесса передаёт только нативный результат и не строит BSL-объекты.
pub type ApplicationResponseMapper = fn(
    io::Result<ApplicationResult>,
    &mut crate::RuntimeShapes,
) -> crate::RtResult<crate::BslValue>;

/// Преобразование отказа launcher до принятия операции.
pub type ApplicationErrorMapper = fn(io::Error) -> crate::RtError;

/// Одноразовая доставка результата в контур ожидания runtime.
pub trait ApplicationCompletionSink: Send {
    fn complete(self: Box<Self>, result: io::Result<ApplicationResult>);
}

/// Возможность запуска и независимого наблюдения за завершением процесса.
///
/// Отмена BSL-ожидания не отменяет программу: после принятия операции host
/// сохраняет ответственность за получение статуса, даже если результат уже
/// никому не нужен. Здесь намеренно нет аналога HTTP RequestHandle::cancel.
pub trait ApplicationLauncher: fmt::Debug + Send + Sync {
    /// Принимает запуск без ожидания завершения программы в вызывающем потоке.
    /// При принятии запрошенное подтверждение ровно один раз передаётся в sink;
    /// без wait_for_exit это запуск/передача, с ним — завершение. Быстрая
    /// операция может завершить callback ещё до возврата submit.
    ///
    /// # Errors
    /// Ошибка до принятия запроса; в этом случае sink не вызывается.
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()>;
}

/// Системный запуск executable через `std::process::Command`, без shell.
///
/// Наблюдатель владеет дочерним процессом до wait; освобождение сервиса или
/// получателя результата не завершает программу. На Linux документы
/// передаются через gio; ожидание их приложения возвращает Unsupported.
/// Сервис не внедряется
/// автоматически в HostEnv::process или embedding.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemApplicationLauncher;

impl ApplicationLauncher for SystemApplicationLauncher {
    fn submit(
        &self,
        request: ApplicationRequest,
        sink: Box<dyn ApplicationCompletionSink>,
    ) -> io::Result<()> {
        // Сначала создаётся наблюдатель: если ресурсов для потока нет,
        // дочерняя программа ещё не запущена и не останется без владельца.
        let (started, starting) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("open-bsl-process".into())
            .spawn(move || {
                let mut sink = Some(sink);
                let result = (|| {
                    let (mut command, document) = system_command(&request)?;
                    let child = command.spawn();
                    // Даже не ожидаемое обещание не должно потерять запуск
                    // при выходе CLI. Подтверждаем попытку spawn, но не
                    // ждём ни завершения executable, ни ответа gio.
                    let _ = started.send(());
                    let mut child = child.map_err(|error| {
                        if document {
                            io::Error::other(format!(
                                "отказ открытия документа: {:?}",
                                error.kind()
                            ))
                        } else {
                            error
                        }
                    })?;
                    if !request.wait_for_exit
                        && !document
                        && let Some(sink) = sink.take()
                    {
                        sink.complete(Ok(ApplicationResult::Started));
                    }
                    let status = child.wait()?;
                    if document {
                        return if status.success() {
                            Ok(ApplicationResult::DocumentOpened)
                        } else {
                            Err(io::Error::other(
                                "документ не передан обработчику ассоциации",
                            ))
                        };
                    }
                    Ok(ApplicationResult::Exited(ApplicationExit {
                        code: status.code(),
                    }))
                })();
                // При отказе подготовки попытки spawn нет: освобождаем
                // вызывающий поток и доставляем отказ обычным путём sink.
                let _ = started.send(());
                if let Some(sink) = sink {
                    sink.complete(result);
                }
            })?;
        starting
            .recv()
            .map_err(|_| io::Error::other("наблюдатель приложения завершился до запуска"))?;
        Ok(())
    }
}

/// Разрешение относится к пространству host, а не к виртуальной ФС BSL.
fn system_command(request: &ApplicationRequest) -> io::Result<(std::process::Command, bool)> {
    let document = match &request.target {
        ApplicationTarget::Document { path } => Some(path.as_str()),
        ApplicationTarget::Executable { program, arguments } => {
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::fs::PermissionsExt;
                let path = document_path(program, request.working_directory.as_deref())?;
                if arguments.is_empty()
                    && std::fs::metadata(path).is_ok_and(|metadata| {
                        metadata.is_file() && metadata.permissions().mode() & 0o111 == 0
                    })
                {
                    Some(program.as_str())
                } else {
                    None
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (program, arguments);
                None
            }
        }
    };
    let mut command = if let Some(path) = document {
        if !cfg!(target_os = "linux") || request.wait_for_exit {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "ожидание приложения документа недоступно",
            ));
        }
        let path = document_path(path, request.working_directory.as_deref())?;
        // Не даём локальному имени стать опцией или URI gio. Диагностика
        // дочерней утилиты может раскрывать путь; наружу отдаём только класс.
        let mut command = std::process::Command::new("gio");
        command.args([std::ffi::OsStr::new("open"), path.as_os_str()]);
        command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command
    } else {
        let ApplicationTarget::Executable { program, arguments } = &request.target else {
            unreachable!("документ обработан выше");
        };
        let mut command = std::process::Command::new(program);
        command.args(arguments.iter().map(SecretString::expose));
        command
    };
    if let Some(directory) = &request.working_directory {
        command.current_dir(directory);
    }
    Ok((command, document.is_some()))
}

fn document_path(path: &str, directory: Option<&str>) -> io::Result<std::path::PathBuf> {
    if path.is_empty() || path.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "некорректный путь документа",
        ));
    }
    let path = std::path::Path::new(path);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let directory = std::path::Path::new(directory.unwrap_or(""));
    let directory = if directory.is_absolute() {
        directory.to_path_buf()
    } else {
        std::env::current_dir()?.join(directory)
    };
    Ok(directory.join(path))
}
