//! Сеансовый адаптер чтения через компилятор и соединение `open-sdbl`.

mod access_verification;
mod configuration_rights;
mod logical_result;
mod settings;

use std::cell::{Cell as LocalCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use bsl_orm::{
    DataAccess, FindByAttributeRequest, FunctionalOptionLocation, FunctionalOptionRequest,
    MetadataKind, MetadataObject, Principal, QueryRequest, QueryResultData, SessionData,
    TempTablesManagerHandle,
};
use bsl_session::SessionRuntime;
use open_bsl::{
    BslDate, BslNumber, BslObject, BslString, CallContext, RtError, RtResult, TypeId, Value,
};
use open_sdbl::Keyword;
use open_sdbl::metadata::{Guid, MetadataKind as SdblMetadataKind, MetadataSnapshot, ObjectId};
use open_sdbl::query::{
    AccessDecision, Backend, BoundaryKind, ColumnKind, CompileOptions, CompiledQuery,
    MaterializedBatchResult, ParameterBoundary, ParameterBoundaryValue, ParameterColumn,
    ParameterDate, ParameterPointInTime, ParameterReference, ParameterValue, PrepareOptions,
    QueryCompiler, QueryDiagnostic, QueryParameter, RestrictionRequest, SessionParameters,
    TempTablesManager, queryable_fields,
};
use open_sdbl_db::{
    AccessStore, BlockingDatabaseSession, Cell, DatabaseDialect, DbError, RowFlow,
    check_user_field_access, user_decisions,
};

const MAX_PLACEMENT_BYTES: usize = 8 * 1024 * 1024;

/// Доступ к данным одного сеанса через его собственное соединение.
///
/// То же значение соединения удерживает `SessionResources`: уничтожение
/// адаптера отдельно от сеанса не закрывает ещё используемое соединение.
pub struct DatabaseDataAccess {
    connection: Rc<RefCell<BlockingDatabaseSession>>,
    metadata: Arc<MetadataSnapshot>,
    configuration_source: Option<Arc<super::DatabaseConfigurationSource>>,
    access: AccessStore,
    principal_id: String,
    reading_conditions: RefCell<Vec<Guid>>,
    transaction_depth: LocalCell<usize>,
    transaction_cancelled: LocalCell<bool>,
    next_manager: LocalCell<u64>,
    managers: RefCell<HashMap<u64, ManagerEntry>>,
}

struct ManagerEntry {
    // Менеджер принадлежит одному сеансу и хранит строки, захваченные на
    // момент `ПОМЕСТИТЬ`; последующее чтение источника их не меняет.
    state: Option<TempTablesManager>,
    revision: u64,
}

// Разрешение относится только к одному приобретённому источнику опции. Оно не хранится
// в сеансе и не изменяет проверки последующих запросов.
#[derive(Clone, Copy)]
enum ReadAccess {
    CurrentUser,
    PrivilegedSource(ObjectId),
}

enum CaptureError {
    Compile(QueryDiagnostic),
    Runtime(RtError),
}

impl From<QueryDiagnostic> for CaptureError {
    fn from(error: QueryDiagnostic) -> Self {
        Self::Compile(error)
    }
}

impl DatabaseDataAccess {
    /// Связывает приобретённые метаданные и права с соединением сеанса.
    ///
    /// Если выбранного пользователя нет в `AccessStore`, сеанс остаётся
    /// открываемым, но любое чтение данных явно откажет до SQL.
    #[must_use]
    pub fn new(
        connection: Rc<RefCell<BlockingDatabaseSession>>,
        metadata: impl Into<Arc<MetadataSnapshot>>,
        mut access: AccessStore,
        principal: &Principal,
    ) -> Self {
        let user = access.users().and_then(|users| {
            users
                .iter()
                .find(|user| user.data.id.as_str().eq_ignore_ascii_case(principal.id()))
                .cloned()
        });
        access.set_current_user(user);
        Self {
            connection,
            metadata: metadata.into(),
            configuration_source: None,
            access,
            principal_id: principal.id().to_owned(),
            reading_conditions: RefCell::new(Vec::new()),
            transaction_depth: LocalCell::new(0),
            transaction_cancelled: LocalCell::new(false),
            next_manager: LocalCell::new(0),
            managers: RefCell::new(HashMap::new()),
        }
    }

    /// Подключает адресное приобретение метаданных того же источника конфигурации.
    #[must_use]
    pub fn with_configuration_source(
        mut self,
        source: Arc<super::DatabaseConfigurationSource>,
    ) -> Self {
        self.configuration_source = Some(source);
        self
    }

    fn metadata_snapshot(&self) -> RtResult<Arc<MetadataSnapshot>> {
        match &self.configuration_source {
            Some(source) => source.metadata_snapshot(&[]).map_err(error),
            None => Ok(self.metadata.clone()),
        }
    }

    fn object_snapshot(&self, id: &str) -> RtResult<Arc<MetadataSnapshot>> {
        match &self.configuration_source {
            Some(source) => source.metadata_snapshot(&[id]).map_err(error),
            None => Ok(self.metadata.clone()),
        }
    }

    fn check_principal(&self, principal: &Principal) -> RtResult<()> {
        self.check_principal_identity(principal)?;
        if self.transaction_cancelled.get() {
            return Err(error(
                "транзакция отменена; чтение данных невозможно до её завершения",
            ));
        }
        Ok(())
    }

    fn check_principal_identity(&self, principal: &Principal) -> RtResult<()> {
        if principal.id() != self.principal_id {
            return Err(error(
                "пользователь запроса не совпадает с пользователем сеанса",
            ));
        }
        if self.access.current_user().is_none() {
            return Err(error("текущий пользователь не найден в правах базы"));
        }
        Ok(())
    }

    fn manager_snapshot(
        &self,
        handle: Option<TempTablesManagerHandle>,
    ) -> RtResult<(TempTablesManager, Option<u64>, bool)> {
        let Some(handle) = handle else {
            return Ok((TempTablesManager::new(), None, false));
        };
        let managers = self
            .managers
            .try_borrow()
            .map_err(|_| error("менеджер временных таблиц уже используется"))?;
        let entry = managers
            .get(&handle.value())
            .ok_or_else(|| error("менеджер временных таблиц не принадлежит сеансу"))?;
        Ok((
            entry.state.clone().unwrap_or_default(),
            Some(entry.revision),
            entry.state.is_none(),
        ))
    }

    fn check_manager_revision(
        &self,
        handle: Option<TempTablesManagerHandle>,
        revision: Option<u64>,
    ) -> RtResult<()> {
        if let Some(handle) = handle {
            let managers = self
                .managers
                .try_borrow()
                .map_err(|_| error("менеджер временных таблиц уже используется"))?;
            if managers.get(&handle.value()).map(|entry| entry.revision) != revision {
                return Err(error(
                    "менеджер временных таблиц изменился во вложенном вызове",
                ));
            }
        }
        Ok(())
    }

    fn commit_manager(
        &self,
        handle: Option<TempTablesManagerHandle>,
        revision: Option<u64>,
        state: TempTablesManager,
    ) -> RtResult<()> {
        let Some(handle) = handle else {
            return Ok(());
        };
        let mut managers = self
            .managers
            .try_borrow_mut()
            .map_err(|_| error("менеджер временных таблиц уже используется"))?;
        let entry = managers
            .get_mut(&handle.value())
            .ok_or_else(|| error("менеджер временных таблиц не принадлежит сеансу"))?;
        if Some(entry.revision) != revision {
            return Err(error(
                "менеджер временных таблиц изменился во вложенном вызове",
            ));
        }
        if entry
            .state
            .as_ref()
            .is_some_and(|current| current != &state)
        {
            let revision = entry
                .revision
                .checked_add(1)
                .ok_or_else(|| error("исчерпан счётчик операций временных таблиц"))?;
            entry.state = Some(state);
            entry.revision = revision;
        }
        Ok(())
    }

    fn execute_text(
        &self,
        source: &str,
        parameters: &[QueryParameter],
        handle: Option<TempTablesManagerHandle>,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<QueryResultData>> {
        self.execute_text_with_access(
            source,
            parameters,
            handle,
            principal,
            context,
            ReadAccess::CurrentUser,
        )
    }

    fn execute_text_with_access(
        &self,
        source: &str,
        parameters: &[QueryParameter],
        handle: Option<TempTablesManagerHandle>,
        principal: &Principal,
        context: &mut CallContext<'_>,
        access: ReadAccess,
    ) -> RtResult<Option<QueryResultData>> {
        self.check_principal(principal)?;
        let (mut temporary, revision, closed) = self.manager_snapshot(handle)?;
        let dialect = self
            .connection
            .try_borrow()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?
            .dialect()
            .ok_or_else(|| error("соединение сеанса закрыто"))?;
        let (compiled, snapshot) = match dialect {
            DatabaseDialect::Postgres => self.compile(
                source,
                parameters,
                open_sdbl::query::PostgresBackend,
                &mut temporary,
                context,
                access,
            )?,
            DatabaseDialect::MsSql { backend } => {
                self.compile(source, parameters, backend, &mut temporary, context, access)?
            }
        };
        self.check_manager_revision(handle, revision)?;
        if closed && temporary != TempTablesManager::new() {
            return Err(error("менеджер временных таблиц закрыт"));
        }
        let compiled = match compiled {
            MaterializedBatchResult::Dropped => {
                self.commit_manager(handle, revision, temporary)?;
                return Ok(None);
            }
            MaterializedBatchResult::Placed(count) => {
                let count = i64::try_from(count)
                    .map_err(|_| error("число помещённых строк превышает диапазон BSL"))?;
                let result = QueryResultData::new(
                    vec!["Count".into()],
                    vec![vec![Value::number_from_i64(count)]],
                )
                .map_err(|failure| error(failure.to_string()))?;
                self.commit_manager(handle, revision, temporary)?;
                return Ok(Some(result));
            }
            MaterializedBatchResult::Query(compiled) => compiled,
        };
        if !compiled.deferred_presentations.is_empty()
            || !compiled.nested.is_empty()
            || compiled
                .service_columns
                .iter()
                .any(|index| Some(*index) != compiled.forbidden_row_column)
        {
            return Err(error(
                "составной результат запроса пока не поддерживается адаптером",
            ));
        }
        let projection = compiled
            .logical_projection
            .as_ref()
            .ok_or_else(|| error("полная логическая проекция результата недоступна"))?;
        let columns = projection
            .iter()
            .map(|column| column.name.clone())
            .collect::<Vec<_>>();
        // Тип берём из декларации поля, а не из содержимого бинарной колонки.
        let storage_columns = compiled.columns.iter().map(|column| {
            matches!(column.kind, ColumnKind::Binary { .. })
                && column.origin.as_ref().is_some_and(|origin| {
                    !origin.composite_member && snapshot.descriptors().iter().any(|descriptor| {
                        origin.field == open_sdbl::metadata::FieldId::Metadata(
                            open_sdbl::metadata::AttributeId::from(&descriptor.object_guid))
                            && descriptor.declared_types.as_ref().is_some_and(|types| {
                                types.len() == 1 && super::document_attributes::is_value_storage_declared_type(&types[0])
                            })
                    })
                })
        }).collect::<Vec<_>>();
        let raw = self
            .connection
            .try_borrow_mut()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?
            .query(&compiled.sql, compiled.columns.len())
            .map_err(|failure| error(failure.to_string()))?;
        validate_query_rows(&raw, compiled.columns.len(), compiled.forbidden_row_column)?;
        let rows = raw
            .into_iter()
            .map(|row| {
                logical_result::materialize_row(row, projection, &mut |index, cell| {
                    let column = compiled
                        .columns
                        .get(index)
                        .ok_or_else(|| error("индекс физической колонки неверен"))?;
                    if storage_columns[index]
                        && let Cell::Bytes(content) = &cell
                    {
                        let text = open_sdbl::metadata::decode_stored_value_text(
                            content,
                            MAX_PLACEMENT_BYTES,
                        )
                        .map_err(|failure| error(failure.to_string()))?;
                        let catalog = std::sync::Arc::clone(
                            context
                                .host_data()?
                                .downcast_ref::<SessionData>()
                                .ok_or_else(|| error("данные сеанса не переданы"))?
                                .catalog(),
                        );
                        let mut resolve = |kind: &str, payload: &str| {
                            resolve_storage_scalar(kind, payload, &snapshot, &catalog)
                        };
                        return bsl_orm::value_storage_from_internal_with_resolver(
                            &text,
                            context,
                            &mut resolve,
                        );
                    }
                    value_from_cell_with_metadata(cell, &column.kind, &snapshot, context)
                })
            })
            .collect::<RtResult<Vec<_>>>()?;
        let result =
            QueryResultData::new(columns, rows).map_err(|failure| error(failure.to_string()))?;
        self.commit_manager(handle, revision, temporary)?;
        Ok(Some(result))
    }

    fn compile<B: Backend>(
        &self,
        source: &str,
        parameters: &[QueryParameter],
        backend: B,
        temporary: &mut TempTablesManager,
        context: &mut CallContext<'_>,
        access: ReadAccess,
    ) -> RtResult<(MaterializedBatchResult, Arc<MetadataSnapshot>)> {
        let mut session = SessionParameters::new();
        let mut initialized = Vec::<String>::new();
        let mut snapshot = match &self.configuration_source {
            Some(provider) => provider.query_metadata(source).map_err(error)?,
            None => self.metadata.clone(),
        };
        'prepare: loop {
            let compiler = QueryCompiler::new(&snapshot, backend);
            let prepared = compiler
                .prepare_with_options(
                    source,
                    &PrepareOptions::new()
                        .restricted()
                        .temporary_tables(temporary),
                )
                .map_err(|failure| error(failure.to_string()))?;
            if !prepared.presentation_request().targets.is_empty() {
                return Err(error(
                    "представления ссылок в запросе пока не поддерживаются адаптером",
                ));
            }
            // Режим вычисляется ядром для текущего кадра с учётом безопасного
            // режима. Адаптер не хранит собственный флаг и не меняет роли пользователя.
            let privileged_frame = context.privileged_mode()?;
            if matches!(access, ReadAccess::CurrentUser) && !privileged_frame {
                check_user_field_access(&self.access, &snapshot, prepared.field_usage())
                    .map_err(|failure| error(failure.to_string()))?;
            }
            loop {
                let decisions = match access {
                    ReadAccess::PrivilegedSource(source) => {
                        privileged_option_decisions(prepared.restriction_request(), source)?
                    }
                    ReadAccess::CurrentUser if privileged_frame => prepared
                        .restriction_request()
                        .targets
                        .iter()
                        .cloned()
                        .map(AccessDecision::unrestricted)
                        .collect(),
                    ReadAccess::CurrentUser => match user_decisions(
                        &self.access,
                        &snapshot,
                        prepared.restriction_request(),
                        &[],
                        &session,
                    ) {
                        Ok(decisions) => decisions,
                        Err(failure) => {
                            if let Some(name) = failure.missing_session_parameter() {
                                initialize_parameter(
                                    name,
                                    &mut session,
                                    &mut initialized,
                                    context,
                                )?;
                                continue;
                            }
                            return Err(error(failure.to_string()));
                        }
                    },
                };
                if let Some(provider) = &self.configuration_source {
                    let mut current = snapshot.clone();
                    for decision in &decisions {
                        let Some(restriction) = decision.restriction() else {
                            continue;
                        };
                        let object = snapshot
                            .object_by_id(decision.object())
                            .ok_or_else(|| error("нет объекта ограничения"))?;
                        let name = open_sdbl::query::object_query_name(&snapshot, object)
                            .ok_or_else(|| error("нет имени объекта ограничения"))?;
                        let scope = open_sdbl::access::RestrictionScope {
                            table_name: &name,
                            right: &open_sdbl::metadata::Right::Read,
                            session: &session,
                        };
                        let expanded = open_sdbl::access::expand_restriction(
                            restriction.condition(),
                            &[],
                            &scope,
                        )
                        .map_err(|failure| error(failure.to_string()))?;
                        let alias = expanded
                            .alias
                            .as_deref()
                            .unwrap_or(open_sdbl::access::CURRENT_TABLE);
                        let discovery = format!(
                            "ВЫБРАТЬ 1 ИЗ {name} КАК {alias} {} ГДЕ {}",
                            expanded.joins, expanded.condition
                        );
                        current = provider.query_metadata(&discovery).map_err(error)?;
                    }
                    if !Arc::ptr_eq(&snapshot, &current) {
                        snapshot = current;
                        continue 'prepare;
                    }
                }
                let options = CompileOptions::new()
                    .parameters(parameters)
                    .session(&session)
                    .decisions(&decisions)
                    .ordinary_row_checks(true);
                match prepared.compile_materialized_batch::<CaptureError, _>(
                    &snapshot,
                    &options,
                    temporary,
                    |selection| {
                        // Непроверенный вложенный SELECT отклоняем до чтения источника,
                        // а не после материализации временной таблицы.
                        require_allowed_for_restrictions(source, &decisions)
                            .map_err(CaptureError::Runtime)?;
                        self.capture_placement(selection, &snapshot)
                    },
                ) {
                    Ok(compiled) => {
                        if !matches!(&compiled, MaterializedBatchResult::Query(query)
                        if query.forbidden_row_column.is_some())
                        {
                            require_allowed_for_restrictions(source, &decisions)?;
                        }
                        return Ok((compiled, snapshot));
                    }
                    Err(CaptureError::Compile(failure))
                        if failure.missing_separation_condition().is_some() =>
                    {
                        self.initialize_separation_condition(
                            failure
                                .missing_separation_condition()
                                .expect("проверено выше"),
                            &mut session,
                            context,
                        )?;
                    }
                    Err(CaptureError::Compile(failure))
                        if failure.is_missing_session_parameter() =>
                    {
                        let name = failure
                            .missing_parameter_name()
                            .ok_or_else(|| error("компилятор не указал имя параметра сеанса"))?;
                        initialize_parameter(name, &mut session, &mut initialized, context)?;
                    }
                    Err(CaptureError::Compile(failure)) => {
                        return Err(error(failure.to_string()));
                    }
                    Err(CaptureError::Runtime(failure)) => return Err(failure),
                }
            }
        }
    }

    fn initialize_separation_condition(
        &self,
        condition: &Guid,
        target: &mut SessionParameters,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        let snapshot = self.object_snapshot(condition.as_str())?;
        if target.separation_condition(condition).is_some() {
            return Err(error(
                "компилятор повторно запросил уже переданное условие разделения",
            ));
        }
        let (catalog, principal) = {
            let data = context
                .host_data()?
                .downcast_ref::<SessionData>()
                .ok_or_else(|| error("данные сеанса не переданы"))?;
            (
                std::sync::Arc::clone(data.catalog()),
                data.principal().clone(),
            )
        };
        let constant = catalog
            .object_by_id(condition.as_str())
            .filter(|object| object.kind() == MetadataKind::Constant)
            .ok_or_else(|| error("условие разделения не указывает на приобретённую константу"))?;
        if !snapshot.descriptors().iter().any(|descriptor| {
            descriptor.object_guid == *condition
                && descriptor
                    .declared_types
                    .as_ref()
                    .is_some_and(|types| types.len() == 1 && types[0].tag == "B")
        }) {
            return Err(error("Boolean-тип константы условия разделения неизвестен"));
        }
        if self.reading_conditions.borrow().contains(condition) {
            return Err(error("циклическое чтение константы условия разделения"));
        }
        // Читаем условие тем же адаптером и от текущего пользователя. Значение
        // живёт только в компиляции этого запроса: следующий запрос читает заново.
        self.reading_conditions.borrow_mut().push(condition.clone());
        let result = self.constant_value(constant, &principal, context);
        self.reading_conditions.borrow_mut().pop();
        let Value::Boolean(value) = result? else {
            return Err(error("константа условия разделения не вернула Boolean"));
        };
        target.set_separation_condition(condition.clone(), value);
        Ok(())
    }

    fn read_constant(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        context: &mut CallContext<'_>,
        privileged: bool,
    ) -> RtResult<Value> {
        let snapshot = self.object_snapshot(metadata.id())?;
        if metadata.kind() != MetadataKind::Constant {
            return Err(error(
                "значение запрошено у объекта, который не является константой",
            ));
        }
        let object_id = snapshot
            .object_id(SdblMetadataKind::Constant, metadata.name())
            .map_err(|failure| error(failure.to_string()))?;
        let object = snapshot
            .object_by_id(object_id)
            .ok_or_else(|| error("константа отсутствует в снимке данных"))?;
        ensure_same_config_object(metadata, object.guid.as_str())?;
        let query = format!(
            "ВЫБРАТЬ ПЕРВЫЕ 1 К.Значение КАК Значение ИЗ Константа.{} КАК К",
            metadata.name()
        );
        let access = if privileged {
            ReadAccess::PrivilegedSource(object_id)
        } else {
            ReadAccess::CurrentUser
        };
        let result = self
            .execute_text_with_access(&query, &[], None, principal, context, access)?
            .ok_or_else(|| error("константа не вернула результат"))?;
        result
            .rows()
            .first()
            .and_then(|row| row.first())
            .cloned()
            .ok_or_else(|| error("константа не вернула значение"))
    }

    fn capture_placement(
        &self,
        selection: &CompiledQuery,
        snapshot: &MetadataSnapshot,
    ) -> Result<Vec<Vec<ParameterValue>>, CaptureError> {
        if !selection.deferred_presentations.is_empty()
            || !selection.nested.is_empty()
            || !selection.service_columns.is_empty()
        {
            return Err(CaptureError::Runtime(error(
                "составной источник временной таблицы пока не поддерживается адаптером",
            )));
        }
        let mut rows = Vec::new();
        let mut retained_bytes = 0_usize;
        self.connection
            .try_borrow_mut()
            .map_err(|_| {
                CaptureError::Runtime(error("соединение уже используется вложенным вызовом"))
            })?
            .query_each(&selection.sql, selection.columns.len(), |row| {
                if row.len() != selection.columns.len() {
                    return Err(DbError::Data(
                        "ширина строки SQL не совпадает с результатом компилятора".into(),
                    ));
                }
                let row_bytes = row.iter().try_fold(32_usize, |size, cell| {
                    size.checked_add(materialized_cell_size(cell))
                        .ok_or_else(|| DbError::Data("размер временной таблицы переполнен".into()))
                })?;
                retained_bytes = retained_bytes
                    .checked_add(row_bytes)
                    .ok_or_else(|| DbError::Data("размер временной таблицы переполнен".into()))?;
                if retained_bytes > MAX_PLACEMENT_BYTES {
                    return Err(DbError::Data(
                        "временная таблица превышает предел 8 МиБ на помещение".into(),
                    ));
                }
                let values = row
                    .into_iter()
                    .zip(&selection.columns)
                    .map(|(cell, column)| {
                        materialized_parameter(cell, &column.kind, snapshot)
                            .map_err(|failure| DbError::Data(failure.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                rows.push(values);
                Ok(RowFlow::Continue)
            })
            .map_err(|failure| CaptureError::Runtime(error(failure.to_string())))?;
        Ok(rows)
    }
}

fn materialized_cell_size(cell: &Cell) -> usize {
    match cell {
        Cell::Text(value) | Cell::Number(value) => value.len().saturating_add(32),
        Cell::Bytes(value) => value.len().saturating_add(32),
        _ => 32,
    }
}

impl DataAccess for DatabaseDataAccess {
    fn access_right(
        &self,
        request: &bsl_orm::AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        self.check_principal_identity(principal)?;
        let snapshot = self.object_snapshot(request.metadata().id())?;
        configuration_rights::metadata_access_right(
            &self.access,
            &snapshot,
            request,
            principal,
            context,
            false,
        )
    }

    fn verify_access_rights(
        &self,
        request: &bsl_orm::AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.check_principal_identity(principal)?;
        let snapshot = self.object_snapshot(request.metadata().id())?;
        let granted = configuration_rights::metadata_access_right(
            &self.access,
            &snapshot,
            request,
            principal,
            context,
            true,
        )?;
        access_verification::record_verification(request, context, granted)
    }

    fn exclusive_mode(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<bool> {
        self.check_principal_identity(principal)?;
        let connection = self
            .connection
            .try_borrow()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?;
        if connection.is_closed() {
            return Err(error("соединение сеанса закрыто"));
        }
        if connection.is_dead() {
            return Err(error("соединение сеанса непригодно для использования"));
        }
        // Хост открывает обычный SDK-сеанс без владения монопольным режимом.
        // Это не проверка доступности базы для получения монопольной блокировки.
        Ok(false)
    }

    fn begin_transaction(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<()> {
        self.check_principal_identity(principal)?;
        let depth = self.transaction_depth.get();
        let next = depth
            .checked_add(1)
            .ok_or_else(|| error("исчерпана глубина транзакции"))?;
        let mut connection = self
            .connection
            .try_borrow_mut()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?;
        if depth == 0 {
            connection
                .begin_read_only_transaction()
                .map_err(|failure| error(failure.to_string()))?;
        } else if !connection
            .transaction_active()
            .map_err(|failure| error(failure.to_string()))?
        {
            self.transaction_depth.set(0);
            return Err(error("транзакция соединения уже завершена"));
        }
        self.transaction_depth.set(next);
        Ok(())
    }

    fn commit_transaction(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<()> {
        self.check_principal_identity(principal)?;
        let depth = self.transaction_depth.get();
        if depth == 0 {
            return Err(error("транзакция не начата"));
        }
        let mut connection = self
            .connection
            .try_borrow_mut()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?;
        if !connection
            .transaction_active()
            .map_err(|failure| error(failure.to_string()))?
        {
            self.transaction_depth.set(0);
            self.transaction_cancelled.set(false);
            return Err(error("транзакция соединения уже завершена"));
        }
        if depth > 1 {
            self.transaction_depth.set(depth - 1);
            return Ok(());
        }
        // Внутренний откат запрещает чтения до внешней границы; внешняя
        // фиксация такой транзакции завершает реальный откат соединения.
        let result = if self.transaction_cancelled.get() {
            connection.rollback_transaction()
        } else {
            connection.commit_transaction()
        };
        self.transaction_depth.set(0);
        self.transaction_cancelled.set(false);
        result.map_err(|failure| error(failure.to_string()))
    }

    fn rollback_transaction(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<()> {
        self.check_principal_identity(principal)?;
        let depth = self.transaction_depth.get();
        if depth == 0 {
            return Err(error("транзакция не начата"));
        }
        let mut connection = self
            .connection
            .try_borrow_mut()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?;
        if !connection
            .transaction_active()
            .map_err(|failure| error(failure.to_string()))?
        {
            self.transaction_depth.set(0);
            self.transaction_cancelled.set(false);
            return Err(error("транзакция соединения уже завершена"));
        }
        if depth > 1 {
            self.transaction_depth.set(depth - 1);
            self.transaction_cancelled.set(true);
            return Ok(());
        }
        let result = connection.rollback_transaction();
        self.transaction_depth.set(0);
        self.transaction_cancelled.set(false);
        result.map_err(|failure| error(failure.to_string()))
    }

    fn transaction_active(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<bool> {
        self.check_principal_identity(principal)?;
        let active = self
            .connection
            .try_borrow_mut()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?
            .transaction_active()
            .map_err(|failure| error(failure.to_string()))?;
        if !active {
            self.transaction_depth.set(0);
            self.transaction_cancelled.set(false);
        }
        Ok(active)
    }

    fn constant_value(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        self.read_constant(metadata, principal, context, false)
    }

    fn functional_option(
        &self,
        request: &FunctionalOptionRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        self.check_principal(principal)?;
        let catalog = context
            .host_data()?
            .downcast_ref::<SessionData>()
            .map(|data| std::sync::Arc::clone(data.catalog()))
            .ok_or_else(|| error("данные сеанса не переданы"))?;
        let option = catalog
            .object(MetadataKind::FunctionalOption, request.name())
            .ok_or_else(|| {
                error(format!(
                    "функциональная опция «{}» не найдена",
                    request.name()
                ))
            })?;
        let option = catalog
            .materialized_object_by_id(option.id())?
            .ok_or_else(|| error("функциональная опция отсутствует"))?;
        let settings = option
            .functional_option_settings()
            .ok_or_else(|| error("настройки функциональной опции не приобретены"))?;
        let privileged = settings
            .privileged_get_mode
            .ok_or_else(|| error("режим получения функциональной опции неизвестен"))?;
        match settings.location.as_ref() {
            Some(FunctionalOptionLocation::Constant(id)) => {
                let constant = catalog
                    .object_by_id(id)
                    .ok_or_else(|| error("константа функциональной опции отсутствует"))?;
                self.read_constant(constant, principal, context, privileged)
            }
            Some(
                FunctionalOptionLocation::CatalogAttribute { .. }
                | FunctionalOptionLocation::InformationRegisterResource { .. },
            ) => {
                // Константа ниже приобретает собственный полный снимок. Здесь
                // он нужен только для плана параметризованного источника.
                let snapshot = self.metadata_snapshot()?;
                let supplied = request.parameters().collect::<Vec<_>>();
                let plan = super::functional_option_reads::prepare(
                    &snapshot, &catalog, settings, &supplied,
                )?;
                let access = if privileged {
                    ReadAccess::PrivilegedSource(plan.owner)
                } else {
                    ReadAccess::CurrentUser
                };
                let result = self
                    .execute_text_with_access(
                        &plan.query,
                        &plan.parameters,
                        None,
                        principal,
                        context,
                        access,
                    )?
                    .ok_or_else(|| error("источник опции не вернул результат запроса"))?;
                Ok(plan.value(result.rows()))
            }
            Some(FunctionalOptionLocation::Absent) => {
                Err(error("источник функциональной опции отсутствует"))
            }
            None => Err(error("источник функциональной опции неизвестен")),
        }
    }

    fn find_by_attribute(
        &self,
        metadata: &MetadataObject,
        request: &FindByAttributeRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        self.check_principal(principal)?;
        let snapshot = self.object_snapshot(metadata.id())?;
        let (kind, source_kind) = match metadata.kind() {
            MetadataKind::Catalog => (SdblMetadataKind::Catalog, "Справочник"),
            MetadataKind::Document => (SdblMetadataKind::Document, "Документ"),
            _ => return Err(error("вид объекта не поддерживает поиск по реквизиту")),
        };
        let owner = snapshot
            .object_id(kind, metadata.name())
            .map_err(|failure| error(failure.to_string()))?;
        let object = snapshot
            .object_by_id(owner)
            .ok_or_else(|| error(format!("{source_kind} отсутствует в снимке данных")))?;
        ensure_same_config_object(metadata, object.guid.as_str())?;
        let attribute = snapshot
            .attribute_id(owner, request.attribute())
            .map_err(|failure| error(failure.to_string()))?;
        snapshot
            .attribute_by_id(attribute)
            .map_err(|failure| error(failure.to_string()))?;
        // Тип читается из той же проекции SDK, которую использует компилятор.
        // Пробная компиляция SQL потребовала бы параметры разделения раньше
        // обычного пути их ленивой инициализации через текущий контекст.
        let fields =
            queryable_fields(&snapshot, object).map_err(|failure| error(failure.to_string()))?;
        let field = fields
            .iter()
            .find(|field| field.field == Some(open_sdbl::metadata::FieldId::Metadata(attribute)))
            .ok_or_else(|| error("тип реквизита не приобретён"))?;
        let [column] = field.columns.as_slice() else {
            return Err(error(
                "составной тип реквизита для поиска пока не поддерживается",
            ));
        };
        let kind = column.kind.clone();
        match &kind {
            ColumnKind::String {
                length: Some(length),
            } if *length > 0 => {}
            ColumnKind::Number { .. }
            | ColumnKind::Boolean
            | ColumnKind::DateTime
            | ColumnKind::Reference { .. } => {}
            _ => return Err(error("тип реквизита не поддерживает поиск по реквизиту")),
        }
        // При полном пустом составе владельцев аргумент не ограничивает поиск:
        // это измерено в `measure-catalog-find-attribute`. Неизвестный состав
        // и подчинённые справочники сохраняют явный отказ.
        if request.owner().is_some()
            && !metadata
                .catalog_owners()
                .is_some_and(|owners| owners.is_empty())
        {
            return Err(error(
                "сведения о владельцах справочника для поиска не приобретены",
            ));
        }
        let comparison = if matches!(request.value(), Value::Undefined) {
            // Литерал сохраняет различие между `Неопределено` и SQL NULL.
            // Компилятор проверяет тип поля; запрос по-прежнему проходит права и RLS.
            format!("Источник.{} = НЕОПРЕДЕЛЕНО", request.attribute())
        } else if matches!(kind, ColumnKind::String { .. })
            && matches!(request.value(), Value::Str(_))
        {
            format!("СОКРП(Источник.{}) = СОКРП(&Значение)", request.attribute())
        } else {
            format!("Источник.{} = &Значение", request.attribute())
        };
        let mut predicates = vec![comparison];
        let mut parameters = if matches!(request.value(), Value::Undefined) {
            vec![]
        } else {
            vec![QueryParameter::new(
                "Значение",
                parameter_value(request.value())?,
            )]
        };
        if let Some(parent) = request.parent() {
            match metadata.catalog_hierarchical() {
                Some(false) => {}
                Some(true) => {
                    let (owner, _) = bsl_orm::catalog_reference_parts(parent).ok_or_else(|| {
                        error("родитель поиска должен быть ссылкой того же справочника")
                    })?;
                    ensure_same_config_object(owner, object.guid.as_str())?;
                    predicates.push("Источник.Родитель = &Родитель".into());
                    parameters.push(QueryParameter::new("Родитель", parameter_value(parent)?));
                }
                None => return Err(error("иерархичность справочника не приобретена")),
            }
        }
        let source = format!(
            "ВЫБРАТЬ ПЕРВЫЕ 1 Источник.Ссылка КАК Ссылка ИЗ {source_kind}.{} КАК Источник ГДЕ {}",
            metadata.name(),
            predicates.join(" И ")
        );
        let result = self
            .execute_text(&source, &parameters, None, principal, context)?
            .ok_or_else(|| error("поиск по реквизиту не вернул результат"))?;
        match result.rows().first() {
            None => Ok(None),
            Some(row) => row
                .first()
                .cloned()
                .map(Some)
                .ok_or_else(|| error("поиск по реквизиту не вернул ссылку")),
        }
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.execute_query_result(request, principal, context)?
            .ok_or_else(|| error("пакет запроса не возвращает результат"))
    }

    fn execute_query_result(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<QueryResultData>> {
        let parameters = query_parameters(request.text(), request.parameters())?;
        self.execute_text(
            request.text(),
            &parameters,
            request.temp_tables_manager(),
            principal,
            context,
        )
    }

    fn create_temp_tables_manager(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<TempTablesManagerHandle> {
        self.check_principal(principal)?;
        let connection = self
            .connection
            .try_borrow()
            .map_err(|_| error("соединение уже используется вложенным вызовом"))?;
        if connection.dialect().is_none() {
            return Err(error("соединение сеанса закрыто"));
        }
        drop(connection);
        let next = self
            .next_manager
            .get()
            .checked_add(1)
            .ok_or_else(|| error("исчерпаны идентификаторы временных таблиц"))?;
        let mut managers = self
            .managers
            .try_borrow_mut()
            .map_err(|_| error("менеджер временных таблиц уже используется"))?;
        managers.insert(
            next,
            ManagerEntry {
                state: Some(TempTablesManager::new()),
                revision: 0,
            },
        );
        self.next_manager.set(next);
        Ok(TempTablesManagerHandle::new(next))
    }

    fn close_temp_tables_manager(
        &self,
        manager: TempTablesManagerHandle,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.check_principal(principal)?;
        let mut managers = self
            .managers
            .try_borrow_mut()
            .map_err(|_| error("менеджер временных таблиц уже используется"))?;
        let entry = managers
            .get_mut(&manager.value())
            .ok_or_else(|| error("менеджер временных таблиц не принадлежит сеансу"))?;
        if entry.state.is_some() {
            let revision = entry
                .revision
                .checked_add(1)
                .ok_or_else(|| error("исчерпан счётчик операций временных таблиц"))?;
            entry.state = None;
            entry.revision = revision;
        }
        Ok(())
    }
}

fn privileged_option_decisions(
    request: &RestrictionRequest,
    source: ObjectId,
) -> RtResult<Vec<AccessDecision>> {
    let [target] = request.targets.as_slice() else {
        return Err(error(
            "привилегированное чтение опции требует единственный источник",
        ));
    };
    if target.object != source || target.table_part.is_some() {
        return Err(error(
            "источник привилегированного чтения не совпадает с источником опции",
        ));
    }
    Ok(vec![AccessDecision::unrestricted(target.clone())])
}

fn ensure_same_config_object(metadata: &MetadataObject, acquired_id: &str) -> RtResult<()> {
    if !metadata.id().eq_ignore_ascii_case(acquired_id) {
        return Err(error(format!(
            "идентификатор константы «{}» не совпадает со снимком данных",
            metadata.name()
        )));
    }
    Ok(())
}

fn all_selections_allowed(source: &str) -> RtResult<bool> {
    let tokens = open_sdbl::tokenize(source).map_err(|failure| error(failure.to_string()))?;
    let mut selections = 0;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == open_sdbl::TokenKind::Keyword(Keyword::Select) {
            selections += 1;
            if tokens.get(index + 1).map(|next| &next.kind)
                != Some(&open_sdbl::TokenKind::Keyword(Keyword::Allowed))
            {
                return Ok(false);
            }
        }
    }
    Ok(selections > 0)
}

// Проверяем весь SQL-результат до материализации значений: служебная строка
// отказа содержит NULL вместо данных и не должна попасть в объектную модель.
fn validate_query_rows(
    rows: &[Vec<Cell>],
    width: usize,
    forbidden_row_column: Option<usize>,
) -> RtResult<()> {
    if forbidden_row_column.is_some_and(|index| index >= width) {
        return Err(error(
            "индекс проверки доступа выходит за ширину SQL-результата",
        ));
    }
    for row in rows {
        if row.len() != width {
            return Err(error(
                "ширина строки SQL не совпадает с результатом компилятора",
            ));
        }
        if let Some(index) = forbidden_row_column {
            match row.get(index) {
                Some(Cell::Bool(false)) => {}
                Some(Cell::Bool(true)) => {
                    return Err(error("недостаточно прав для чтения результата запроса"));
                }
                _ => {
                    return Err(error(
                        "служебная колонка проверки доступа не содержит Boolean",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn require_allowed_for_restrictions(source: &str, decisions: &[AccessDecision]) -> RtResult<()> {
    if decisions
        .iter()
        .any(|decision| !matches!(decision, AccessDecision::Unrestricted(_)))
        && !all_selections_allowed(source)?
    {
        return Err(error(
            "обычный запрос читает данные с ограничением доступа; требуется `ВЫБРАТЬ РАЗРЕШЕННЫЕ`",
        ));
    }
    Ok(())
}

fn initialize_parameter(
    name: &str,
    target: &mut SessionParameters,
    initialized: &mut Vec<String>,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    if initialized
        .iter()
        .any(|known| open_bsl::folded_eq(known, name))
    {
        return Err(error(format!(
            "параметр сеанса «{name}» остался недоступен после инициализации"
        )));
    }
    initialized.push(name.to_owned());
    let parameters = {
        let data = context
            .host_data()?
            .downcast_ref::<SessionData>()
            .ok_or_else(|| error("данные сеанса не переданы"))?;
        Rc::clone(
            data.extension::<SessionRuntime>()
                .ok_or_else(|| error("runtime сеанса не передан"))?
                .parameters(),
        )
    };
    let value = parameters.get(name, context)?;
    target.set(QueryParameter::new(name, parameter_value(&value)?));
    Ok(())
}

// Коллекция BSL может сохранять параметры прежнего текста. Строгому API SDK
// передаём только используемые имена, не преобразуя лишние значения.
fn query_parameters<'a>(
    source: &str,
    parameters: impl Iterator<Item = (&'a str, &'a Value)>,
) -> RtResult<Vec<QueryParameter>> {
    let tokens = open_sdbl::tokenize(source).map_err(|failure| error(failure.to_string()))?;
    parameters
        .filter(|(name, _)| {
            tokens.iter().any(|token| {
                token.kind == open_sdbl::TokenKind::Parameter
                    && open_bsl::folded_eq(
                        name,
                        token.lexeme.strip_prefix('&').unwrap_or(token.lexeme),
                    )
            })
        })
        .map(|(name, value)| parameter_value(value).map(|value| QueryParameter::new(name, value)))
        .collect()
}

fn parameter_date(value: &BslDate) -> RtResult<ParameterDate> {
    if !value.ticks().is_multiple_of(10_000) {
        return Err(error(
            "дробная секунда не поддерживается параметром запроса",
        ));
    }
    let civil = value.to_civil();
    let year =
        u16::try_from(civil.year).map_err(|_| error("дата вне диапазона параметра запроса"))?;
    let date = ParameterDate::new(
        year,
        civil.month as u8,
        civil.day as u8,
        civil.hour as u8,
        civil.minute as u8,
        civil.second as u8,
    )
    .map_err(|failure| error(failure.to_string()))?;
    Ok(date)
}

fn parameter_point(point: &open_bsl::PointInTimeValue) -> RtResult<ParameterPointInTime> {
    let reference = if matches!(point.reference(), Value::Undefined) {
        None
    } else {
        let (metadata, uuid) = bsl_orm::configuration_reference_parts(point.reference())
            .ok_or_else(|| error("ссылка момента времени не поддерживается параметром запроса"))?;
        let object: Guid = metadata
            .id()
            .parse()
            .map_err(|failure: open_sdbl::metadata::MetadataError| error(failure.to_string()))?;
        let object = ObjectId::from_bytes(object.to_bytes());
        let id = open_bsl::uuid::to_1c_bytes(uuid);
        Some(ParameterReference { object, id })
    };
    Ok(ParameterPointInTime {
        date: parameter_date(&point.date())?,
        reference,
    })
}

pub(super) fn parameter_value(value: &Value) -> RtResult<ParameterValue> {
    if let Some((value, kind)) = bsl_orm::query_boundary_parts(value) {
        let value = match parameter_value(value)? {
            ParameterValue::Date(date) => ParameterBoundaryValue::Date(date),
            ParameterValue::PointInTime(point) => ParameterBoundaryValue::PointInTime(point),
            _ => {
                return Err(error(
                    "значение границы запроса должно быть датой или моментом времени",
                ));
            }
        };
        let kind = match kind {
            open_bsl::EnumValue::BoundaryTypeIncluding => BoundaryKind::Including,
            open_bsl::EnumValue::BoundaryTypeExcluding => BoundaryKind::Excluding,
            _ => return Err(error("неизвестный вид границы запроса")),
        };
        return Ok(ParameterValue::Boundary(ParameterBoundary { value, kind }));
    }
    if let Value::Object(object) = value
        && let BslObject::Extension(component) = object.as_ref()
        && let Some(point) = component.downcast_ref::<open_bsl::PointInTimeValue>()
    {
        return parameter_point(point).map(ParameterValue::PointInTime);
    }

    if let Value::Object(object) = value
        && let BslObject::Array(items) = object.as_ref()
    {
        let items = items
            .try_borrow()
            .map_err(|_| error("массив параметра запроса уже изменяется"))?;
        let values = items.iter().map(|item| {
            // Обходим только плоский список: вложенная коллекция может ссылаться
            // на сам массив. Скалярные значения кодируются общим путём.
            if matches!(item, Value::Object(object) if matches!(object.as_ref(), BslObject::Array(_) | BslObject::ValueTable(_))) {
                return Err(error("вложенная коллекция не поддерживается параметром запроса"));
            }
            parameter_value(item)
        }).collect::<RtResult<Vec<_>>>()?;
        return Ok(ParameterValue::List(values));
    }
    if let Some((metadata, uuid)) = bsl_orm::catalog_reference_parts(value) {
        let object: Guid = metadata
            .id()
            .parse()
            .map_err(|failure: open_sdbl::metadata::MetadataError| error(failure.to_string()))?;
        let id: Guid = Value::Object(Rc::new(BslObject::Uuid(uuid)))
            .to_string()
            .parse()
            .map_err(|failure: open_sdbl::metadata::MetadataError| error(failure.to_string()))?;
        return Ok(ParameterValue::Reference {
            object: ObjectId::from_bytes(object.to_bytes()),
            id: id.to_1c_bytes(),
        });
    }
    match value {
        Value::Object(object) => {
            if let BslObject::Uuid(bytes) = object.as_ref() {
                let id = open_bsl::uuid::format(bytes).parse().map_err(
                    |failure: open_sdbl::metadata::MetadataError| error(failure.to_string()),
                )?;
                return Ok(ParameterValue::Uuid(id));
            }
            let BslObject::ValueTable(table) = object.as_ref() else {
                return Err(error(format!(
                    "значение типа {} не поддерживается параметром запроса",
                    value.type_name()
                )));
            };
            let table = table
                .try_borrow()
                .map_err(|_| error("таблица параметра запроса уже изменяется"))?;
            if table.column_names.len() != table.column_types.len()
                || table.column_names.len() != table.columns.len()
            {
                return Err(error("схема таблицы параметра запроса повреждена"));
            }
            let columns = table
                .column_names
                .iter()
                .zip(&table.column_types)
                .map(|(name, types)| {
                    let [column_type] = types.as_deref().unwrap_or_default() else {
                        return Err(error(format!(
                            "колонка «{name}» параметра запроса не имеет одного типа"
                        )));
                    };
                    let kind = match column_type.id.native() {
                        Some(TypeId::Boolean) => ColumnKind::Boolean,
                        Some(TypeId::Number) => ColumnKind::Number {
                            precision: None,
                            scale: None,
                        },
                        Some(TypeId::String) => ColumnKind::String { length: None },
                        Some(TypeId::Date) => ColumnKind::DateTime,
                        _ if bsl_orm::catalog_reference_type_id(&column_type.id).is_some() => {
                            let id: Guid = bsl_orm::catalog_reference_type_id(&column_type.id)
                                .expect("проверен выше")
                                .parse()
                                .map_err(|failure: open_sdbl::metadata::MetadataError| {
                                    error(failure.to_string())
                                })?;
                            ColumnKind::Reference {
                                targets: vec![ObjectId::from_bytes(id.to_bytes())],
                                runtime_typed: false,
                            }
                        }
                        _ => {
                            return Err(error(format!(
                                "тип колонки «{name}» не поддерживается параметром запроса"
                            )));
                        }
                    };
                    Ok(ParameterColumn::new(name.clone(), kind))
                })
                .collect::<RtResult<Vec<_>>>()?;
            let rows = (0..table.row_count())
                .map(|position| {
                    table
                        .columns
                        .iter()
                        .map(|column| {
                            let cell = column.get(position).ok_or_else(|| {
                                error("ширина таблицы параметра запроса повреждена")
                            })?;
                            if matches!(cell, Value::Object(_))
                                && bsl_orm::catalog_reference_parts(cell).is_none()
                            {
                                return Err(error(
                                    "объект в ячейке не поддерживается параметром запроса",
                                ));
                            }
                            let value = parameter_value(cell)?;
                            if matches!(
                                value,
                                ParameterValue::Table { .. } | ParameterValue::List(_)
                            ) {
                                return Err(error(
                                    "вложенная коллекция не поддерживается параметром запроса",
                                ));
                            }
                            Ok(value)
                        })
                        .collect::<RtResult<Vec<_>>>()
                })
                .collect::<RtResult<Vec<_>>>()?;
            Ok(ParameterValue::Table { columns, rows })
        }
        Value::Null => Ok(ParameterValue::Null),
        Value::Undefined => Err(error(
            "передача Неопределено в параметр запроса пока не измерена на 1С",
        )),
        Value::Boolean(value) => Ok(ParameterValue::Boolean(*value)),
        Value::Str(value) => String::from_utf16(value.units())
            .map(ParameterValue::String)
            .map_err(|_| error("строка параметра запроса содержит непарный UTF-16 суррогат")),
        Value::Number(value) => {
            let text = value.to_canonical();
            let (integer, fraction) = text.split_once('.').unwrap_or((&text, ""));
            let scale = u8::try_from(fraction.len())
                .map_err(|_| error("масштаб числа превышает возможности параметра запроса"))?;
            let unscaled = format!("{integer}{fraction}")
                .parse::<i128>()
                .map_err(|_| error("число превышает возможности параметра запроса"))?;
            Ok(ParameterValue::Number { unscaled, scale })
        }
        Value::Date(value) => parameter_date(value).map(ParameterValue::Date),
        _ => Err(error(format!(
            "значение типа {} не поддерживается параметром запроса",
            value.type_name()
        ))),
    }
}

fn materialized_parameter(
    cell: Cell,
    kind: &ColumnKind,
    metadata: &MetadataSnapshot,
) -> RtResult<ParameterValue> {
    match (cell, kind) {
        (Cell::Null, _) => Ok(ParameterValue::Null),
        (Cell::Bool(value), ColumnKind::Boolean) => Ok(ParameterValue::Boolean(value)),
        (Cell::Text(value), ColumnKind::String { .. }) => Ok(ParameterValue::String(value)),
        (Cell::Number(value), ColumnKind::Number { .. }) => {
            let number =
                BslNumber::parse_canonical(&value).map_err(|failure| error(failure.to_string()))?;
            parameter_value(&Value::Number(number))
        }
        (Cell::DateTime(value), ColumnKind::DateTime) => {
            // SQL callback уже возвращает логическую дату 1С, в том числе для MSSQL.
            let year = u16::try_from(value.year)
                .map_err(|_| error("год временной таблицы вне диапазона 1С"))?;
            let month = u8::try_from(value.month)
                .map_err(|_| error("месяц временной таблицы вне диапазона 1С"))?;
            let day = u8::try_from(value.day)
                .map_err(|_| error("день временной таблицы вне диапазона 1С"))?;
            let hour = u8::try_from(value.hour)
                .map_err(|_| error("час временной таблицы вне диапазона 1С"))?;
            let minute = u8::try_from(value.minute)
                .map_err(|_| error("минута временной таблицы вне диапазона 1С"))?;
            let second = u8::try_from(value.second)
                .map_err(|_| error("секунда временной таблицы вне диапазона 1С"))?;
            ParameterDate::new(year, month, day, hour, minute, second)
                .map(ParameterValue::Date)
                .map_err(|failure| error(failure.to_string()))
        }
        (Cell::Bytes(value), ColumnKind::Binary { .. }) => Ok(ParameterValue::Binary(value)),
        (
            Cell::Bytes(value),
            ColumnKind::Reference {
                targets,
                runtime_typed: false,
            },
        ) => {
            let [object] = targets.as_slice() else {
                return Err(error(
                    "тип ссылки временной таблицы не является единственным",
                ));
            };
            let id: [u8; 16] = value
                .try_into()
                .map_err(|_| error("ссылка временной таблицы имеет неверную длину"))?;
            Ok(ParameterValue::Reference {
                object: *object,
                id,
            })
        }
        (
            Cell::Bytes(value),
            ColumnKind::Reference {
                targets,
                runtime_typed: true,
            },
        ) => {
            let payload: [u8; 20] = value
                .try_into()
                .map_err(|_| error("составная ссылка временной таблицы имеет неверную длину"))?;
            let database_type =
                u32::from_be_bytes(payload[..4].try_into().map_err(|_| {
                    error("составная ссылка временной таблицы имеет неверную длину")
                })?);
            let object = metadata
                .object_id_by_database_type(database_type)
                .map_err(|failure| error(failure.to_string()))?;
            if !targets.is_empty() && !targets.contains(&object) {
                return Err(error(
                    "тип ссылки временной таблицы не входит в состав поля",
                ));
            }
            let id: [u8; 16] = payload[4..]
                .try_into()
                .map_err(|_| error("составная ссылка временной таблицы имеет неверную длину"))?;
            Ok(ParameterValue::Reference { object, id })
        }
        _ => Err(error("тип ячейки временной таблицы не поддерживается")),
    }
}

fn value_from_cell_with_metadata(
    cell: Cell,
    kind: &ColumnKind,
    metadata: &MetadataSnapshot,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if matches!(kind, ColumnKind::Reference { .. }) && matches!(cell, Cell::Bytes(_)) {
        let catalog = context
            .host_data()?
            .downcast_ref::<SessionData>()
            .map(|session| std::sync::Arc::clone(session.catalog()))
            .ok_or_else(|| error("данные сеанса не переданы"))?;
        return reference_from_cell(cell, kind, metadata, catalog);
    }
    value_from_cell(cell, kind)
}

fn reference_from_cell(
    cell: Cell,
    kind: &ColumnKind,
    metadata: &MetadataSnapshot,
    catalog: std::sync::Arc<bsl_orm::MetadataCatalog>,
) -> RtResult<Value> {
    let ParameterValue::Reference { object, id } = materialized_parameter(cell, kind, metadata)?
    else {
        return Err(error("результат не является ссылкой"));
    };
    let object_id = object.to_string();
    if catalog.object_by_id(&object_id).is_some_and(|owner| {
        matches!(
            owner.kind(),
            bsl_orm::MetadataKind::ChartOfAccounts
                | bsl_orm::MetadataKind::Enumeration
                | bsl_orm::MetadataKind::Document
        )
    }) {
        return bsl_orm::configuration_reference_value(
            catalog,
            &object_id,
            super::canonical_uuid_from_physical(id),
        );
    }
    bsl_orm::catalog_reference_value(catalog, &object_id, super::canonical_uuid_from_physical(id))
}

fn value_from_cell(cell: Cell, kind: &ColumnKind) -> RtResult<Value> {
    match (cell, kind) {
        (Cell::Null, ColumnKind::Undefined) => Ok(Value::Undefined),
        (Cell::Null, _) => Ok(Value::Null),
        (Cell::Uuid(value), ColumnKind::Uuid) => Ok(Value::Object(Rc::new(BslObject::Uuid(value)))),
        (Cell::Bool(value), ColumnKind::Boolean) => Ok(Value::Boolean(value)),
        (Cell::Text(value), ColumnKind::String { .. }) => {
            Ok(Value::Str(BslString::from_utf8_string(value)))
        }
        (Cell::Number(value), ColumnKind::Number { .. }) => BslNumber::parse_canonical(&value)
            .map(Value::Number)
            .map_err(|failure| error(failure.to_string())),
        (Cell::DateTime(value), ColumnKind::DateTime) => BslDate::from_civil(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
        )
        .map(Value::Date)
        .ok_or_else(|| error("дата результата запроса вне диапазона BSL")),
        (Cell::Bytes(_) | Cell::Uuid(_), _) => Err(error(
            "двоичное значение или ссылка результата запроса пока не поддерживается",
        )),
        _ => Err(error("тип ячейки SQL не совпадает с типом колонки запроса")),
    }
}

fn error(message: impl Into<String>) -> RtError {
    RtError::DynamicError(message.into())
}

#[cfg(test)]
#[path = "data_access_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_transactions_tests.rs"]
mod session_transactions_tests;

fn resolve_storage_scalar(
    kind: &str,
    payload: &str,
    metadata: &MetadataSnapshot,
    catalog: &std::sync::Arc<bsl_orm::MetadataCatalog>,
) -> RtResult<Option<Value>> {
    if kind.eq_ignore_ascii_case("fc01b5df-97fe-449b-83d4-218a090e681e") {
        return Ok(Some(Value::Object(Rc::new(BslObject::Uuid(
            open_bsl::uuid::parse(payload)?,
        )))));
    }
    let mut owners = metadata.descriptors().iter().filter(|descriptor| {
        descriptor
            .object_reference_type
            .as_ref()
            .is_some_and(|ty| ty.as_str().eq_ignore_ascii_case(kind))
    });
    let Some(owner) = owners.next() else {
        return Ok(None);
    };
    if owners.next().is_some() {
        return Err(error("тип внутренней ссылки неоднозначен в метаданных"));
    }
    if !catalog
        .object_by_id(owner.object_guid.as_str())
        .is_some_and(|object| object.kind() == MetadataKind::Catalog)
    {
        return Ok(None);
    }
    let (prefix, identity) = payload
        .split_once(':')
        .ok_or_else(|| error("неверная скалярная запись ссылки"))?;
    // Нативный контроль допускает знак и ведущие нули; числовое значение
    // не меняет тип или UUID ссылки и не является ключом поиска владельца.
    // Замер: `tests/conformance/measure/1c/measure-storage-reference-prefix.platform.txt`.
    let digits = prefix
        .strip_prefix('+')
        .or_else(|| prefix.strip_prefix('-'))
        .unwrap_or(prefix);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(error("неверный числовой префикс ссылки"));
    }
    let uuid = super::canonical_uuid_from_physical_hex(identity)
        .ok_or_else(|| error("неверный физический UUID внутренней ссылки"))?;
    bsl_orm::catalog_reference_value(
        std::sync::Arc::clone(catalog),
        owner.object_guid.as_str(),
        uuid,
    )
    .map(Some)
}

#[cfg(test)]
#[path = "data_access_uuid_tests.rs"]
mod uuid_tests;

#[cfg(test)]
#[path = "query_parameter_selection_tests.rs"]
mod query_parameter_selection_tests;

#[cfg(test)]
#[path = "data_access_enumeration_tests.rs"]
mod data_access_enumeration_tests;

#[cfg(test)]
#[path = "data_access_array_tests.rs"]
mod array_tests;

#[cfg(test)]
#[path = "local_database_read_tests.rs"]
mod local_database_read_tests;

#[cfg(test)]
#[path = "ordinary_row_tests.rs"]
mod ordinary_row_tests;

#[cfg(test)]
#[path = "local_ordinary_rls_tests.rs"]
mod local_ordinary_rls_tests;

#[cfg(test)]
#[path = "local_functional_option_tests.rs"]
mod local_functional_option_tests;

#[cfg(test)]
#[path = "local_option_source_rights_tests.rs"]
mod local_option_source_rights_tests;

#[cfg(test)]
#[path = "local_option_register_reads_tests.rs"]
mod local_option_register_reads_tests;

#[cfg(test)]
#[path = "local_option_catalog_reads_tests.rs"]
mod local_option_catalog_reads_tests;

#[cfg(test)]
#[path = "local_option_constant_reads_tests.rs"]
mod local_option_constant_reads_tests;

#[cfg(test)]
#[path = "local_conditional_separation_tests.rs"]
mod local_conditional_separation_tests;

#[cfg(test)]
#[path = "local_temporary_tables_tests.rs"]
mod local_temporary_tables_tests;

#[cfg(test)]
#[path = "data_access/access_verification_tests.rs"]
mod access_verification_tests;

#[cfg(test)]
#[path = "data_access_temporal_tests.rs"]
mod data_access_temporal_tests;
