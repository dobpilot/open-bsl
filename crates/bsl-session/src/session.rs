//! Открытый сеанс и ошибки его создания и исполнения.

use super::*;

/// Один запущенный и изолированный сеанс конфигурации.
pub struct Session {
    pub(crate) engine: Engine,
    pub(crate) state: State,
    pub(crate) parameters: Rc<SessionParameters>,
    pub(crate) principal: Principal,
    pub(crate) connection: Box<dyn std::any::Any>,
    pub(crate) kind: SessionKind,
}

impl Session {
    /// Компилирует и исполняет верхний уровень тестового BSL-модуля в общем
    /// каталоге и состоянии этого сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку компиляции, связывания или исполнения.
    pub fn execute(&mut self, source: &str) -> Result<(), SessionError> {
        let module = self.engine.compile_entry(source)?;
        self.state.instantiate(&module)?;
        Ok(())
    }

    /// Компилирует и создаёт вызываемый экземпляр тестового модуля в этом
    /// сеансе.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку компиляции, связывания или инициализации.
    pub fn instantiate(&mut self, source: &str) -> Result<ModuleInstance, SessionError> {
        let module = self.engine.compile_entry(source)?;
        Ok(self.state.instantiate(&module)?)
    }

    /// Вызывает экспорт созданного через [`Session::instantiate`] модуля.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку чужого экземпляра, поиска, арности или исполнения.
    pub fn call(
        &mut self,
        module: &ModuleInstance,
        name: &str,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), SessionError> {
        Ok(module.call(&mut self.state, name, args)?)
    }

    /// Возвращает параметры этого сеанса.
    #[must_use]
    pub fn parameters(&self) -> &Rc<SessionParameters> {
        &self.parameters
    }

    /// Возвращает замороженный снимок пользователя.
    #[must_use]
    pub const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Возвращает вид сеанса.
    #[must_use]
    pub const fn kind(&self) -> SessionKind {
        self.kind
    }

    /// Возвращает ссылку на принадлежащее сеансу соединение указанного типа.
    #[must_use]
    pub fn connection<T: std::any::Any>(&self) -> Option<&T> {
        self.connection.downcast_ref()
    }
}

/// Ошибка построения или запуска конфигурационного сеанса.
#[derive(Debug)]
pub enum SessionError {
    /// Версия платформы не задана.
    EmptyPlatformVersion,
    /// Строка регистрации информационной базы пуста.
    EmptyConnectionString,
    /// Номер соединения не является положительным числом, представимым в BSL.
    InvalidConnectionNumber(u64),
    /// Номер сеанса не является положительным числом, представимым в BSL.
    InvalidSessionNumber(u64),
    /// Вариант интерфейса не принадлежит соответствующему перечислению.
    InvalidClientInterfaceVariant,
    /// Номер соединения из снимка не представим в BSL.
    InvalidListedConnectionNumber(u64),
    /// В снимке отсутствует соединение создаваемого сеанса.
    MissingCurrentConnection(u64),
    /// Код и метаданные относятся к разным конфигурациям.
    ConfigurationIdentityMismatch {
        /// Идентичность снимка кода.
        code: String,
        /// Идентичность каталога метаданных.
        metadata: String,
    },
    /// Модуль менеджера не называет владельца.
    ManagerWithoutOwner {
        /// Имя модуля менеджера.
        module: String,
    },
    /// В метаданных нет владельца модуля менеджера.
    MissingManagerOwner {
        /// Имя модуля менеджера.
        module: String,
        /// Идентификатор владельца.
        owner: String,
    },
    /// В снимке нет выбранного модуля сеанса.
    MissingSessionModule(String),
    /// Импорт ссылается на отсутствующий модуль.
    MissingImportedModule {
        /// Имя импортирующего модуля.
        module: String,
        /// Идентификатор отсутствующего модуля.
        imported: String,
    },
    /// Фабрика собрана не для фонового задания.
    NotBackgroundFactory,
    /// Имя метода фонового задания пусто.
    EmptyBackgroundMethod,
    /// Ошибка объявления параметров.
    Parameters(ParameterError),
    /// Ошибка open-bsl.
    OpenBsl(OpenBslError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlatformVersion => formatter.write_str("версия платформы пуста"),
            Self::EmptyConnectionString => {
                formatter.write_str("строка соединения с информационной базой пуста")
            }
            Self::InvalidConnectionNumber(number) => write!(
                formatter,
                "номер соединения с информационной базой недопустим: {number}"
            ),
            Self::InvalidSessionNumber(number) => {
                write!(
                    formatter,
                    "номер сеанса информационной базы недопустим: {number}"
                )
            }
            Self::InvalidClientInterfaceVariant => {
                formatter.write_str("недопустимый вариант интерфейса клиентского приложения")
            }
            Self::InvalidListedConnectionNumber(number) => write!(
                formatter,
                "снимок содержит недопустимый номер соединения: {number}"
            ),
            Self::MissingCurrentConnection(number) => write!(
                formatter,
                "текущее соединение `{number}` отсутствует в снимке соединений"
            ),
            Self::ConfigurationIdentityMismatch { code, metadata } => write!(
                formatter,
                "код конфигурации `{code}` не совпадает с метаданными `{metadata}`"
            ),
            Self::ManagerWithoutOwner { module } => {
                write!(
                    formatter,
                    "модуль менеджера `{module}` не называет владельца"
                )
            }
            Self::MissingManagerOwner { module, owner } => write!(
                formatter,
                "владелец `{owner}` модуля менеджера `{module}` отсутствует в метаданных"
            ),
            Self::MissingSessionModule(module) => {
                write!(formatter, "модуль сеанса `{module}` не найден")
            }
            Self::MissingImportedModule { module, imported } => write!(
                formatter,
                "модуль `{module}` импортирует отсутствующий модуль `{imported}`"
            ),
            Self::NotBackgroundFactory => {
                formatter.write_str("фабрика не предназначена для фонового задания")
            }
            Self::EmptyBackgroundMethod => formatter.write_str("имя метода фонового задания пусто"),
            Self::Parameters(error) => error.fmt(formatter),
            Self::OpenBsl(error) => error.fmt(formatter),
        }
    }
}

impl Error for SessionError {}

impl From<ParameterError> for SessionError {
    fn from(error: ParameterError) -> Self {
        Self::Parameters(error)
    }
}

impl From<OpenBslError> for SessionError {
    fn from(error: OpenBslError) -> Self {
        Self::OpenBsl(error)
    }
}
