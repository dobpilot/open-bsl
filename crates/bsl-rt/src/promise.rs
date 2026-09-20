//! Непрозрачное BSL-значение `Обещание`.
//!
//! Само состояние ожидания принадлежит одному запуску VM. Значение несёт
//! только пару идентификаторов и не переносит в базовый runtime будущие,
//! каналы или типы конкретного исполнителя host-операций.

use crate::{ObjectProtocol, TypeDescriptor};

/// Построение локального BSL-значения из перенесённого временного файла.
///
/// Функция исполняется только в потоке VM: она может создать `Rc`-объект
/// компонента и передать право очистки в локальный реестр.
pub type TemporaryFileValueMapper = fn(
    crate::OpenedTemporaryFile,
    &crate::TemporaryFileRegistry,
) -> crate::RtResult<crate::BslValue>;

/// Операция оповещения на потоке VM. Только нейтральный запрос, не BSL-значения,
/// может быть передан исполнителем в файловый поток.
pub enum FileNotificationOperation {
    Ready(crate::RtResult<crate::BslValue>),
    Pending {
        request: crate::RtResult<crate::FileOperationRequest>,
        files: std::rc::Rc<dyn crate::FileSystem>,
        zone: std::rc::Rc<dyn crate::TimeZone>,
    },
}

/// Регистрация внешней операции в таблице обещаний исполнения.
/// Каналы и потоки остаются у исполнителя; компонент передаёт только запрос.
pub trait HostPromiseSpawner {
    /// Регистрирует отложенное оповещение в текущем исполнении.
    /// Отказ по умолчанию не начинает операцию и не вызывает обработчиков.
    ///
    /// # Errors
    /// Отсутствие поддержки оповещений либо ошибка регистрации до начала I/O.
    fn begin_file_operation(
        &mut self,
        _operation: FileNotificationOperation,
        _description: crate::NotificationDescription,
        _with_result: bool,
    ) -> crate::RtResult<()> {
        Err(crate::RtError::IoError(
            "исполнитель не предоставляет файловые оповещения".into(),
        ))
    }

    /// Создаёт уже завершённое файловое обещание без переноса значения в поток.
    ///
    /// # Errors
    /// Отсутствие файловой возможности или ошибка регистрации обещания.
    fn ready_file_promise(
        &mut self,
        _result: crate::RtResult<crate::BslValue>,
    ) -> crate::RtResult<crate::BslValue> {
        Err(crate::RtError::IoError(
            "исполнитель не предоставляет файловые обещания".into(),
        ))
    }

    /// Запускает HTTP-операцию и возвращает непрозрачное BSL-обещание.
    ///
    /// # Errors
    /// Ошибка создания обещания или синхронный отказ транспорта до принятия запроса.
    fn spawn_http(
        &mut self,
        client: std::sync::Arc<dyn crate::HttpClient>,
        request: crate::HttpWireRequest,
        mapper: crate::HttpResponseMapper,
        error_mapper: crate::HttpErrorMapper,
    ) -> crate::RtResult<crate::BslValue>;

    /// Регистрирует файловый запрос с исходными сервисами объекта.
    /// Ошибка подготовки сохраняется в обещании поддерживающим исполнителем.
    /// Default не обращается к файловой системе и сохраняет HTTP-only host.
    ///
    /// # Errors
    /// Отсутствие файловой возможности либо ошибка создания обещания.
    fn spawn_file_operation(
        &mut self,
        _request: crate::RtResult<crate::FileOperationRequest>,
        _files: std::rc::Rc<dyn crate::FileSystem>,
        _zone: std::rc::Rc<dyn crate::TimeZone>,
    ) -> crate::RtResult<crate::BslValue> {
        Err(crate::RtError::IoError(
            "исполнитель не предоставляет файловые обещания".into(),
        ))
    }

    /// Атомарно создаёт переносимый временный файл в worker и возвращает
    /// обещание локального BSL-значения. Default не начинает I/O.
    ///
    /// # Errors
    /// Отсутствие файловой возможности либо ошибка регистрации обещания.
    fn spawn_temporary_file_operation(
        &mut self,
        _entropy: [u8; 16],
        _files: std::rc::Rc<dyn crate::FileSystem>,
        _registry: crate::TemporaryFileRegistry,
        _mapper: TemporaryFileValueMapper,
    ) -> crate::RtResult<crate::BslValue> {
        Err(crate::RtError::IoError(
            "исполнитель не предоставляет фоновое создание временных файлов".into(),
        ))
    }
}

/// Уникальный идентификатор одного запуска VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionToken(u64);

impl ExecutionToken {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Номер обещания внутри одного запуска.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PromiseId(u64);

impl PromiseId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Непрозрачная ссылка на состояние обещания в таблице `Execution`.
#[derive(Debug)]
pub struct PromiseValue {
    execution_token: ExecutionToken,
    promise_id: PromiseId,
}

impl PromiseValue {
    #[must_use]
    pub const fn new(execution_token: ExecutionToken, promise_id: PromiseId) -> Self {
        Self {
            execution_token,
            promise_id,
        }
    }

    #[must_use]
    pub const fn execution_token(&self) -> ExecutionToken {
        self.execution_token
    }

    #[must_use]
    pub const fn promise_id(&self) -> PromiseId {
        self.promise_id
    }
}

/// Тип намеренно не имеет конструктора, методов и свойств BSL.
pub static PROMISE_TYPE: TypeDescriptor = TypeDescriptor::new(crate::PACKAGE_NAME, "Обещание");

impl ObjectProtocol for PromiseValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PROMISE_TYPE
    }
}
