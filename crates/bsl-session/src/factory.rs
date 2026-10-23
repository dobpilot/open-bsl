//! Фабрика сеансов: выбор пользователя, сборка исполняемого окружения и
//! инициализация параметров разделения.

use super::*;

/// Пользователь информационной базы до выбора principal.
#[derive(Debug, Clone)]
pub struct PrincipalCandidate {
    principal: Principal,
    can_authenticate: bool,
}

impl PrincipalCandidate {
    /// Создаёт кандидата из декодированной записи `v8users`.
    #[must_use]
    pub fn new(principal: Principal, can_authenticate: bool) -> Self {
        Self {
            principal,
            can_authenticate,
        }
    }

    /// Возвращает снимок пользователя.
    #[must_use]
    pub const fn principal(&self) -> &Principal {
        &self.principal
    }

    /// Разрешён ли хотя бы один способ аутентификации.
    #[must_use]
    pub const fn can_authenticate(&self) -> bool {
        self.can_authenticate
    }
}

/// Выбирает principal по имени или идентификатору.
///
/// При пустом `v8users` возвращает платформенного пустого пользователя с
/// главными ролями конфигурации. Из непустого списка участвуют только записи
/// с хотя бы одним способом аутентификации.
///
/// # Errors
///
/// Возвращает ошибку неоднозначного автоматического выбора, неизвестного
/// пользователя, явного выбора запрещённой записи или отсутствия допустимых
/// пользователей.
pub fn select_principal(
    candidates: &[PrincipalCandidate],
    selection: Option<&str>,
    main_roles: impl IntoIterator<Item = String>,
) -> Result<Principal, PrincipalSelectionError> {
    if candidates.is_empty() {
        if let Some(selection) = selection {
            return Err(PrincipalSelectionError::Unknown(selection.into()));
        }
        return Ok(Principal::new("", "", main_roles));
    }
    if let Some(selection) = selection {
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate.principal.id() == selection
                    || folded_eq(candidate.principal.name(), selection)
            })
            .ok_or_else(|| PrincipalSelectionError::Unknown(selection.into()))?;
        if !candidate.can_authenticate {
            return Err(PrincipalSelectionError::Forbidden(
                candidate.principal.name().into(),
            ));
        }
        return Ok(candidate.principal.clone());
    }

    let eligible = candidates
        .iter()
        .filter(|candidate| candidate.can_authenticate)
        .collect::<Vec<_>>();
    match eligible.as_slice() {
        [] => Err(PrincipalSelectionError::NoSelectableUsers),
        [candidate] => Ok(candidate.principal.clone()),
        _ => Err(PrincipalSelectionError::Ambiguous(
            eligible
                .iter()
                .map(|candidate| candidate.principal.name().to_string())
                .collect(),
        )),
    }
}

/// Ошибка выбора пользователя информационной базы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalSelectionError {
    /// Имя или идентификатор не найдены.
    Unknown(String),
    /// Запись существует, но не имеет разрешённого способа аутентификации.
    Forbidden(String),
    /// В непустом `v8users` нет допустимых пользователей.
    NoSelectableUsers,
    /// Для автоматического выбора найдено несколько пользователей.
    Ambiguous(Vec<String>),
}

impl fmt::Display for PrincipalSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(formatter, "пользователь `{value}` не найден"),
            Self::Forbidden(value) => write!(
                formatter,
                "пользователю `{value}` не разрешён вход в информационную базу"
            ),
            Self::NoSelectableUsers => {
                formatter.write_str("в информационной базе нет пользователей с разрешённым входом")
            }
            Self::Ambiguous(names) => write!(
                formatter,
                "требуется выбрать пользователя: {}",
                names.join(", ")
            ),
        }
    }
}

impl Error for PrincipalSelectionError {}

/// Скомпилированная конфигурация, из которой открываются изолированные сеансы.
#[derive(Clone)]
pub struct SessionFactory {
    pub(crate) engine: Engine,
    pub(crate) catalog: Arc<MetadataCatalog>,
    session_module: String,
    parameter_names: Arc<[String]>,
    pub(crate) separation_objects: Arc<[usize]>,
    pub(crate) kind: SessionKind,
    platform_version: Arc<str>,
    pub(crate) connection_string: Option<Arc<str>>,
}

impl SessionFactory {
    /// Проверяет совместимость кода и метаданных и один раз компилирует
    /// конфигурационный граф для указанного вида сеанса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку пустой версии платформы, несовпадающей идентичности,
    /// отсутствующего объекта владельца или модуля сеанса, а также ошибку
    /// сборки open-bsl.
    pub fn new(
        code: ConfigurationCodeSnapshot,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
    ) -> Result<Self, SessionError> {
        Self::new_with_entry_sources(
            code,
            catalog,
            session_module,
            kind,
            platform_version,
            std::iter::empty::<String>(),
        )
    }

    /// Компилирует фабрику с зависимостями известных входных BSL-текстов.
    /// Тексты участвуют только в выборе модулей и не исполняются при создании
    /// фабрики. Входы не добавляются в исходник модуля сеанса.
    ///
    /// # Errors
    /// Возвращает ошибки [`Self::new`], а также синтаксические и семантические
    /// ошибки объявленных входных текстов при выборе замыкания.
    pub fn new_with_entry_sources(
        code: ConfigurationCodeSnapshot,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
        entry_sources: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, SessionError> {
        Self::build_source(
            Arc::new(open_bsl::SnapshotSource::new(code)),
            catalog,
            session_module,
            kind,
            platform_version,
            entry_sources,
            false,
        )
    }

    /// Создаёт фабрику из индекса с подготовкой исходников по обращению.
    /// В начале подготавливаются модуль сеанса, входные тексты и их зависимости.
    ///
    /// # Errors
    /// Возвращает ошибки идентичности, объявлений, загрузки и подготовки
    /// достигнутых модулей либо необходимых метаданных.
    pub fn new_with_source(
        source: Arc<dyn open_bsl::ConfigurationSource>,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
        entry_sources: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, SessionError> {
        Self::build_source(
            source,
            catalog,
            session_module,
            kind,
            platform_version,
            entry_sources,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_source(
        source: Arc<dyn open_bsl::ConfigurationSource>,
        catalog: Arc<MetadataCatalog>,
        session_module: &str,
        kind: SessionKind,
        platform_version: impl Into<String>,
        entry_sources: impl IntoIterator<Item = impl Into<String>>,
        on_demand: bool,
    ) -> Result<Self, SessionError> {
        let code = source.index();
        let platform_version = platform_version.into();
        if platform_version.is_empty() {
            return Err(SessionError::EmptyPlatformVersion);
        }
        if code.identity() != catalog.identity() {
            return Err(SessionError::ConfigurationIdentityMismatch {
                code: code.identity().into(),
                metadata: catalog.identity().into(),
            });
        }
        for module in code
            .modules()
            .iter()
            .filter(|module| module.kind() == ModuleKind::Manager)
        {
            let owner = module
                .owner()
                .ok_or_else(|| SessionError::ManagerWithoutOwner {
                    module: module.name().into(),
                })?;
            if catalog.object_by_id(owner).is_none() {
                return Err(SessionError::MissingManagerOwner {
                    module: module.name().into(),
                    owner: owner.into(),
                });
            }
        }
        let session_module = code
            .modules()
            .iter()
            .find(|module| {
                module.kind() == ModuleKind::Session && folded_eq(module.name(), session_module)
            })
            .map(|module| module.name().to_string())
            .ok_or_else(|| SessionError::MissingSessionModule(session_module.into()))?;

        let modules = code
            .modules()
            .iter()
            .filter(|module| {
                module.kind() != ModuleKind::Form
                    && (module.kind() != ModuleKind::Common || module.execution().server())
            })
            .map(|module| {
                let mut imports = Vec::new();
                for import in module.imports() {
                    let target = code.module(import.module()).ok_or_else(|| {
                        SessionError::MissingImportedModule {
                            module: module.name().into(),
                            imported: import.module().into(),
                        }
                    })?;
                    if target.kind() != ModuleKind::Common || target.execution().server() {
                        imports.push((import.alias().to_string(), target.name().to_string()));
                    }
                }
                Ok(ModuleRecipe {
                    visibility: match module.kind() {
                        ModuleKind::Common if module.execution().global() => {
                            open_bsl::ModuleVisibility::Global
                        }
                        ModuleKind::Common => open_bsl::ModuleVisibility::Named,
                        // Модуль сеанса закрыт для BSL-импортов; хост
                        // вызывает обработчик в каталожном экземпляре `State`.
                        ModuleKind::Session => open_bsl::ModuleVisibility::Explicit,
                        _ => open_bsl::ModuleVisibility::Private,
                    },
                    name: module.name().into(),
                    source: if on_demand {
                        String::new()
                    } else {
                        source
                            .module_source(module.id())
                            .map_err(open_bsl::Error::Configuration)?
                            .to_string()
                    },
                    imports,
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        // В индексном режиме динамические имена не становятся корнями:
        // их реальная цель подготавливается при первом обращении.
        let mut roots = vec![session_module.clone()];
        if !on_demand {
            roots.extend(
                modules
                    .iter()
                    .filter(|module| {
                        matches!(
                            module.visibility,
                            open_bsl::ModuleVisibility::Named | open_bsl::ModuleVisibility::Global
                        )
                    })
                    .map(|module| module.name.clone()),
            );
        }
        let mut builder = Engine::builder()
            .defer_common_module_name_errors(true)
            .register_library(bsl_orm::library())
            .register_library(library())
            // Фабрика исполняет модуль сеанса на сервере. Дефолтный
            // standalone-контекст фасада дополнительно включает внешнее
            // соединение, поэтому его ветви здесь нужно явно выключить.
            .preproc_symbol("ВнешнееСоединение", false)
            .configuration_roots(roots)
            .configuration_entry_sources(entry_sources);
        if !on_demand {
            builder = builder.configuration(ModuleGraphRecipe {
                modules,
                eager_init: false,
            });
        }
        if on_demand {
            builder = builder
                .configuration_source(source.clone())
                .configuration_context_source(Arc::new(source_context::SessionModuleContext::new(
                    source.index().clone(),
                    catalog.clone(),
                )));
        }
        for ty in catalog.transferable_value_types() {
            builder = builder.register_value_type(ty);
        }
        for ty in catalog.route_point_reference_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.catalog_selection_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.catalog_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.document_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog
            .data_processor_object_types()
            .chain(catalog.data_processor_tabular_section_types())
            .chain(catalog.report_object_types())
            .chain(catalog.report_tabular_section_types())
        {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.business_process_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.characteristic_object_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.information_record_set_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.document_tabular_section_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for ty in catalog.business_process_tabular_section_types() {
            builder = builder.register_value_type(Arc::clone(ty));
        }
        for object in catalog.objects() {
            if object.kind() == MetadataKind::Catalog
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module("Справочники", object.name(), module)
                    .configuration_property_names(
                        module,
                        bsl_orm::catalog_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .catalog_attribute_names()
                                    .into_iter()
                                    .flatten()
                                    .cloned(),
                            )
                            .chain(
                                object
                                    .catalog_common_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            )
                            .chain(
                                object
                                    .catalog_tabular_section_names()
                                    .into_iter()
                                    .flatten()
                                    .cloned(),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::catalog_object_receiver_methods(),
                    );
            }
            if object.kind() == MetadataKind::Document
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module("Документы", object.name(), module)
                    .configuration_property_names(
                        module,
                        bsl_orm::document_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .document_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            )
                            .chain(
                                object
                                    .document_tabular_sections()
                                    .into_iter()
                                    .flatten()
                                    .map(|section| section.name().to_owned()),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::document_object_receiver_methods(),
                    );
            }
            if object.kind() == MetadataKind::BusinessProcess
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module("БизнесПроцессы", object.name(), module)
                    .configuration_property_names(
                        module,
                        bsl_orm::business_process_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .business_process_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned())
                                    .chain(
                                        object
                                            .business_process_tabular_sections()
                                            .into_iter()
                                            .flatten()
                                            .map(|section| section.name().to_owned()),
                                    ),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::business_process_object_receiver_methods(),
                    );
            }
            if matches!(
                object.kind(),
                MetadataKind::DataProcessor | MetadataKind::Report
            ) && let Some(module) = object.object_module()
            {
                let (global, attributes, sections) = if object.kind() == MetadataKind::Report {
                    (
                        "Отчеты",
                        object.report_attributes(),
                        object.report_tabular_sections(),
                    )
                } else {
                    (
                        "Обработки",
                        object.data_processor_attributes(),
                        object.data_processor_tabular_sections(),
                    )
                };
                let properties = if object.kind() == MetadataKind::Report {
                    bsl_orm::report_object_receiver_properties()
                } else {
                    &[]
                };
                builder = builder
                    .configuration_component_module(global, object.name(), module)
                    .configuration_property_names(
                        module,
                        properties
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                attributes
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            )
                            .chain(
                                sections
                                    .into_iter()
                                    .flatten()
                                    .map(|section| section.name().to_owned()),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::data_processor_object_receiver_methods(),
                    );
            }
            if object.kind() == MetadataKind::ChartOfCharacteristicTypes
                && let Some(module) = object.object_module()
            {
                builder = builder
                    .configuration_component_module(
                        "ПланыВидовХарактеристик",
                        object.name(),
                        module,
                    )
                    .configuration_property_names(
                        module,
                        bsl_orm::characteristic_object_receiver_properties()
                            .iter()
                            .flat_map(|property| property.names.iter().copied())
                            .map(str::to_owned)
                            .chain(
                                object
                                    .characteristic_attributes()
                                    .into_iter()
                                    .flatten()
                                    .map(|attribute| attribute.name().to_owned()),
                            ),
                    )
                    .configuration_method_context(
                        module,
                        bsl_orm::characteristic_object_receiver_methods(),
                    );
            }
            if let Some(module) = object.manager_module() {
                let global = object.kind().collection_name().ok_or_else(|| {
                    open_bsl::Error::Configuration(format!(
                        "объект «{}» не имеет коллекции менеджеров",
                        object.name()
                    ))
                })?;
                builder = builder.configuration_component_module(global, object.name(), module);
                if matches!(
                    object.kind(),
                    MetadataKind::Catalog
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfAccounts
                ) {
                    // Предопределённые имена являются свойствами собственного
                    // менеджера. Переменные сохраняют обычный приоритет ядра.
                    builder = builder.configuration_property_names(
                        module,
                        object.catalog_predefined_names().iter().cloned(),
                    );
                }
                if object.kind() == MetadataKind::BusinessProcess {
                    builder = builder
                        .configuration_property_names(module, ["ТочкиМаршрута", "RoutePoints"]);
                }
                let methods = object.kind().receiver_methods();
                if !methods.is_empty() {
                    builder = builder.configuration_method_context(module, methods);
                }
            }
        }
        let engine = builder.build()?;
        let parameter_names = catalog
            .objects()
            .iter()
            .filter(|object| object.kind() == MetadataKind::SessionParameter)
            .map(|object| object.name().to_string())
            .collect::<Vec<_>>()
            .into();
        // Состав каталога неизменяем. Обход всех объектов оплачивает фабрика,
        // а каждый новый сеанс посещает только связанные разделители.
        let separation_objects = catalog
            .objects()
            .iter()
            .enumerate()
            .filter_map(|(index, object)| {
                object
                    .common_attribute_separation_parameters()
                    .map(|_| index)
            })
            .collect::<Vec<_>>()
            .into();
        Ok(Self {
            engine,
            catalog,
            session_module,
            parameter_names,
            separation_objects,
            kind,
            platform_version: platform_version.into(),
            connection_string: None,
        })
    }

    /// Выбирает вид создаваемых сеансов с повторным использованием
    /// скомпилированного серверного контекста.
    /// Уже открытые сеансы сохраняют свой вид и состояние.
    #[must_use]
    pub fn with_kind(mut self, kind: SessionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Задаёт строку регистрации информационной базы для создаваемых сеансов.
    /// Строка сохраняется без разбора и преобразования.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если передана пустая строка.
    pub fn with_connection_string(
        mut self,
        connection_string: impl Into<String>,
    ) -> Result<Self, SessionError> {
        let connection_string = connection_string.into();
        if connection_string.is_empty() {
            return Err(SessionError::EmptyConnectionString);
        }
        self.connection_string = Some(Arc::from(connection_string));
        Ok(self)
    }

    /// Открывает изолированный сеанс на уже созданном отдельном соединении.
    /// Соединение передаётся во владение и не может быть унаследовано другим
    /// сеансом случайным клонированием.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку недопустимого номера соединения, создания контейнера,
    /// инициализации модуля или стартового
    /// `УстановкаПараметровСеанса(Неопределено)`. Частичный [`Session`] при
    /// ошибке не возвращается.
    pub fn open(
        &self,
        principal: Principal,
        resources: SessionResources,
    ) -> Result<Session, SessionError> {
        self.open_with_resources(principal, resources, None, None)
    }

    /// Открывает фоновый сеанс с копией параметров родителя.
    /// Последующие изменения контейнеров независимы.
    ///
    /// # Errors
    ///
    /// Те же ошибки, что у [`SessionFactory::open`]. Фабрика обязана быть
    /// собрана для [`SessionKind::BackgroundJob`].
    pub fn open_background(
        &self,
        principal: Principal,
        resources: SessionResources,
        method_name: &str,
        parent: &Session,
    ) -> Result<Session, SessionError> {
        if self.kind != SessionKind::BackgroundJob {
            return Err(SessionError::NotBackgroundFactory);
        }
        if method_name.is_empty() {
            return Err(SessionError::EmptyBackgroundMethod);
        }
        self.open_with_resources(
            principal,
            resources,
            Some(Rc::new(parent.parameters.copy_for_background())),
            Some(Arc::from(method_name)),
        )
    }

    fn open_with_resources(
        &self,
        principal: Principal,
        resources: SessionResources,
        parameters: Option<Rc<SessionParameters>>,
        background_method: Option<Arc<str>>,
    ) -> Result<Session, SessionError> {
        let SessionResources {
            connection,
            connection_number,
            arguments,
            session_number,
            session_started,
            data_access,
            connections,
            event_log_writer,
            client_settings,
            error_processing_settings,
            database_configuration_changed_dynamically,
            data_separation_common_attributes,
            client_displays,
            client_connection_speed,
            current_client_interface_variant,
            computer_name,
            locale_code,
            infobase_locale_code,
            system_language,
            external_resources_mode,
        } = resources;
        if connection_number == 0 || connection_number > i64::MAX as u64 {
            return Err(SessionError::InvalidConnectionNumber(connection_number));
        }
        if let Some(variant) = current_client_interface_variant
            && !matches!(
                variant,
                EnumValue::ClientApplicationInterfaceVariantTaxi
                    | EnumValue::ClientApplicationInterfaceVariantVersion8_2
            )
        {
            return Err(SessionError::InvalidClientInterfaceVariant);
        }
        if let Some(number) = session_number
            && (number == 0 || number > i64::MAX as u64)
        {
            return Err(SessionError::InvalidSessionNumber(number));
        }
        if let Some(connections) = connections.as_deref() {
            for connection in connections {
                if connection.number == 0 || connection.number > i64::MAX as u64 {
                    return Err(SessionError::InvalidListedConnectionNumber(
                        connection.number,
                    ));
                }
            }
            if !connections
                .iter()
                .any(|connection| connection.number == connection_number)
            {
                return Err(SessionError::MissingCurrentConnection(connection_number));
            }
        }
        let parameters = match parameters {
            Some(parameters) => parameters,
            None => {
                let parameters = SessionParameters::new(
                    self.session_module.clone(),
                    self.parameter_names.iter().cloned(),
                )?;
                // Платформа заполняет связанные параметры разделителя до обработчика.
                // Обычные параметры остаются незаполненными; унаследованные сюда не попадают.
                initialize_separation_parameters(
                    &self.catalog,
                    &self.separation_objects,
                    &parameters,
                )?;
                Rc::new(parameters)
            }
        };
        let runtime = SessionRuntime {
            parameters: Rc::clone(&parameters),
            kind: self.kind,
            platform_version: Arc::clone(&self.platform_version),
            connection_string: self.connection_string.clone(),
            connection_number,
            session_number,
            session_started,
            connections,
            event_log_writer,
            client_settings,
            error_processing_settings,
            database_configuration_changed_dynamically,
            data_separation_common_attributes,
            client_displays: if self.kind == SessionKind::BackgroundJob {
                None
            } else {
                client_displays
            },
            client_connection_speed,
            current_client_interface_variant,
            computer_name,
            locale_code,
            infobase_locale_code,
            system_language,
            external_resources_mode,
            background_method,
        };
        let system_language = runtime.system_language.clone();
        let mut data = SessionData::new(Arc::clone(&self.catalog), principal.clone())
            .with_extension(Rc::new(runtime));
        if let Some(language) = system_language {
            data = data.with_system_language(language);
        }
        if let Some(data_access) = data_access {
            data = data.with_data_access(data_access);
        }
        let mut state = self
            .engine
            .state_builder()
            .arguments(arguments)
            .message_sink(Rc::new(messages::SessionMessages::default()))
            .host_data(Rc::new(data))
            .build();
        // НЕ ИЗМЕРЕНО(SESSION.INIT.WEB_PRIVILEGE): автоматический режим
        // измерен для тонкого клиента и фонового задания; веб-вход требует
        // отдельного нативного контроля в опубликованной базе.
        state.call_configuration_handler(
            &self.session_module,
            "УстановкаПараметровСеанса",
            vec![Value::Undefined],
            open_bsl::ConfigurationHandlerEntryMode::Privileged,
        )?;
        Ok(Session {
            engine: self.engine.clone(),
            state,
            parameters,
            principal,
            connection,
            kind: self.kind,
        })
    }
}

fn initialize_separation_parameters(
    catalog: &MetadataCatalog,
    objects: &[usize],
    parameters: &SessionParameters,
) -> Result<(), SessionError> {
    for &index in objects {
        let object = &catalog.objects()[index];
        let Some(settings) = object.common_attribute_separation_parameters() else {
            continue;
        };
        let invalid = || {
            SessionError::OpenBsl(OpenBslError::Runtime(RtError::DynamicError(format!(
                "начальное значение параметра разделителя `{}` неизвестно",
                object.name()
            ))))
        };
        if let Some(id) = &settings.use_parameter {
            let parameter = catalog.object_by_id(id).ok_or_else(invalid)?;
            parameters.set(parameter.name(), Value::Boolean(false))?;
        }
        if let Some(id) = &settings.value_parameter {
            let parameter = catalog.object_by_id(id).ok_or_else(invalid)?;
            let value = match settings.value_types.as_deref() {
                Some([kind]) if kind.id == open_bsl::TypeRef::Native(open_bsl::TypeId::String) => {
                    Value::Str("".into())
                }
                Some([kind]) if kind.id == open_bsl::TypeRef::Native(open_bsl::TypeId::Number) => {
                    Value::number_from_i64(0)
                }
                _ => return Err(invalid()),
            };
            parameters.set(parameter.name(), value)?;
        }
    }
    Ok(())
}
