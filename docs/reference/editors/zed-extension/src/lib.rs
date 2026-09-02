//! Расширение Zed: отладка open-bsl.
//!
//! Два режима, и выбирает их наличие поля `program` в конфигурации.
//!
//! **Запуск.** Есть `program` — Zed сам поднимает `bsl-cli --debug` на
//! свободном порту и подключается к нему. Так работает кнопка «начать
//! отладку»: пользователю не нужно ничего заводить в терминале заранее.
//! Интерфейс это позволяет прямо: в `DebugAdapterBinary` можно задать и
//! команду, и `connection`, и тогда Zed запускает процесс, а транспортом
//! берёт TCP.
//!
//! **Присоединение.** `program` нет — значит, процесс уже запущен руками
//! (`bsl-cli --debug скрипт.bsl`), и расширение только говорит, куда
//! подключаться: команда пустая, заполнен `connection`.
//!
//! Первый режим появился потому, что без него редактор показывал
//! «Connection to TCP DAP timeout»: нажать кнопку и получить отказ, если
//! забыл отдельно запустить процесс, — это не отладчик, а загадка.

use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DebugTaskDefinition,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, TcpArgumentsTemplate,
    resolve_tcp_template,
};

/// Умолчания те же, что у `bsl-cli`: петля и 4711.
///
/// Петля не для удобства: отладчик вычисляет произвольный BSL, и открытый
/// наружу порт равен удалённому запуску кода.
const DEFAULT_HOST: u32 = u32::from_be_bytes([127, 0, 0, 1]);
const DEFAULT_PORT: u16 = 4711;

struct OpenBslDebug;

/// Строковое поле конфигурации, если оно задано и непусто.
fn field(config: &zed::serde_json::Value, name: &str) -> Option<String> {
    config
        .get(name)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

impl zed::Extension for OpenBslDebug {
    fn new() -> Self {
        Self
    }

    fn get_dap_binary(
        &mut self,
        _adapter_name: String,
        config: DebugTaskDefinition,
        user_installed_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        let parsed: zed::serde_json::Value =
            zed::serde_json::from_str(&config.config).unwrap_or(zed::serde_json::Value::Null);

        // Порт: что просили — то и берём; иначе умолчание `bsl-cli`.
        let template = config.tcp_connection.unwrap_or(TcpArgumentsTemplate {
            host: None,
            port: None,
            timeout: None,
        });
        let port = template.port.unwrap_or(DEFAULT_PORT);
        let connection = resolve_tcp_template(TcpArgumentsTemplate {
            host: Some(template.host.unwrap_or(DEFAULT_HOST)),
            port: Some(port),
            timeout: template.timeout,
        })?;

        let request_args = StartDebuggingRequestArguments {
            configuration: config.config.clone(),
            // Всегда присоединение: какую программу исполнять, решает
            // командная строка `bsl-cli`, а не редактор.
            request: StartDebuggingRequestArgumentsRequest::Attach,
        };

        // Переменные вида `$ZED_FILE` Zed подставляет в `configuration`,
        // которую отдаёт адаптеру при старте сеанса, — но не в поля,
        // которые расширение читает ЗДЕСЬ. Неподставленное имя ушло бы в
        // командную строку, `bsl-cli` не нашёл бы такого файла и вышел с
        // ошибкой ДО открытия порта, а редактор показал бы «process
        // exited before debugger attached» — сообщение, по которому
        // причину не найти. Поэтому отказ здесь, с названным значением.
        if let Some(raw) = field(&parsed, "program")
            && raw.contains('$')
        {
            return Err(format!(
                "в поле «program» осталась неподставленная переменная: «{raw}». \
                 Укажите путь к скрипту — абсолютный либо от корня проекта."
            ));
        }
        let Some(program) = field(&parsed, "program") else {
            // Присоединение к уже запущенному процессу.
            return Ok(DebugAdapterBinary {
                command: None,
                arguments: Vec::new(),
                envs: Vec::new(),
                cwd: None,
                connection: Some(connection),
                request_args,
            });
        };

        // Запуск. Путь к `bsl-cli` берётся из настройки расширения, из
        // поля конфигурации либо из PATH — в таком порядке: явное
        // указание важнее найденного.
        let cli = user_installed_path
            .or_else(|| field(&parsed, "bsl_cli"))
            .or_else(|| worktree.which("bsl-cli"))
            .ok_or_else(|| {
                "не найден bsl-cli: укажите путь полем «bsl_cli» в конфигурации \
                 либо положите его в PATH"
                    .to_string()
            })?;
        // Заданный путь может быть относительным — и считается он от
        // корня проекта, как `program`. Так конфигурация ссылается на
        // сборку рядом с собой, не называя ничьего домашнего каталога и
        // не завися от того, что оказалось в PATH.
        let cli = if cli.contains('/') && !cli.starts_with('/') {
            format!("{}/{}", worktree.root_path(), cli)
        } else {
            cli
        };
        // Найденный `bsl-cli` может не уметь отлаживать: отладка
        // появилась недавно, и ссылка на сборку из другой копии
        // репозитория — обычное дело. Такой отвечает «неизвестная
        // команда «--debug»» и выходит до открытия порта, то есть даёт
        // ровно то же «process exited before debugger attached». Один
        // `--help` отделяет одно от другого. Если сам запуск не удался,
        // молчим и идём дальше: проверка вправе добавить знание, но не
        // отнять возможность запуска.
        if let Ok(out) = zed::process::Command::new(&cli).arg("--help").output() {
            let mut help = String::from_utf8_lossy(&out.stdout).into_owned();
            help.push_str(&String::from_utf8_lossy(&out.stderr));
            if !help.contains("--debug") {
                return Err(format!(
                    "«{cli}» не знает ключа --debug — это сборка без отладчика. \
                     Соберите bsl-cli из этого репозитория либо укажите нужный \
                     полем «bsl_cli»."
                ));
            }
        }
        // Относительный путь считается от корня проекта, открытого в
        // Zed, — не от каталога, где лежит `.zed/debug.json`. Разница
        // заметна не сразу, а ошибка от неё молчалива: файла нет,
        // процесс умирает до открытия порта. Поэтому читаем файл и, если
        // не вышло, называем корень, от которого считали.
        let program = if program.starts_with('/') {
            program
        } else {
            if worktree.read_text_file(&program).is_err() {
                return Err(format!(
                    "от корня проекта «{}» файл «{program}» не читается. Путь в \
                     поле «program» считается от корня, открытого в Zed, а не от \
                     каталога с .zed/debug.json.",
                    worktree.root_path()
                ));
            }
            format!("{}/{}", worktree.root_path(), program)
        };
        Ok(DebugAdapterBinary {
            command: Some(cli),
            arguments: vec![
                "--debug".to_string(),
                "--debug-port".to_string(),
                port.to_string(),
                program,
            ],
            envs: Vec::new(),
            cwd: field(&parsed, "cwd").or_else(|| Some(worktree.root_path())),
            connection: Some(connection),
            request_args,
        })
    }

    fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        _config: zed::serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        Ok(StartDebuggingRequestArgumentsRequest::Attach)
    }

    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario, String> {
        // Из окна «новый сеанс» приходит либо запуск с программой, либо
        // присоединение. Оба ложатся в один сценарий; разницу делает
        // наличие `program`.
        let body = match &config.request {
            DebugRequest::Launch(launch) => {
                zed::serde_json::json!({ "program": launch.program, "cwd": launch.cwd })
            }
            DebugRequest::Attach(_) => zed::serde_json::json!({}),
        };
        Ok(DebugScenario {
            label: config.label,
            adapter: config.adapter,
            build: None,
            config: body.to_string(),
            tcp_connection: Some(TcpArgumentsTemplate {
                host: Some(DEFAULT_HOST),
                port: Some(DEFAULT_PORT),
                timeout: None,
            }),
        })
    }
}

zed::register_extension!(OpenBslDebug);
