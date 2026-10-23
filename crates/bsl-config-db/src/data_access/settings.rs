//! Чтение общих и системных настроек через измеренные контракты SDK.

use super::*;
use bsl_session::{
    ClientSettingsAdapter, ClientSettingsEntry, ClientSettingsLoadRequest,
    ClientSettingsSaveRequest, ClientSettingsSelectRequest, ClientSettingsStorage,
    HomePageFormsData,
};
use open_sdbl::metadata::{DecodedCommonSetting, decode_common_setting};
use open_sdbl::query::{CommonSettingsSeparation, PostgresBackend};

impl std::fmt::Debug for DatabaseDataAccess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DatabaseDataAccess")
            .finish_non_exhaustive()
    }
}

impl ClientSettingsAdapter for DatabaseDataAccess {
    fn load(&self, _: &Principal, _: &ClientSettingsLoadRequest) -> RtResult<Option<Value>> {
        Err(error(
            "для чтения настроек требуется контекст вызова сеанса",
        ))
    }

    fn load_with_context(
        &self,
        principal: &Principal,
        request: &ClientSettingsLoadRequest,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        let snapshot = self.metadata_snapshot()?;
        self.check_principal(principal)?;
        let table = settings_table(request.storage())?;
        let user = request.user_name().unwrap_or(principal.name());
        let catalog = {
            let data = context
                .host_data()?
                .downcast_ref::<SessionData>()
                .ok_or_else(|| error("данные сеанса не переданы"))?;
            if data.principal().id() != principal.id() {
                return Err(error(
                    "контекст настроек принадлежит другому сеансу пользователя",
                ));
            }
            std::sync::Arc::clone(data.catalog())
        };
        // Имя задаёт точное пространство настроек, а не поиск пользователя.
        // Проверяем доступ до SQL, даже если в таком пространстве нет записи.
        authorize_settings_user(
            &self.access,
            principal,
            user,
            catalog.configuration_object_id(),
            context.privileged_mode()?,
        )?;
        if self.connection.borrow().dialect() != Some(DatabaseDialect::Postgres) {
            return Err(error("чтение настроек этого провайдера не поддержано"));
        }
        let scope = self.common_settings_scope(context, table)?;
        let compiler = QueryCompiler::new(&snapshot, PostgresBackend);
        let sql = if request.storage() == ClientSettingsStorage::System {
            compiler.compile_system_settings_load(
                user,
                request.object_key(),
                request.settings_key(),
                &scope,
            )
        } else {
            compiler.compile_common_settings_load(
                user,
                request.object_key(),
                request.settings_key(),
                &scope,
            )
        }
        .map_err(|failure| error(failure.to_string()))?;
        // Ссылка на соединение отпускается до материализации и возможных
        // вложенных вызовов: состояние не удерживается между BSL-обработчиками.
        let rows = self
            .connection
            .borrow_mut()
            .query(&sql, 1)
            .map_err(|failure| error(failure.to_string()))?;
        let Some(bytes) = one_payload(rows)? else {
            return Ok(None);
        };
        let decoded = decode_common_setting(&bytes, MAX_PLACEMENT_BYTES)
            .map_err(|failure| error(failure.to_string()))?;
        let mut resolver =
            |kind: &str, payload: &str| resolve_storage_scalar(kind, payload, &snapshot, &catalog);
        let value = match decoded {
            DecodedCommonSetting::Value(text) => {
                open_bsl::value_from_string_internal_with_resolver(
                    &text,
                    context.runtime_shapes(),
                    &mut resolver,
                )?
            }
            DecodedCommonSetting::ValueStorage(text) => {
                bsl_orm::value_storage_from_internal_with_resolver(&text, context, &mut resolver)?
            }
            _ => return Err(error("неподдержанный вид значения настройки")),
        };
        Ok(Some(value))
    }

    fn save(&self, _: &Principal, _: &ClientSettingsSaveRequest) -> RtResult<()> {
        Err(error("запись настроек не поддерживается адаптером базы"))
    }
    fn select(
        &self,
        _: &Principal,
        _: &ClientSettingsSelectRequest,
    ) -> RtResult<Vec<ClientSettingsEntry>> {
        Err(error("выборка настроек не поддерживается адаптером базы"))
    }
    fn home_page_forms(&self, _: &Principal) -> RtResult<HomePageFormsData> {
        Err(error(
            "чтение форм начальной страницы не поддерживается адаптером базы",
        ))
    }
    fn set_home_page_forms(&self, _: &Principal, _: &HomePageFormsData) -> RtResult<()> {
        Err(error(
            "запись форм начальной страницы не поддерживается адаптером базы",
        ))
    }
}

impl DatabaseDataAccess {
    fn common_settings_scope(
        &self,
        context: &mut CallContext<'_>,
        table_name: &str,
    ) -> RtResult<Vec<CommonSettingsSeparation>> {
        let snapshot = self.metadata_snapshot()?;
        let table = snapshot
            .live_table(table_name)
            .ok_or_else(|| error("системная таблица настроек не приобретена"))?;
        let mut scope = Vec::new();
        let mut parameters = SessionParameters::new();
        let mut initialized = Vec::new();
        for field in snapshot.separators().filter(|field| {
            table
                .columns
                .iter()
                .any(|column| column.name.eq_ignore_ascii_case(&field.physical_name))
        }) {
            let settings = field
                .separation
                .as_ref()
                .ok_or_else(|| error("настройки разделителя настроек неизвестны"))?;
            if let Some(condition) = &settings.conditional_separation {
                if parameters.separation_condition(condition).is_none() {
                    self.initialize_separation_condition(condition, &mut parameters, context)?;
                }
                let enabled = parameters
                    .separation_condition(condition)
                    .ok_or_else(|| error("глобальное условие разделения не прочитано"))?;
                if !enabled {
                    // Платформа выбирает каноническую область true/пусто,
                    // не читая ни параметр использования, ни значение.
                    scope.push(CommonSettingsSeparation::ConditionDisabled {
                        guid: field.guid.clone(),
                    });
                    continue;
                }
            }
            let use_name = settings.use_parameter.as_deref().ok_or_else(|| {
                error("разделитель настроек без параметра использования не поддержан")
            })?;
            if parameters.get(use_name).is_none() {
                initialize_parameter(use_name, &mut parameters, &mut initialized, context)?;
            }
            let active = match parameters.get(use_name).map(|parameter| parameter.value()) {
                Some(ParameterValue::Boolean(active)) => *active,
                _ => {
                    return Err(error(
                        "параметр использования разделителя настроек не Boolean",
                    ));
                }
            };
            let state = if active {
                let name = settings
                    .value_parameter
                    .as_deref()
                    .or(field.name.as_deref())
                    .ok_or_else(|| error("параметр значения разделителя настроек неизвестен"))?;
                if parameters.get(name).is_none() {
                    initialize_parameter(name, &mut parameters, &mut initialized, context)?;
                }
                let value = parameters
                    .get(name)
                    .map(|parameter| parameter.value())
                    .ok_or_else(|| error("значение разделителя настроек не прочитано"))?;
                if !matches!(
                    value,
                    ParameterValue::String(_) | ParameterValue::Number { .. }
                ) {
                    return Err(error("тип значения разделителя настроек не поддержан"));
                }
                CommonSettingsSeparation::Active {
                    guid: field.guid.clone(),
                    value: value.clone(),
                }
            } else {
                CommonSettingsSeparation::Inactive {
                    guid: field.guid.clone(),
                }
            };
            scope.push(state);
        }
        Ok(scope)
    }
}

fn authorize_settings_user(
    access: &AccessStore,
    principal: &Principal,
    user: &str,
    configuration_id: Option<&str>,
    privileged: bool,
) -> RtResult<()> {
    if user == principal.name() || privileged {
        return Ok(());
    }
    let root: Guid = configuration_id
        .ok_or_else(|| error("идентификатор корня конфигурации не приобретён"))?
        .parse()
        .map_err(|failure| error(format!("неверный идентификатор конфигурации: {failure}")))?;
    if !configuration_rights::grants(
        access,
        &configuration_rights::role_ids(principal)?,
        &root,
        &open_sdbl::metadata::Right::DataAdministration,
    )? {
        return Err(error(
            "для чтения настроек другого пользователя требуется АдминистрированиеДанных",
        ));
    }
    Ok(())
}

fn settings_table(storage: ClientSettingsStorage) -> RtResult<&'static str> {
    match storage {
        ClientSettingsStorage::Common => Ok("_commonsettings"),
        ClientSettingsStorage::System => Ok("_systemsettings"),
        _ => Err(error(
            "чтение этого хранилища настроек не поддерживается адаптером базы",
        )),
    }
}

fn one_payload(rows: Vec<Vec<Cell>>) -> RtResult<Option<Vec<u8>>> {
    if rows.is_empty() {
        return Ok(None);
    }
    let [row]: [Vec<Cell>; 1] = rows
        .try_into()
        .map_err(|_| error("общая настройка неоднозначна в текущей области"))?;
    let [cell]: [Cell; 1] = row
        .try_into()
        .map_err(|_| error("неверная ширина строки настройки"))?;
    let Cell::Bytes(bytes) = cell else {
        return Err(error("общая настройка не содержит двоичного значения"));
    };
    Ok(Some(bytes))
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "settings_rights_tests.rs"]
mod rights_tests;
