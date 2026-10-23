//! Граница доступа к данным: пользователь сеанса, запросы к адаптеру,
//! сам адаптер и данные сеанса, через которые его получают функции крейта.

use super::*;

/// Замороженное описание пользователя текущего сеанса.
#[derive(Debug, Clone)]
pub struct Principal {
    id: Arc<str>,
    name: Arc<str>,
    full_name: Arc<str>,
    role_ids: Arc<[Arc<str>]>,
    interface_language_id: Option<Arc<str>>,
    pub(crate) data_separation: Option<Arc<SerializedValueGraph>>,
}

impl Principal {
    /// Создаёт снимок пользователя и назначенных ему ролей.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        role_ids: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            full_name: Arc::from(""),
            role_ids: role_ids
                .into_iter()
                .map(Arc::<str>::from)
                .collect::<Vec<_>>()
                .into(),
            interface_language_id: None,
            data_separation: None,
        }
    }

    /// Сохраняет полное имя пользователя на момент открытия сеанса.
    #[must_use]
    pub fn with_full_name(mut self, full_name: impl Into<String>) -> Self {
        self.full_name = Arc::from(full_name.into());
        self
    }

    /// Сохраняет выбранный для пользователя язык интерфейса.
    #[must_use]
    pub fn with_interface_language_id(mut self, id: impl Into<String>) -> Self {
        self.interface_language_id = Some(Arc::from(id.into()));
        self
    }

    /// Сохраняет приобретённый снимок структуры разделения данных пользователя.
    ///
    /// Один корень типа `Структура` проверяется при чтении BSL-свойства.
    /// Отсутствие снимка отличается от известной пустой структуры.
    #[must_use]
    pub fn with_data_separation(mut self, snapshot: SerializedValueGraph) -> Self {
        self.data_separation = Some(Arc::new(snapshot));
        self
    }

    #[must_use]
    /// Возвращает идентификатор пользователя информационной базы.
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    /// Возвращает имя пользователя информационной базы.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Возвращает полное имя пользователя, отдельное от краткого имени.
    #[must_use]
    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    /// Перечисляет идентификаторы назначенных ролей.
    pub fn role_ids(&self) -> impl Iterator<Item = &str> {
        self.role_ids.iter().map(AsRef::as_ref)
    }

    /// Возвращает идентификатор выбранного языка интерфейса.
    #[must_use]
    pub fn interface_language_id(&self) -> Option<&str> {
        self.interface_language_id.as_deref()
    }

    pub(crate) fn has_role(&self, id: &str) -> bool {
        self.role_ids
            .iter()
            .any(|candidate| candidate.as_ref() == id)
    }
}

/// Неизменяемый запрос к адаптеру данных текущего сеанса.
#[derive(Debug, Clone)]
pub struct QueryRequest {
    pub(crate) text: Rc<str>,
    pub(crate) parameters: Rc<[(Rc<str>, Value)]>,
    pub(crate) temp_tables_manager: Option<TempTablesManagerHandle>,
}

/// Поиск собственного реквизита справочника или документа в данных текущего сеанса.
#[derive(Debug, Clone)]
pub struct FindByAttributeRequest {
    pub(crate) attribute: Rc<str>,
    pub(crate) value: Value,
    pub(crate) parent: Option<Value>,
    pub(crate) owner: Option<Value>,
}

impl FindByAttributeRequest {
    /// Приобретённое имя собственного реквизита.
    #[must_use]
    pub fn attribute(&self) -> &str {
        &self.attribute
    }
    /// Значение поиска без приведения к другому типу.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }
    /// Явно указанный родитель; отсутствие означает поиск без этого отбора.
    #[must_use]
    pub fn parent(&self) -> Option<&Value> {
        self.parent.as_ref()
    }
    /// Явно указанный владелец; отсутствие означает поиск без этого отбора.
    #[must_use]
    pub fn owner(&self) -> Option<&Value> {
        self.owner.as_ref()
    }
}

/// Запрос поиска ссылок в данных текущего сеанса.
#[derive(Debug, Clone)]
pub struct FindByRefRequest {
    pub(crate) references: Value,
    pub(crate) search_area: Option<Value>,
    pub(crate) include_objects: Option<Value>,
    pub(crate) exclude_objects: Option<Value>,
}

impl FindByRefRequest {
    /// Возвращает массив искомых ссылок.
    #[must_use]
    pub const fn references(&self) -> &Value {
        &self.references
    }

    /// Возвращает явно заданную область поиска.
    #[must_use]
    pub const fn search_area(&self) -> Option<&Value> {
        self.search_area.as_ref()
    }

    /// Возвращает явно включённые объекты метаданных.
    #[must_use]
    pub const fn include_objects(&self) -> Option<&Value> {
        self.include_objects.as_ref()
    }

    /// Возвращает явно исключённые объекты метаданных.
    #[must_use]
    pub const fn exclude_objects(&self) -> Option<&Value> {
        self.exclude_objects.as_ref()
    }
}

/// Непрозрачный идентификатор менеджера временных таблиц у адаптера данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempTablesManagerHandle(u64);

impl TempTablesManagerHandle {
    /// Создаёт идентификатор из значения, уникального в пределах соединения.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Возвращает значение для сопоставления с ресурсом адаптера.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Неизменяемый запрос значения функциональной опции.
#[derive(Debug, Clone)]
pub struct FunctionalOptionRequest {
    pub(crate) name: Rc<str>,
    pub(crate) parameters: Rc<[(Rc<str>, Value)]>,
}

/// Именованное пространство управляемой блокировки данных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLockRequest {
    pub(crate) lock_space: Rc<str>,
}

/// Чей набор ролей проверяет [`DataAccess::access_right`].
#[derive(Debug, Clone)]
pub enum AccessRightSubject {
    /// Пользователь текущего сеанса.
    CurrentPrincipal,
    /// Явно переданный пользователь информационной базы.
    Principal(Principal),
    /// Явно переданная роль конфигурации.
    Role(Arc<str>),
}

/// Неизменяемый запрос проверки права на объект метаданных.
#[derive(Debug, Clone)]
pub struct AccessRightRequest {
    pub(crate) right: Rc<str>,
    pub(crate) metadata: MetadataObject,
    pub(crate) metadata_value: Value,
    pub(crate) attribute: Option<DocumentAttribute>,
    pub(crate) subject: AccessRightSubject,
    pub(crate) standard_attribute: Option<Rc<str>>,
}

/// Снимок политики паролей пользователей, прочитанный текущим сеансом.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPasswordPolicySnapshot {
    minimum_password_length: u32,
}

/// Полный снимок настроек блокировки аутентификации пользователей базы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticationLockSettingsSnapshot {
    lock_duration: BslNumber,
    max_unsuccessful_attempts_count: BslNumber,
    user_name_addition_codes: Arc<str>,
}

/// Числовая настройка политики паролей информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserPasswordNumericSetting {
    /// Максимальный срок действия пароля в секундах.
    MaxEffectivePeriod,
    /// Минимальный срок действия пароля в секундах.
    MinEffectivePeriod,
    /// Число последних паролей, которые нельзя повторять.
    ReuseLimit,
    /// Срок предупреждения об истечении пароля в секундах.
    ExpirationNotificationPeriod,
}

/// Настройка политики паролей, изменяемая через адаптер информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserPasswordSetting {
    /// Проверка сложности паролей.
    StrengthCheck,
    /// Минимальная длина пароля.
    MinimumLength,
    /// Числовая настройка срока или повторения паролей.
    Numeric(UserPasswordNumericSetting),
}

impl AuthenticationLockSettingsSnapshot {
    /// Создаёт снимок из двух числовых настроек и строки кодов дополнения.
    #[must_use]
    pub fn new(
        lock_duration: BslNumber,
        max_unsuccessful_attempts_count: BslNumber,
        user_name_addition_codes: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            lock_duration,
            max_unsuccessful_attempts_count,
            user_name_addition_codes: user_name_addition_codes.into(),
        }
    }

    /// Возвращает длительность блокировки в секундах.
    #[must_use]
    pub const fn lock_duration(&self) -> &BslNumber {
        &self.lock_duration
    }

    /// Возвращает предельное число неуспешных попыток.
    #[must_use]
    pub const fn max_unsuccessful_attempts_count(&self) -> &BslNumber {
        &self.max_unsuccessful_attempts_count
    }

    /// Возвращает строку кодов дополнения имени пользователя.
    #[must_use]
    pub fn user_name_addition_codes(&self) -> &str {
        &self.user_name_addition_codes
    }
}

impl UserPasswordPolicySnapshot {
    /// Создаёт снимок с минимальной длиной пароля.
    #[must_use]
    pub const fn new(minimum_password_length: u32) -> Self {
        Self {
            minimum_password_length,
        }
    }

    /// Возвращает минимальную длину пароля.
    #[must_use]
    pub const fn minimum_password_length(&self) -> u32 {
        self.minimum_password_length
    }
}

impl AccessRightRequest {
    /// Возвращает имя проверяемого права.
    #[must_use]
    pub fn right(&self) -> &str {
        &self.right
    }

    /// Возвращает объект метаданных, владельца реквизита или корень конфигурации.
    #[must_use]
    pub const fn metadata(&self) -> &MetadataObject {
        &self.metadata
    }

    /// Возвращает реквизит документа; `metadata()` при этом содержит его владельца.
    #[must_use]
    pub const fn attribute(&self) -> Option<&DocumentAttribute> {
        self.attribute.as_ref()
    }

    /// Возвращает исходное BSL-значение проверенных метаданных.
    ///
    /// Значение принадлежит каталогу текущего сеанса и сохраняется без
    /// повторного создания объекта, в том числе для записи в журнал.
    #[must_use]
    pub const fn metadata_value(&self) -> &Value {
        &self.metadata_value
    }

    /// Возвращает явно выбранный субъект либо текущего пользователя.
    #[must_use]
    pub const fn subject(&self) -> &AccessRightSubject {
        &self.subject
    }

    /// Возвращает путь стандартного реквизита или табличной части.
    #[must_use]
    pub fn standard_attribute(&self) -> Option<&str> {
        self.standard_attribute.as_deref()
    }
}

impl DataLockRequest {
    /// Возвращает имя пространства блокировок в синтаксисе платформы.
    #[must_use]
    pub fn lock_space(&self) -> &str {
        &self.lock_space
    }
}

impl QueryRequest {
    /// Возвращает текст запроса BSL.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Перечисляет окончательные значения параметров в порядке их первой
    /// установки. Написание ключей соответствует текущей структуре параметров.
    pub fn parameters(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.parameters
            .iter()
            .map(|(name, value)| (name.as_ref(), value))
    }

    /// Возвращает менеджер временных таблиц, назначенный запросу.
    #[must_use]
    pub const fn temp_tables_manager(&self) -> Option<TempTablesManagerHandle> {
        self.temp_tables_manager
    }
}

impl FunctionalOptionRequest {
    /// Возвращает имя функциональной опции в языке текущей конфигурации.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Перечисляет переданные параметры в порядке полей исходной структуры.
    pub fn parameters(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.parameters
            .iter()
            .map(|(name, value)| (name.as_ref(), value))
    }
}

/// Именованные строки, которые адаптер данных вернул для одного запроса.
#[derive(Debug, Clone)]
pub struct QueryResultData {
    pub(crate) columns: Rc<[Rc<str>]>,
    pub(crate) rows: Rc<[Rc<[Value]>]>,
}

impl QueryResultData {
    /// Проверяет и создаёт линейный результат запроса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого или повторного имени колонки и строки,
    /// ширина которой не совпадает с числом колонок.
    pub fn new(
        columns: impl IntoIterator<Item = String>,
        rows: impl IntoIterator<Item = Vec<Value>>,
    ) -> Result<Self, QueryResultDataError> {
        let columns = columns.into_iter().map(Rc::<str>::from).collect::<Vec<_>>();
        for (index, name) in columns.iter().enumerate() {
            if name.is_empty() {
                return Err(QueryResultDataError::EmptyColumnName { index });
            }
            if columns[..index]
                .iter()
                .any(|candidate| folded_eq(candidate, name))
            {
                return Err(QueryResultDataError::DuplicateColumnName {
                    name: name.to_string(),
                });
            }
        }
        let rows = rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                if row.len() != columns.len() {
                    return Err(QueryResultDataError::RowWidth {
                        row: index,
                        expected: columns.len(),
                        actual: row.len(),
                    });
                }
                Ok(Rc::<[Value]>::from(row))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            columns: columns.into(),
            rows: rows.into(),
        })
    }

    /// Возвращает имена колонок в порядке результата.
    #[must_use]
    pub fn columns(&self) -> &[Rc<str>] {
        &self.columns
    }

    /// Возвращает строки в порядке результата.
    #[must_use]
    pub fn rows(&self) -> &[Rc<[Value]>] {
        &self.rows
    }
}

/// Ошибка формы результата, возвращённого адаптером данных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResultDataError {
    /// Имя колонки пусто.
    EmptyColumnName {
        /// Номер колонки.
        index: usize,
    },
    /// Имя колонки повторяется без учёта регистра.
    DuplicateColumnName {
        /// Повторившееся имя.
        name: String,
    },
    /// Число значений строки не совпадает с числом колонок.
    RowWidth {
        /// Номер строки.
        row: usize,
        /// Ожидаемое число значений.
        expected: usize,
        /// Фактическое число значений.
        actual: usize,
    },
}

impl fmt::Display for QueryResultDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyColumnName { index } => write!(formatter, "имя колонки {index} пусто"),
            Self::DuplicateColumnName { name } => {
                write!(formatter, "имя колонки `{name}` повторяется")
            }
            Self::RowWidth {
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "строка {row} содержит {actual} значений вместо {expected}"
            ),
        }
    }
}

impl Error for QueryResultDataError {}

/// Доступ к данным конфигурации в пределах одного сеанса.
///
/// Реализация принадлежит адаптеру источника данных. `bsl-orm` передаёт ей
/// метаданные объекта, неизменяемого пользователя сеанса и текущий контекст
/// вызова, через который доступны параметры того же сеанса.
pub trait DataAccess {
    /// Выдаёт код справочника по измеренным настройкам серии конфигурации.
    /// Получает префикс после прикладного обработчика и пользователя сеанса.
    /// Адаптер использует уже принадлежащее этому сеансу соединение.
    ///
    /// # Errors
    /// Возвращает ошибку неизвестных настроек, прав, источника данных или
    /// неподдерживаемой выдачи кода. По умолчанию выдача не поддерживается.
    fn allocate_catalog_code(
        &self,
        _metadata: &MetadataObject,
        _principal: &Principal,
        _prefix: &str,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "выдача кода справочника не поддерживается адаптером данных".into(),
        ))
    }

    /// Выдаёт новый номер документа в области нумерации конфигурации.
    /// Получает дату документа и префикс после прикладного обработчика.
    /// Адаптер принадлежит сеансу, но резервирование номера может быть общим
    /// для нескольких сеансов информационной базы.
    ///
    /// # Errors
    /// Возвращает ошибку неизвестных настроек, прав, резервирования или
    /// неподдерживаемого адаптера. По умолчанию генерация не поддерживается.
    fn allocate_document_number(
        &self,
        _metadata: &MetadataObject,
        _principal: &Principal,
        _date: open_bsl::BslDate,
        _prefix: &str,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "выдача номера документа не поддерживается адаптером данных".into(),
        ))
    }

    /// Читает значение константы с учётом прав текущего пользователя.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, проверки прав, RLS или
    /// инициализации требуемого параметра сеанса.
    fn constant_value(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value>;

    /// Записывает константу от имени текущего пользователя.
    /// Адаптер отвечает за приведение типа, права, обработчики и сохранение
    /// в транзакции своего соединения. По умолчанию запись недоступна.
    ///
    /// # Errors
    /// Возвращает ошибку отсутствующей возможности записи, проверки прав,
    /// преобразования значения, обработчика или источника данных.
    fn set_constant_value(
        &self,
        _metadata: &MetadataObject,
        _value: &Value,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "запись константы не поддерживается адаптером данных".into(),
        ))
    }

    /// Ищет доступную ссылку по собственному реквизиту справочника или документа.
    /// `None` означает успешный поиск без совпадения, а не ошибку чтения.
    ///
    /// # Errors
    /// Возвращает ошибку неподдерживаемого поиска, метаданных, прав, RLS
    /// либо источника данных. По умолчанию поиск явно не поддерживается.
    fn find_by_attribute(
        &self,
        _metadata: &MetadataObject,
        _request: &FindByAttributeRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        Err(RtError::DynamicError(
            "поиск по реквизиту не поддерживается адаптером данных".into(),
        ))
    }

    /// Выполняет запрос с учётом прав и RLS текущего пользователя.
    ///
    /// Реализация получает текущий [`CallContext`], чтобы чтение требуемого
    /// параметра сеанса прошло через его обычную ленивую инициализацию.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, проверки прав, RLS, компиляции
    /// запроса или инициализации параметра сеанса. Реализация по умолчанию
    /// явно отвергает запросы.
    fn execute_query(
        &self,
        _request: &QueryRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        Err(RtError::DynamicError(
            "выполнение запросов не поддерживается адаптером данных".into(),
        ))
    }

    /// Выполняет запрос, допускающий отсутствие результата после уничтожения
    /// временной таблицы. Прежние адаптеры автоматически возвращают результат.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку адаптера данных, проверки прав или компиляции запроса.
    fn execute_query_result(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<QueryResultData>> {
        self.execute_query(request, principal, context).map(Some)
    }

    /// Ищет ссылки в доступных текущему пользователю данных.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, разделения данных или
    /// неподдерживаемой операции.
    fn find_by_ref(
        &self,
        _request: &FindByRefRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        Err(RtError::DynamicError(
            "поиск по ссылкам не поддерживается адаптером данных".into(),
        ))
    }

    /// Возвращает значение функциональной опции для текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестной опции, неполного набора параметров,
    /// источника данных, проверки прав или неподдерживаемой операции.
    fn functional_option(
        &self,
        _request: &FunctionalOptionRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "функциональные опции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Строит навигационную ссылку для прикладного значения текущего сеанса.
    ///
    /// `arguments` сохраняет позиции перегруженного платформенного вызова,
    /// включая `Неопределено` на месте пропущенного аргумента.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неподдерживаемого вида ссылки, источника данных,
    /// прав, внешнего хранилища навигационных данных или самого значения.
    fn navigation_link(
        &self,
        _arguments: &[Value],
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<String> {
        Err(RtError::DynamicError(
            "навигационные ссылки не поддерживаются адаптером данных".into(),
        ))
    }

    /// Ищет политику паролей пользователей по имени в текущей информационной базе.
    ///
    /// `None` в `name` означает вызов платформенного метода без необязательного
    /// аргумента. `None` в результате означает, что политика не найдена.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_policy(
        &self,
        _name: Option<&str>,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Option<UserPasswordPolicySnapshot>> {
        Err(RtError::DynamicError(
            "политики паролей пользователей не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает минимальную длину паролей пользователей информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_minimum_length(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<u32> {
        Err(RtError::DynamicError(
            "минимальная длина паролей пользователей не поддерживается адаптером данных".into(),
        ))
    }

    /// Проверяет, включена ли проверка сложности паролей пользователей базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_strength_check(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "проверка сложности паролей пользователей не поддерживается адаптером данных".into(),
        ))
    }

    /// Читает числовую настройку политики паролей пользователей базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn user_password_numeric_setting(
        &self,
        _setting: UserPasswordNumericSetting,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslNumber> {
        Err(RtError::DynamicError(
            "числовые настройки паролей пользователей не поддерживаются адаптером данных".into(),
        ))
    }

    /// Изменяет настройку политики паролей пользователей базы.
    ///
    /// Значение передаётся без преобразования: его проверка и сохранение
    /// принадлежат конкретному адаптеру информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав, проверки
    /// значения или неподдерживаемой операции.
    fn set_user_password_setting(
        &self,
        _setting: UserPasswordSetting,
        _value: &Value,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "изменение настроек паролей пользователей не поддерживается адаптером данных".into(),
        ))
    }

    /// Читает полный снимок настроек блокировки аутентификации пользователей.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав или
    /// неподдерживаемой операции.
    fn authentication_lock_settings(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<AuthenticationLockSettingsSnapshot> {
        Err(RtError::DynamicError(
            "настройки блокировки аутентификации не поддерживаются адаптером данных".into(),
        ))
    }

    /// Записывает полный снимок настроек блокировки аутентификации пользователей.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав, проверки
    /// значений или неподдерживаемой операции.
    fn set_authentication_lock_settings(
        &self,
        _settings: &AuthenticationLockSettingsSnapshot,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "изменение настроек блокировки аутентификации не поддерживается адаптером данных"
                .into(),
        ))
    }

    /// Создаёт менеджер временных таблиц на соединении текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, соединения или неподдерживаемой
    /// операции.
    fn create_temp_tables_manager(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<TempTablesManagerHandle> {
        Err(RtError::DynamicError(
            "временные таблицы не поддерживаются адаптером данных".into(),
        ))
    }

    /// Закрывает менеджер и удаляет принадлежащие ему временные таблицы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного идентификатора, соединения, удаления
    /// таблиц или неподдерживаемой операции.
    fn close_temp_tables_manager(
        &self,
        _manager: TempTablesManagerHandle,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "временные таблицы не поддерживаются адаптером данных".into(),
        ))
    }

    /// Проверяет эффективное право на объект метаданных.
    ///
    /// Адаптер применяет иерархию прав ролей и самостоятельно проверяет
    /// административное право текущего пользователя для явно указанного
    /// пользователя. Проверка корня конфигурации делегируется адаптеру и
    /// в привилегированном режиме: право и явный субъект требуют проверки.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку отсутствующих данных ролей, неизвестного права,
    /// неприменимого объекта либо недостаточного административного права.
    fn access_right(
        &self,
        _request: &AccessRightRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "проверка прав метаданных не поддерживается адаптером данных".into(),
        ))
    }

    /// Проверяет право текущего пользователя с записью отказа в журнал.
    ///
    /// Адаптер отвечает за проверку права и событие `ОтказВДоступе` при
    /// отказе: без обеих операций он должен вернуть ошибку.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при отказе в доступе, недоступном журнале или
    /// отсутствии у адаптера такой операции.
    fn verify_access_rights(
        &self,
        _request: &AccessRightRequest,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "проверка прав с записью отказа не поддерживается адаптером данных".into(),
        ))
    }

    /// Устанавливает управляемые блокировки на соединении текущего сеанса.
    /// Адаптер отвечает за транзакцию, время жизни и снятие блокировок.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, проверки прав, транзакции или
    /// неподдерживаемого механизма блокировок.
    fn lock_data(
        &self,
        _requests: &[DataLockRequest],
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "управляемые блокировки данных не поддерживаются адаптером данных".into(),
        ))
    }

    /// Начинает транзакцию на соединении текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных или неподдерживаемой транзакции.
    fn begin_transaction(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Фиксирует текущую транзакцию соединения сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, состояния или фиксации транзакции.
    fn commit_transaction(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Отменяет текущую транзакцию соединения сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, состояния или отката транзакции.
    fn rollback_transaction(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает состояние транзакции на соединении текущего сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных или неподдерживаемой транзакции.
    fn transaction_active(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "транзакции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает состояние монопольного режима текущего соединения.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных или неподдерживаемой
    /// операции.
    fn exclusive_mode(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        Err(RtError::DynamicError(
            "монопольный режим не поддерживается адаптером данных".into(),
        ))
    }

    /// Изменяет монопольный режим текущего соединения.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, конкурирующего сеанса
    /// или неподдерживаемой операции.
    fn set_exclusive_mode(
        &self,
        _enabled: bool,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "монопольный режим не поддерживается адаптером данных".into(),
        ))
    }

    /// Запускает или останавливает обработку сервисов интеграции.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, разделителей сеанса или
    /// неподдерживаемой операции.
    fn set_integration_services_processing(
        &self,
        _enabled: bool,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "обработка сервисов интеграции не поддерживается адаптером данных".into(),
        ))
    }

    /// Возвращает менеджер именованного сервиса интеграции.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку неизвестного сервиса, источника данных, прав или
    /// неподдерживаемой операции.
    fn integration_service(
        &self,
        _name: &str,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "сервисы интеграции не поддерживаются адаптером данных".into(),
        ))
    }

    /// Возвращает режим обновления предопределённых данных информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав или неподдерживаемой операции.
    fn info_base_predefined_data_update(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<PredefinedDataUpdateMode> {
        Err(RtError::DynamicError(
            "режим обновления предопределённых данных не поддерживается адаптером данных".into(),
        ))
    }

    /// Изменяет режим обновления предопределённых данных информационной базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, административных прав,
    /// разделителей сеанса или неподдерживаемой операции.
    fn set_info_base_predefined_data_update(
        &self,
        _mode: PredefinedDataUpdateMode,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "режим обновления предопределённых данных не поддерживается адаптером данных".into(),
        ))
    }

    /// Инициализирует предопределённые данные текущей области базы.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку источника данных, прав, разделителей сеанса или
    /// неподдерживаемой операции.
    fn initialize_predefined_data(
        &self,
        _principal: &Principal,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        Err(RtError::DynamicError(
            "инициализация предопределённых данных не поддерживается адаптером данных".into(),
        ))
    }
}

/// Данные ORM, принадлежащие ровно одному `open_bsl::State`.
pub struct SessionData {
    pub(crate) catalog: Arc<MetadataCatalog>,
    pub(crate) principal: Principal,
    pub(crate) data_access: Option<Rc<dyn DataAccess>>,
    extension: Option<Rc<dyn Any>>,
    pub(crate) reference_uuids: RefCell<reference_uuid::ReferenceUuids>,
    system_language: Option<Arc<str>>,
}

impl SessionData {
    /// Создаёт неизменяемые данные ORM одного сеанса.
    #[must_use]
    pub fn new(catalog: Arc<MetadataCatalog>, principal: Principal) -> Self {
        Self {
            catalog,
            principal,
            data_access: None,
            extension: None,
            reference_uuids: RefCell::new(reference_uuid::ReferenceUuids::default()),
            system_language: None,
        }
    }

    /// Сохраняет язык интерфейса этого сеанса отдельно от общего каталога.
    #[must_use]
    pub fn with_system_language(mut self, language: impl Into<Arc<str>>) -> Self {
        self.system_language = Some(language.into());
        self
    }

    /// Возвращает явно переданный язык интерфейса сеанса.
    #[must_use]
    pub fn system_language(&self) -> Option<&str> {
        self.system_language.as_deref()
    }

    /// Подключает адаптер данных, принадлежащий этому сеансу.
    #[must_use]
    pub fn with_data_access(mut self, data_access: Rc<dyn DataAccess>) -> Self {
        self.data_access = Some(data_access);
        self
    }

    /// Добавляет сеансовые данные верхнего слоя, например контейнер
    /// параметров сеанса. Значение принадлежит тому же `State`.
    #[must_use]
    pub fn with_extension(mut self, extension: Rc<dyn Any>) -> Self {
        self.extension = Some(extension);
        self
    }

    #[must_use]
    /// Возвращает каталог конфигурации.
    pub fn catalog(&self) -> &Arc<MetadataCatalog> {
        &self.catalog
    }

    #[must_use]
    /// Возвращает снимок пользователя сеанса.
    pub const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Возвращает расширение указанного типа, если верхний слой его передал.
    #[must_use]
    pub fn extension<T: Any>(&self) -> Option<&T> {
        self.extension.as_deref()?.downcast_ref()
    }
}

impl fmt::Debug for SessionData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionData")
            .field("catalog", &self.catalog)
            .field("principal", &self.principal)
            .field("has_data_access", &self.data_access.is_some())
            .field("has_extension", &self.extension.is_some())
            .finish()
    }
}

pub(crate) fn session<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionData> {
    ctx.host_data()?
        .downcast_ref::<SessionData>()
        .ok_or_else(|| RtError::DynamicError("данные сеанса имеют неверный тип".into()))
}

pub(crate) fn current_data_access(
    context: &CallContext<'_>,
) -> RtResult<(Principal, Rc<dyn DataAccess>)> {
    let session = session(context)?;
    let data_access = session.data_access.as_ref().ok_or_else(|| {
        RtError::DynamicError("адаптер данных текущего сеанса не подключён".into())
    })?;
    Ok((session.principal.clone(), Rc::clone(data_access)))
}
