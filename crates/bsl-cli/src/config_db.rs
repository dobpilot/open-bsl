//! Параметры запуска скрипта в сеансе информационной базы.

#[path = "config_db_files.rs"]
mod files;

#[cfg(test)]
#[path = "config_db_session_kind_tests.rs"]
mod session_kind_tests;

#[cfg(test)]
#[path = "config_db_decompilation_tests.rs"]
mod decompilation_tests;

#[cfg(test)]
#[path = "config_db_connection_string_tests.rs"]
mod connection_string_tests;

/// Провайдер информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    Postgres,
    MsSql,
}

impl Provider {
    const fn default_port(self) -> u16 {
        match self {
            Self::Postgres => 5432,
            Self::MsSql => 1433,
        }
    }
}

/// Проверенные параметры одного запуска через базу.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    pub(crate) provider: Provider,
    pub(crate) file_source: Option<files::FileSourceOptions>,
    pub(crate) extension_sources: Vec<(String, std::path::PathBuf)>,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) database: String,
    pub(crate) user: String,
    pub(crate) platform_version: String,
    pub(crate) infobase_connection_string: Option<String>,
    pub(crate) principal: Option<String>,
    pub(crate) session_kind: bsl_session::SessionKind,
    pub(crate) locale_code: Option<String>,
    pub(crate) infobase_locale_code: Option<String>,
    pub(crate) system_language: Option<String>,
    pub(crate) postgres_plaintext: bool,
    pub(crate) mssql_trust_server_certificate: bool,
    pub(crate) decompile_protected_modules: bool,
}

const VALUE_FLAGS: &[&str] = &[
    "--config-db",
    "--config-xml",
    "--config-edt",
    "--config-extension",
    "--db-host",
    "--db-port",
    "--db-name",
    "--db-user",
    "--platform-version",
    "--infobase-connection-string",
    "--principal",
    "--session-kind",
    "--locale-code",
    "--infobase-locale-code",
    "--system-language",
];
const BOOLEAN_FLAGS: &[&str] = &[
    "--db-plaintext",
    "--db-trust-server-certificate",
    "--decompile-protected-modules",
];
const PASSWORD_FLAGS: &[&str] = &[
    "--password",
    "--db-password",
    "--postgres-password",
    "--mssql-password",
];

#[derive(Default)]
struct Parsed {
    provider: Option<String>,
    config_xml: Option<String>,
    config_edt: Option<String>,
    config_extensions: Vec<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    user: Option<String>,
    platform_version: Option<String>,
    infobase_connection_string: Option<String>,
    principal: Option<String>,
    session_kind: Option<String>,
    locale_code: Option<String>,
    infobase_locale_code: Option<String>,
    system_language: Option<String>,
    postgres_plaintext: bool,
    mssql_trust_server_certificate: bool,
    decompile_protected_modules: bool,
}

/// Извлекает модификаторы базы, оставляя в `args` обычный вызов CLI.
///
/// Без модификаторов базы возвращает `None` и не меняет массив. Ключи после
/// пути скрипта остаются его аргументами, а ключи до пути либо входят в
/// готовые параметры, либо дают ошибку.
pub(crate) fn take(args: &mut Vec<String>) -> Result<Option<Options>, String> {
    let mut parsed = Parsed::default();
    let mut kept = vec![args[0].clone()];
    let mut has_options = false;
    let mut index = 1;
    while index < args.len() {
        let name = flag_name(&args[index]);
        if PASSWORD_FLAGS.contains(&name) {
            return Err(format!(
                "{name} недопустим: пароль берётся из PGPASSWORD или MSSQL_PASSWORD"
            ));
        }
        if BOOLEAN_FLAGS.contains(&name) {
            has_options = true;
            if args[index].contains('=') {
                return Err(format!("{name} не принимает значение"));
            }
            match name {
                "--db-plaintext" => parsed.postgres_plaintext = true,
                "--db-trust-server-certificate" => {
                    parsed.mssql_trust_server_certificate = true;
                }
                "--decompile-protected-modules" => {
                    if parsed.decompile_protected_modules {
                        return Err(format!("{name} задан дважды"));
                    }
                    parsed.decompile_protected_modules = true;
                }
                _ => unreachable!("имя взято из BOOLEAN_FLAGS"),
            }
            index += 1;
            continue;
        }
        if VALUE_FLAGS.contains(&name) {
            has_options = true;
            let (value, consumed) = value(args, index, name)?;
            match name {
                "--config-db" => set(&mut parsed.provider, value, name)?,
                "--config-xml" => set(&mut parsed.config_xml, value, name)?,
                "--config-edt" => set(&mut parsed.config_edt, value, name)?,
                "--config-extension" => parsed.config_extensions.push(value),
                "--db-host" => set(&mut parsed.host, value, name)?,
                "--db-port" => {
                    if parsed.port.is_some() {
                        return Err(format!("{name} задан дважды"));
                    }
                    parsed.port = Some(
                        value
                            .parse::<u16>()
                            .map_err(|_| format!("{name}: порт `{value}` недопустим"))?,
                    );
                }
                "--db-name" => set(&mut parsed.database, value, name)?,
                "--db-user" => set(&mut parsed.user, value, name)?,
                "--platform-version" => set(&mut parsed.platform_version, value, name)?,
                "--infobase-connection-string" => {
                    set(&mut parsed.infobase_connection_string, value, name)?
                }
                "--principal" => set(&mut parsed.principal, value, name)?,
                "--session-kind" => set(&mut parsed.session_kind, value, name)?,
                "--locale-code" => set(&mut parsed.locale_code, value, name)?,
                "--infobase-locale-code" => set(&mut parsed.infobase_locale_code, value, name)?,
                "--system-language" => set(&mut parsed.system_language, value, name)?,
                _ => unreachable!("имя взято из VALUE_FLAGS"),
            }
            index += consumed;
            continue;
        }
        // Первый обычный аргумент — путь скрипта или команда. Остальные
        // аргументы принадлежат ему, даже если похожи на ключи запуска.
        kept.extend_from_slice(&args[index..]);
        break;
    }

    if parsed.infobase_connection_string.as_deref() == Some("") {
        return Err("--infobase-connection-string без значения".into());
    }
    let (file_source, extension_sources) = files::selection(
        parsed.config_xml.take(),
        parsed.config_edt.take(),
        std::mem::take(&mut parsed.config_extensions),
    )?;
    let Some(provider) = parsed.provider.as_deref() else {
        if file_source.is_some() {
            return Err("--config-xml и --config-edt требуют --config-db".into());
        }
        if parsed.infobase_connection_string.is_some() {
            return Err("--infobase-connection-string допустим только с --config-db".into());
        }
        if !has_options {
            return Ok(None);
        }
        if parsed.session_kind.is_some() {
            return Err("--session-kind допустим только с --config-db".into());
        }
        if parsed.decompile_protected_modules {
            return Err("--decompile-protected-modules допустим только с --config-db".into());
        }
        return Err("--principal и --db-* допустимы только с --config-db".into());
    };
    let provider = match provider {
        "postgres" => Provider::Postgres,
        "mssql" => Provider::MsSql,
        _ => return Err("--config-db: допустимы `postgres` или `mssql`".into()),
    };
    validate_security_flags(provider, &parsed)?;
    let session_kind = match parsed.session_kind.as_deref() {
        None | Some("thin-client") => bsl_session::SessionKind::ThinClient,
        Some("web-client") => bsl_session::SessionKind::WebClient,
        Some(_) => return Err("--session-kind: допустимы `thin-client` или `web-client`".into()),
    };
    let required = |value: Option<String>, flag: &str| {
        value.ok_or_else(|| format!("--config-db требует {flag}"))
    };
    let options = Options {
        provider,
        file_source,
        extension_sources,
        host: required(parsed.host, "--db-host")?,
        port: parsed.port.unwrap_or_else(|| provider.default_port()),
        database: required(parsed.database, "--db-name")?,
        user: required(parsed.user, "--db-user")?,
        platform_version: required(parsed.platform_version, "--platform-version")?,
        infobase_connection_string: parsed.infobase_connection_string,
        principal: parsed.principal,
        session_kind,
        locale_code: parsed.locale_code,
        infobase_locale_code: parsed.infobase_locale_code,
        system_language: parsed.system_language,
        postgres_plaintext: parsed.postgres_plaintext,
        mssql_trust_server_certificate: parsed.mssql_trust_server_certificate,
        decompile_protected_modules: parsed.decompile_protected_modules,
    };
    *args = kept;
    Ok(Some(options))
}

fn flag_name(argument: &str) -> &str {
    argument.split_once('=').map_or(argument, |(name, _)| name)
}

fn set(target: &mut Option<String>, value: String, name: &str) -> Result<(), String> {
    if target.replace(value).is_some() {
        Err(format!("{name} задан дважды"))
    } else {
        Ok(())
    }
}

fn value(args: &[String], index: usize, name: &str) -> Result<(String, usize), String> {
    match args[index].split_once('=') {
        Some((_, "")) => Err(format!("{name} без значения")),
        Some((_, value)) => Ok((value.to_string(), 1)),
        None => args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .cloned()
            .map(|value| (value, 2))
            .ok_or_else(|| format!("{name} без значения")),
    }
}

fn validate_security_flags(provider: Provider, parsed: &Parsed) -> Result<(), String> {
    if provider == Provider::Postgres && parsed.mssql_trust_server_certificate {
        return Err("--db-trust-server-certificate допустим только для mssql".into());
    }
    if provider == Provider::MsSql && parsed.postgres_plaintext {
        return Err("--db-plaintext допустим только для postgres".into());
    }
    Ok(())
}

/// Запускает файловый скрипт в сеансе конфигурации из базы.
pub(crate) fn run(path: &str, arguments: Vec<String>, options: Options) {
    if let Err(error) = run_in_session(path, arguments, &options) {
        eprintln!("--config-db: {error}");
        std::process::exit(1);
    }
}

// Протокол диагностических строк совпадает с вложенными этапами фасада.
fn startup_timing(stage: &str, start: std::time::Instant) {
    if open_bsl::startup_timings_enabled() {
        eprintln!(
            "open-bsl.startup\t{stage}\t{:.6}",
            start.elapsed().as_secs_f64()
        );
    }
}

fn run_in_session(path: &str, arguments: Vec<String>, options: &Options) -> Result<(), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("не удалось прочитать «{path}»: {error}"))?;
    let file_source = options
        .file_source
        .as_ref()
        .map(files::FileSourceOptions::open)
        .transpose()?;
    let extension_sources = options
        .file_source
        .as_ref()
        .map(|mode| {
            options
                .extension_sources
                .iter()
                .map(|(id, path)| mode.extension(path).map(|source| (id.clone(), source)))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let connection = database_connection(options);
    let credentials = database_credentials(options.provider)?;
    let mut session = bsl_config_db::open_database_session(
        &connection,
        &credentials,
        &source,
        bsl_config_db::DatabaseSessionOptions {
            load: bsl_config_db::LoadOptions {
                decompile_protected_modules: options.decompile_protected_modules,
            },
            file_source,
            extension_sources,
            principal: options.principal.clone(),
            session_kind: options.session_kind,
            platform_version: options.platform_version.clone(),
            infobase_connection_string: options.infobase_connection_string.clone(),
            locale_code: options.locale_code.clone(),
            infobase_locale_code: options.infobase_locale_code.clone(),
            system_language: options.system_language.clone(),
            application_name: "bsl-cli".into(),
            arguments,
            event_log_writer: Some(std::rc::Rc::new(
                crate::config_db_event_log::CliEventLogWriter,
            )),
        },
    )?;
    let phase = std::time::Instant::now();
    let result = session.execute(&source).map_err(|error| error.to_string());
    startup_timing("cli.execute", phase);
    result
}

fn database_connection(options: &Options) -> open_sdbl_db::DatabaseConnection {
    use open_sdbl_db::{
        ConnectionOptions, DatabaseConnection, MsSqlConnection, PostgresConnection, PostgresSslMode,
    };

    let common = ConnectionOptions {
        host: options.host.clone(),
        port: options.port,
        database: options.database.clone(),
        user: options.user.clone(),
        socks5_proxy: None,
    };
    match options.provider {
        Provider::Postgres => DatabaseConnection::Postgres(PostgresConnection {
            options: common,
            sslmode: if options.postgres_plaintext {
                PostgresSslMode::Disable
            } else {
                PostgresSslMode::VerifyFull
            },
            trust_ca_file: None,
        }),
        Provider::MsSql => DatabaseConnection::MsSql(MsSqlConnection {
            options: common,
            trust_server_certificate: options.mssql_trust_server_certificate,
            trust_ca_file: None,
            dialect_level: None,
        }),
    }
}

fn database_credentials(provider: Provider) -> Result<open_sdbl_db::Credentials, String> {
    let secret = |name| match std::env::var(name) {
        Ok(secret) => Ok(Some(secret)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(format!("переменная {name} содержит не-UTF-8 пароль"))
        }
    };
    match provider {
        Provider::Postgres => Ok(open_sdbl_db::Credentials::new(
            secret("PGPASSWORD")?,
            None,
            None,
        )),
        Provider::MsSql => Ok(open_sdbl_db::Credentials::new(
            None,
            secret("MSSQL_PASSWORD")?,
            None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(rest: &[&str]) -> Vec<String> {
        std::iter::once("bsl-cli")
            .chain(rest.iter().copied())
            .map(str::to_string)
            .collect()
    }

    fn required(provider: &str) -> Vec<&str> {
        vec![
            "--config-db",
            provider,
            "--db-host",
            "db",
            "--db-name",
            "base",
            "--db-user",
            "reader",
            "--platform-version",
            "8.3.27.2342",
        ]
    }

    #[test]
    fn ordinary_invocation_is_untouched() {
        let mut actual = args(&["script.bsl", "argument"]);
        let original = actual.clone();
        assert_eq!(take(&mut actual).unwrap(), None);
        assert_eq!(actual, original);

        let mut with_option_shaped_arguments = args(&[
            "script.bsl",
            "--config-db",
            "--principal=Другой",
            "--db-password=значение",
        ]);
        let original = with_option_shaped_arguments.clone();
        assert_eq!(take(&mut with_option_shaped_arguments).unwrap(), None);
        assert_eq!(with_option_shaped_arguments, original);
    }

    #[test]
    fn session_language_options_are_only_accepted_with_database_mode() {
        let mut ordinary = args(&["--locale-code", "ru_RU", "script.bsl"]);
        assert!(take(&mut ordinary).is_err());
        let mut ordinary = args(&["--system-language", "ru", "script.bsl"]);
        assert!(take(&mut ordinary).is_err());
        let mut ordinary = args(&["--infobase-locale-code", "ru_RU", "script.bsl"]);
        assert!(take(&mut ordinary).is_err());

        let mut database = required("postgres");
        database.extend([
            "--locale-code=en_US_POSIX",
            "--infobase-locale-code=ru_RU",
            "--system-language=en",
            "script.bsl",
        ]);
        let mut database = args(&database);
        let options = take(&mut database).unwrap().unwrap();
        assert_eq!(options.locale_code.as_deref(), Some("en_US_POSIX"));
        assert_eq!(options.infobase_locale_code.as_deref(), Some("ru_RU"));
        assert_eq!(options.system_language.as_deref(), Some("en"));
        assert_eq!(database, args(&["script.bsl"]));
    }

    #[test]
    fn parses_both_providers_and_removes_only_its_modifiers() {
        let mut postgres = required("postgres");
        postgres.extend([
            "--db-plaintext",
            "--principal=Тест",
            "script.bsl",
            "--db-password=аргумент",
        ]);
        let mut postgres = args(&postgres);
        let parsed = take(&mut postgres).unwrap().unwrap();
        assert_eq!(parsed.provider, Provider::Postgres);
        assert_eq!(parsed.port, 5432);
        assert!(parsed.postgres_plaintext);
        assert_eq!(parsed.principal.as_deref(), Some("Тест"));
        assert_eq!(postgres, args(&["script.bsl", "--db-password=аргумент"]));
        assert!(matches!(
            database_connection(&parsed),
            open_sdbl_db::DatabaseConnection::Postgres(connection)
                if connection.sslmode == open_sdbl_db::PostgresSslMode::Disable
        ));

        let mut mssql = required("mssql");
        mssql.extend([
            "--db-port=1444",
            "--db-trust-server-certificate",
            "script.bsl",
        ]);
        let mut mssql = args(&mssql);
        let parsed = take(&mut mssql).unwrap().unwrap();
        assert_eq!(parsed.provider, Provider::MsSql);
        assert_eq!(parsed.port, 1444);
        assert!(parsed.mssql_trust_server_certificate);
        assert_eq!(mssql, args(&["script.bsl"]));
        assert!(matches!(
            database_connection(&parsed),
            open_sdbl_db::DatabaseConnection::MsSql(connection)
                if connection.trust_server_certificate && connection.options.port == 1444
        ));
    }

    #[test]
    fn database_transport_is_verified_without_an_explicit_opt_out() {
        let postgres = take(&mut args(&required("postgres"))).unwrap().unwrap();
        assert!(matches!(
            database_connection(&postgres),
            open_sdbl_db::DatabaseConnection::Postgres(connection)
                if connection.sslmode == open_sdbl_db::PostgresSslMode::VerifyFull
        ));
        let mssql = take(&mut args(&required("mssql"))).unwrap().unwrap();
        assert!(matches!(
            database_connection(&mssql),
            open_sdbl_db::DatabaseConnection::MsSql(connection)
                if !connection.trust_server_certificate
        ));
    }

    #[test]
    fn modifiers_require_database_mode_and_required_values() {
        for rest in [
            vec!["--principal", "Тест", "script.bsl"],
            vec!["--db-host", "db", "script.bsl"],
            vec!["--db-plaintext", "script.bsl"],
        ] {
            let mut actual = args(&rest);
            assert!(take(&mut actual).is_err(), "{rest:?}");
        }
        for missing in ["--db-host", "--db-name", "--db-user", "--platform-version"] {
            let mut rest = required("postgres");
            let at = rest.iter().position(|value| *value == missing).unwrap();
            rest.drain(at..=at + 1);
            let error = take(&mut args(&rest)).unwrap_err();
            assert!(error.contains(missing), "{error}");
        }
    }

    #[test]
    fn unsafe_transport_flags_are_provider_specific() {
        let mut postgres = required("postgres");
        postgres.push("--db-trust-server-certificate");
        assert!(take(&mut args(&postgres)).is_err());
        let mut mssql = required("mssql");
        mssql.push("--db-plaintext");
        assert!(take(&mut args(&mssql)).is_err());
    }

    #[test]
    fn password_flags_before_script_are_rejected_without_mutation() {
        for flag in PASSWORD_FLAGS {
            let spelling = format!("{flag}=secret");
            let mut actual = args(&[&spelling, "script.bsl"]);
            let original = actual.clone();
            let error = take(&mut actual).unwrap_err();
            assert!(error.contains("PGPASSWORD"), "{error}");
            assert_eq!(actual, original);
        }
    }

    #[test]
    fn bad_or_duplicate_values_leave_arguments_untouched() {
        for rest in [
            vec!["--config-db", "sqlite"],
            vec!["--config-db", "postgres", "--db-port", "70000"],
            vec!["--config-db", "postgres", "--db-host"],
            vec!["--config-db=postgres", "--config-db=mssql"],
            vec!["--config-db", "postgres", "--db-plaintext=true"],
        ] {
            let mut actual = args(&rest);
            let original = actual.clone();
            assert!(take(&mut actual).is_err(), "{rest:?}");
            assert_eq!(actual, original);
        }
    }
}

#[cfg(test)]
#[path = "config_db_files_tests.rs"]
mod files_tests;
