//! Расширение Zed: подключение к отладчику open-bsl.
//!
//! Оно НЕ запускает адаптер. `bsl-cli --debug` — сервер, который слушает
//! и ждёт, поэтому расширению остаётся сказать Zed, куда подключаться:
//! в `DebugAdapterBinary` команда пустая, а заполнен `connection`. Так
//! прямо и написано в описании интерфейса: «Zed will use TCP transport if
//! `connection` is specified».
//!
//! Порядок работы от этого один и тот же и менять его нельзя: сначала
//! процесс, потом редактор.

use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario,
    DebugTaskDefinition, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TcpArgumentsTemplate, resolve_tcp_template,
};

/// Умолчания те же, что у `bsl-cli`: петля и 4711.
///
/// Петля не для удобства: отладчик вычисляет произвольный BSL, и открытый
/// наружу порт равен удалённому запуску кода.
const DEFAULT_HOST: u32 = u32::from_be_bytes([127, 0, 0, 1]);
const DEFAULT_PORT: u16 = 4711;

struct OpenBslDebug;

impl zed::Extension for OpenBslDebug {
    fn new() -> Self {
        Self
    }

    fn get_dap_binary(
        &mut self,
        _adapter_name: String,
        config: DebugTaskDefinition,
        _user_installed_path: Option<String>,
        _worktree: &zed::Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        // Что задал пользователь в `tcp_connection`, то и берём; чего не
        // задал — умолчания `bsl-cli`, чтобы конфигурация из двух строк
        // работала без подробностей.
        let template = config.tcp_connection.unwrap_or(TcpArgumentsTemplate {
            host: None,
            port: None,
            timeout: None,
        });
        let connection = resolve_tcp_template(TcpArgumentsTemplate {
            host: Some(template.host.unwrap_or(DEFAULT_HOST)),
            port: Some(template.port.unwrap_or(DEFAULT_PORT)),
            timeout: template.timeout,
        })?;
        Ok(DebugAdapterBinary {
            // Пусто намеренно: процесс уже запущен пользователем, и
            // запускать второй значило бы отлаживать не ту программу.
            command: None,
            arguments: Vec::new(),
            envs: Vec::new(),
            cwd: None,
            connection: Some(connection),
            request_args: StartDebuggingRequestArguments {
                configuration: config.config,
                // Только присоединение: программу выбирает командная
                // строка `bsl-cli`, а не редактор.
                request: StartDebuggingRequestArgumentsRequest::Attach,
            },
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
        // Из окна «новый сеанс» приходит запрос запуска или присоединения.
        // Запускать мы не умеем — и говорим об этом, а не делаем вид.
        if matches!(config.request, DebugRequest::Launch(_)) {
            return Err(
                "open-bsl не запускает программу из редактора: сначала запустите \
                 `bsl-cli --debug скрипт.bsl`, потом присоединяйтесь"
                    .to_string(),
            );
        }
        Ok(DebugScenario {
            label: config.label,
            adapter: config.adapter,
            build: None,
            config: "{}".to_string(),
            tcp_connection: Some(TcpArgumentsTemplate {
                host: Some(DEFAULT_HOST),
                port: Some(DEFAULT_PORT),
                timeout: None,
            }),
        })
    }
}

zed::register_extension!(OpenBslDebug);
