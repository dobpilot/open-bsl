//! Индекс метаданных конфигурации с необязательной адресной загрузкой подробностей.

use super::*;

type MetadataCollectionKey = (std::mem::Discriminant<MetadataKind>, Option<Arc<str>>);

/// Индекс метаданных с необязательной адресной загрузкой подробностей.
#[derive(Debug)]
pub struct MetadataCatalog {
    pub(crate) identity: Arc<str>,
    pub(crate) objects: Arc<[MetadataObject]>,
    object_index: HashMap<Arc<str>, usize>,
    pub(crate) collection_positions: HashMap<MetadataCollectionKey, Vec<usize>>,
    pub(crate) common_attribute_positions: Vec<usize>,
    object_source: Option<Arc<dyn MetadataObjectSource>>,
    pub(crate) file_source: Option<file_source::Attached>,
    pub(crate) file_validations: Vec<std::sync::OnceLock<Result<(), String>>>,
    object_details: Vec<std::sync::OnceLock<Result<MetadataObject, String>>>,
    common_attribute_ids: Option<Arc<[String]>>,
    pub(crate) configuration: Option<ConfigurationDescription>,
    pub(crate) configuration_object_id: Option<Arc<str>>,
    pub(crate) configuration_extensions: Option<Arc<ConfigurationExtensionsSnapshot>>,
    pub(crate) configuration_changed: Option<bool>,
    pub(crate) default_interface_id: Option<[u8; 16]>,
    pub(crate) enumeration_values: Arc<[ConfigurationEnumValue]>,
    pub(crate) reference_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) catalog_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) document_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) business_process_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) data_processor_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) report_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) report_section_types: Vec<document_tabular_section::Types>,
    pub(crate) data_processor_section_types: Vec<document_tabular_section::Types>,
    pub(crate) characteristic_object_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) document_section_types: Vec<document_tabular_section::Types>,
    pub(crate) business_section_types: Vec<document_tabular_section::Types>,
    pub(crate) information_record_set_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
    pub(crate) catalog_selection_types: Vec<(Arc<str>, Arc<ValueTypeDescriptor>)>,
}

#[derive(Debug)]
pub(crate) struct ConfigurationDescription {
    pub(crate) name: Arc<str>,
    pub(crate) version: Arc<str>,
    pub(crate) script_variant: ScriptVariant,
    pub(crate) compatibility_mode: CompatibilityMode,
    pub(crate) main_role_ids: Arc<[Arc<str>]>,
    primary_language_id: Option<Arc<str>>,
}

impl MetadataCatalog {
    /// Присоединяет типы объекта, приобретённые из авторитетного описания БД.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца.
    pub fn with_object_declared_types(
        mut self,
        owner: &str,
        types: Vec<open_bsl::ColumnType>,
    ) -> Result<Self, String> {
        let index = self
            .object_index
            .get(owner)
            .copied()
            .ok_or_else(|| format!("владелец типов «{owner}» отсутствует"))?;
        Arc::make_mut(&mut self.objects)[index].declared_types = Some(types.into());
        Ok(self)
    }

    /// Присоединяет полный приобретённый состав собственных реквизитов плана обмена.
    /// `None` сохраняет неизвестность; пустой список подтверждает отсутствие реквизитов.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_exchange_plan_attributes(
        mut self,
        owner: &str,
        attributes: Option<Vec<DocumentAttribute>>,
    ) -> Result<Self, String> {
        let index = self
            .object_index
            .get(owner)
            .copied()
            .filter(|&i| self.objects[i].kind == MetadataKind::ExchangePlan)
            .ok_or_else(|| format!("план обмена «{owner}» отсутствует"))?;
        Arc::make_mut(&mut self.objects)[index].exchange_plan_attributes =
            attributes.map(Into::into);
        Ok(self)
    }

    /// Приобретает метаданные достигнутой группы через источник подробностей.
    /// Корневой индекс и членство коллекций не меняются; BSL не исполняется.
    /// Для полностью приобретённого каталога дополнительной работы нет.
    ///
    /// # Errors
    /// Возвращает ошибку источника подробностей или их зависимостей.
    pub fn prepare_objects(&self, ids: &[&str]) -> Result<(), String> {
        if let Some(source) = &self.object_source {
            source.prepare_objects(ids)?;
        }
        Ok(())
    }

    /// Присоединяет приобретённые настройки и собственные реквизиты плана видов характеристик.
    /// Неизвестные данные сохраняются как `None`, известный пустой состав — как `Some([])`.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или объекта другого вида.
    pub fn with_characteristic_metadata(
        mut self,
        owner: &str,
        settings: Option<CharacteristicSettings>,
        attributes: Option<Vec<DocumentAttribute>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::ChartOfCharacteristicTypes
            })
            .ok_or_else(|| CatalogError::UnknownCharacteristicOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.characteristic_settings = settings.map(Arc::new);
        object.characteristic_attributes = attributes.map(Into::into);
        Ok(self)
    }

    /// Сохраняет полный корневой список общих реквизитов конфигурации.
    /// Пустой список означает известное отсутствие общих реквизитов.
    #[must_use]
    pub fn with_common_attribute_ids(mut self, ids: Vec<String>) -> Self {
        self.common_attribute_positions = ids
            .iter()
            .filter_map(|id| self.object_index.get(id.as_str()).copied())
            .collect();
        self.common_attribute_ids = Some(ids.into());
        self
    }

    /// Возвращает полный корневой список; `None` означает неизвестный состав.
    #[must_use]
    pub fn common_attribute_ids(&self) -> Option<&[String]> {
        self.common_attribute_ids.as_deref()
    }

    /// Присоединяет приобретённые связи параметров разделяющего общего реквизита.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего или неразделяющего общего реквизита либо связи,
    /// которая не указывает на объявленный параметр сеанса.
    pub fn with_common_attribute_separation_parameters(
        mut self,
        owner: &str,
        settings: CommonAttributeSeparationParameters,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner
                    && object.kind() == MetadataKind::CommonAttribute
                    && object.common_attribute_data_separation == Some(true)
            })
            .ok_or_else(|| CatalogError::InvalidCommonAttributeSeparation(owner.into()))?;
        for parameter in [&settings.value_parameter, &settings.use_parameter]
            .into_iter()
            .flatten()
        {
            if !self
                .object_by_id(parameter)
                .is_some_and(|object| object.kind() == MetadataKind::SessionParameter)
            {
                return Err(CatalogError::InvalidCommonAttributeSeparation(owner.into()));
            }
        }
        Arc::make_mut(&mut self.objects)[index].common_attribute_separation_parameters =
            Some(Arc::new(settings));
        Ok(self)
    }

    /// Присоединяет полный применимый состав общих реквизитов справочника.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего объекта или объекта другого вида.
    pub fn with_catalog_common_attributes(
        mut self,
        owner: &str,
        attributes: Vec<DocumentAttribute>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Catalog)
            .ok_or_else(|| CatalogError::UnknownCatalogOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].catalog_common_attributes = Some(attributes.into());
        Ok(self)
    }

    /// Присоединяет полные собственные реквизиты приобретённого справочника.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего объекта или объекта другого вида.
    pub fn with_catalog_attributes(
        mut self,
        owner: &str,
        attributes: Vec<DocumentAttribute>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Catalog)
            .ok_or_else(|| CatalogError::UnknownCatalogOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].catalog_attributes = Some(attributes.into());
        Ok(self)
    }

    /// Неизменяемые описания типов изменяемых объектов справочников.
    /// У них нет декодера графа: объявление типа не разрешает перенос
    /// переменных модуля объекта или данных живого сеанса.
    pub fn catalog_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.catalog_object_types.iter().map(|(_, ty)| ty)
    }

    /// Типы объектов документов для регистрации без данных живого сеанса.
    pub fn document_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.document_object_types.iter().map(|(_, ty)| ty)
    }

    /// Неизменяемые типы объектов бизнес-процессов для регистрации в движке.
    /// Регистрация описания не разрешает перенос состояния экземпляра.
    pub fn business_process_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.business_process_object_types.iter().map(|(_, ty)| ty)
    }

    /// Типы объектов обработки; описание не разрешает перенос памяти экземпляра.
    pub fn data_processor_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.data_processor_object_types.iter().map(|(_, ty)| ty)
    }

    /// Неизменяемые описания типов объектов отчётов без памяти экземпляров.
    pub fn report_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.report_object_types.iter().map(|(_, ty)| ty)
    }

    /// Описания типов собственных табличных частей отчётов и их строк.
    pub fn report_tabular_section_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.report_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Типы собственных частей обработки и их строк без изменяемых данных сеанса.
    pub fn data_processor_tabular_section_types(
        &self,
    ) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.data_processor_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Типы объектов планов видов характеристик без данных живого сеанса.
    pub fn characteristic_object_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.characteristic_object_types.iter().map(|(_, ty)| ty)
    }

    /// Типы собственных наборов записей регистров сведений.
    /// Регистрация не разрешает перенос строк или живого сеанса через граф.
    pub fn information_record_set_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.information_record_set_types.iter().map(|(_, ty)| ty)
    }

    /// Типы приобретённых табличных частей документов и их строк.
    /// Описания разделяются сеансами; строки и значения в них не хранятся.
    pub fn document_tabular_section_types(
        &self,
    ) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.document_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Типы приобретённых табличных частей бизнес-процессов и их строк.
    /// Описания не содержат изменяемых значений сеанса.
    pub fn business_process_tabular_section_types(
        &self,
    ) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.business_section_types
            .iter()
            .flat_map(|types| [&types.section_type, &types.row_type])
    }

    /// Неизменяемые типы ссылок для регистрации в движке конфигурации.
    /// Описания создаются один раз вместе с каталогом и разделяются сеансами.
    pub fn value_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.reference_types
            .iter()
            .map(|(_, description)| description)
    }

    /// Типы выборок конкретных справочников для регистрации в движке.
    /// Изменяемые курсоры не имеют декодера графа значений.
    pub fn catalog_selection_types(&self) -> impl Iterator<Item = &Arc<ValueTypeDescriptor>> {
        self.catalog_selection_types.iter().map(|(_, ty)| ty)
    }

    /// Типы ссылок с декодерами для движка и графа неизменяемых значений.
    /// Декодеры удерживают только исходный каталог, не образуя цикл с описаниями.
    pub fn transferable_value_types(
        self: &Arc<Self>,
    ) -> impl Iterator<Item = Arc<ValueTypeDescriptor>> + '_ {
        self.reference_types.iter().map(|(id, ty)| {
            let catalog = Arc::clone(self);
            let id = Arc::clone(id);
            Arc::new(
                ty.as_ref()
                    .clone()
                    .with_materializer(Arc::new(move |bytes| {
                        if catalog
                            .object_by_id(&id)
                            .is_some_and(|object| object.kind == MetadataKind::Enumeration)
                        {
                            let member = std::str::from_utf8(bytes).map_err(|_| {
                                RtError::DynamicError(
                                    "повреждён идентификатор значения перечисления".into(),
                                )
                            })?;
                            return enumeration_reference_value(Arc::clone(&catalog), &id, member);
                        }
                        let uuid = bytes.try_into().map_err(|_| {
                            RtError::DynamicError("длина UUID ссылки не равна 16".into())
                        })?;
                        configuration_reference_value(Arc::clone(&catalog), &id, uuid)
                    })),
            )
        })
    }

    /// Проверяет и создаёт каталог.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустой идентичности, пустых идентификаторов и
    /// имён, повторных идентификаторов или повторных имён одного вида.
    pub fn new(
        identity: impl Into<String>,
        objects: Vec<MetadataObject>,
    ) -> Result<Self, CatalogError> {
        let identity = identity.into();
        if identity.is_empty() {
            return Err(CatalogError::EmptyConfigurationIdentity);
        }
        let mut ids = HashSet::with_capacity(objects.len());
        let mut names = open_bsl::NameInterner::new();
        let mut object_names = HashSet::with_capacity(objects.len());
        for object in &objects {
            if object.id.is_empty() {
                return Err(CatalogError::EmptyObjectIdentity);
            }
            if object.name.is_empty() {
                return Err(CatalogError::EmptyObjectName {
                    object: object.id.to_string(),
                });
            }
            if !ids.insert(object.id.as_ref()) {
                return Err(CatalogError::DuplicateObjectIdentity {
                    object: object.id.to_string(),
                });
            }
            // Интернер использует тот же `folded_eq`, включая проверку
            // коллизий: проверка имён не требует попарного обхода каталога.
            let parent = if object.kind == MetadataKind::Subsystem {
                object.subsystem_parent.as_deref()
            } else {
                None
            };
            let name = names.intern(&object.name);
            if !object_names.insert((std::mem::discriminant(&object.kind), parent, name)) {
                return Err(CatalogError::DuplicateObjectName {
                    name: object.name.to_string(),
                });
            }
            if object.manager_module.as_deref() == Some("") {
                return Err(CatalogError::EmptyManagerModule {
                    object: object.id.to_string(),
                });
            }
            if object.object_module.as_deref() == Some("") {
                return Err(CatalogError::EmptyObjectModule {
                    object: object.id.to_string(),
                });
            }
            for (property_index, (name, _)) in object.string_properties.iter().enumerate() {
                if name.is_empty() {
                    return Err(CatalogError::EmptyPropertyName {
                        object: object.id.to_string(),
                    });
                }
                if object.string_properties[..property_index]
                    .iter()
                    .any(|(known, _)| folded_eq(known, name))
                {
                    return Err(CatalogError::DuplicatePropertyName {
                        object: object.id.to_string(),
                        property: name.to_string(),
                    });
                }
            }
        }
        for object in &objects {
            if let Some(parent) = &object.subsystem_parent
                && (object.kind != MetadataKind::Subsystem
                    || !objects.iter().any(|candidate| {
                        candidate.id == *parent && candidate.kind == MetadataKind::Subsystem
                    }))
            {
                return Err(CatalogError::UnknownSubsystemParent(parent.to_string()));
            }
        }
        let reference_types = objects
            .iter()
            .filter(|object| {
                matches!(
                    object.kind,
                    MetadataKind::Catalog
                        | MetadataKind::Document
                        | MetadataKind::Enumeration
                        | MetadataKind::ChartOfCharacteristicTypes
                        | MetadataKind::ChartOfCalculationTypes
                        | MetadataKind::ChartOfAccounts
                        | MetadataKind::BusinessProcess
                        | MetadataKind::Task
                        | MetadataKind::ExchangePlan
                )
            })
            .map(|object| {
                let (identity_kind, russian, english) = if object.kind == MetadataKind::Document {
                    ("document-ref", "ДокументСсылка", "DocumentRef")
                } else if object.kind == MetadataKind::Enumeration {
                    ("enum-ref", "ПеречислениеСсылка", "EnumRef")
                } else if object.kind == MetadataKind::ChartOfCharacteristicTypes {
                    (
                        "characteristic-ref",
                        "ПланВидовХарактеристикСсылка",
                        "ChartOfCharacteristicTypesRef",
                    )
                } else if object.kind == MetadataKind::ChartOfCalculationTypes {
                    (
                        "calculation-type-ref",
                        "ПланВидовРасчетаСсылка",
                        "ChartOfCalculationTypesRef",
                    )
                } else if object.kind == MetadataKind::ChartOfAccounts {
                    (
                        "chart-of-accounts-ref",
                        "ПланСчетовСсылка",
                        "ChartOfAccountsRef",
                    )
                } else if object.kind == MetadataKind::BusinessProcess {
                    (
                        "business-process-ref",
                        "БизнесПроцессСсылка",
                        "BusinessProcessRef",
                    )
                } else if object.kind == MetadataKind::Task {
                    ("task-ref", "ЗадачаСсылка", "TaskRef")
                } else if object.kind == MetadataKind::ExchangePlan {
                    ("exchange-plan-ref", "ПланОбменаСсылка", "ExchangePlanRef")
                } else {
                    ("catalog-ref", "СправочникСсылка", "CatalogRef")
                };
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/{identity_kind}/{}", object.id),
                        object
                            .reference_type_presentation
                            .as_deref()
                            .unwrap_or(&object.name)
                            .to_owned(),
                        [
                            format!("{russian}.{}", object.name),
                            format!("{english}.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let catalog_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Catalog)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/catalog-object/{}", object.id),
                        format!("Справочник объект: {}", object.name),
                        [
                            format!("СправочникОбъект.{}", object.name),
                            format!("CatalogObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let document_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Document)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/document-object/{}", object.id),
                        format!("Документ объект: {}", object.name),
                        [
                            format!("ДокументОбъект.{}", object.name),
                            format!("DocumentObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let data_processor_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::DataProcessor)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/data-processor-object/{}", object.id),
                        format!("Обработка объект: {}", object.name),
                        [
                            format!("ОбработкаОбъект.{}", object.name),
                            format!("DataProcessorObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let report_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Report)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/report-object/{}", object.id),
                        format!("Report Object: {}", object.name),
                        [
                            format!("ОтчетОбъект.{}", object.name),
                            format!("ReportObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let report_section_types = document_tabular_section::report_types(&identity, &objects);
        let data_processor_section_types =
            document_tabular_section::processor_types(&identity, &objects);
        let business_process_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::BusinessProcess)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/business-process-object/{}", object.id),
                        // НЕ ИЗМЕРЕНО(TYPE.BUSINESS_PROCESS_OBJECT_PRESENTATION)
                        // Каноническое имя не подменяет измерение локали и синонима.
                        format!("БизнесПроцессОбъект.{}", object.name),
                        [
                            format!("БизнесПроцессОбъект.{}", object.name),
                            format!("BusinessProcessObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let characteristic_object_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::ChartOfCharacteristicTypes)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/characteristic-object/{}", object.id),
                        format!("План видов характеристик объект: {}", object.name),
                        [
                            format!("ПланВидовХарактеристикОбъект.{}", object.name),
                            format!("ChartOfCharacteristicTypesObject.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let catalog_selection_types = objects
            .iter()
            .filter(|object| object.kind == MetadataKind::Catalog)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/catalog-selection/{}", object.id),
                        format!("Справочник выборка: {}", object.name),
                        [
                            format!("СправочникВыборка.{}", object.name),
                            format!("CatalogSelection.{}", object.name),
                        ],
                    )),
                )
            })
            .collect();
        let document_section_types = document_tabular_section::types(&identity, &objects);
        let information_record_set_types = objects
            .iter()
            .filter(|object| object.kind() == MetadataKind::InformationRegister)
            .map(|object| {
                (
                    Arc::clone(&object.id),
                    Arc::new(ValueTypeDescriptor::new(
                        "bsl-orm",
                        format!("{identity}/information-record-set/{}", object.id()),
                        format!("Регистр сведений набор записей: {}", object.name()),
                        [
                            format!("РегистрСведенийНаборЗаписей.{}", object.name()),
                            format!("InformationRegisterRecordSet.{}", object.name()),
                        ],
                    )),
                )
            })
            .collect();
        let business_section_types = document_tabular_section::business_types(&identity, &objects);
        let object_index = objects
            .iter()
            .enumerate()
            .map(|(index, object)| (Arc::clone(&object.id), index))
            .collect();
        // Членство коллекций неизменно. Один проход сохраняет порядок каталога,
        // чтобы длина и доступ по индексу не фильтровали все объекты при каждом шаге.
        let mut collection_positions = HashMap::<_, Vec<usize>>::new();
        for (position, object) in objects.iter().enumerate() {
            let parent = if object.kind == MetadataKind::Subsystem {
                object.subsystem_parent.clone()
            } else {
                None
            };
            collection_positions
                .entry((std::mem::discriminant(&object.kind), parent))
                .or_default()
                .push(position);
        }
        Ok(Self {
            identity: identity.into(),
            objects: objects.into(),
            object_index,
            collection_positions,
            common_attribute_positions: Vec::new(),
            object_source: None,
            file_source: None,
            file_validations: Vec::new(),
            object_details: Vec::new(),
            common_attribute_ids: None,
            configuration: None,
            configuration_object_id: None,
            configuration_extensions: None,
            configuration_changed: None,
            default_interface_id: None,
            enumeration_values: Arc::from([]),
            reference_types,
            catalog_object_types,
            document_object_types,
            business_process_object_types,
            data_processor_object_types,
            report_object_types,
            report_section_types,
            data_processor_section_types,
            characteristic_object_types,
            business_section_types,
            document_section_types,
            information_record_set_types,
            catalog_selection_types,
        })
    }

    /// Присоединяет приобретённые реквизиты к документу до создания сеансов.
    /// Сохраняет остальные свойства каталога и идентичности его типов.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного владельца или владельца другого вида.
    pub fn with_document_attributes(
        mut self,
        owner: &str,
        attributes: Vec<DocumentAttribute>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Document)
            .ok_or_else(|| CatalogError::UnknownDocumentOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].document_attributes = Some(attributes.into());
        Ok(self)
    }

    /// Присоединяет приобретённые табличные части к собственному документу.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного владельца или владельца другого вида.
    pub fn with_document_tabular_sections(
        mut self,
        owner: &str,
        sections: Vec<DocumentTabularSection>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Document)
            .ok_or_else(|| CatalogError::UnknownDocumentOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].document_tabular_sections = Some(sections.into());
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.document_section_types,
        );
        Ok(self)
    }

    /// Присоединяет полные приобретённые коллекции обработки до создания сеансов.
    /// `None` означает неизвестность, пустой список — известное отсутствие.
    /// Сохраняет идентичности типов и обновляет производные типы строк и частей.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_data_processor_metadata(
        mut self,
        owner: &str,
        attributes: Option<Vec<DocumentAttribute>>,
        sections: Option<Vec<DocumentTabularSection>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::DataProcessor)
            .ok_or_else(|| CatalogError::UnknownDataProcessorOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.data_processor_attributes = attributes.map(Into::into);
        object.data_processor_sections = sections.map(Into::into);
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.data_processor_section_types,
        );
        Ok(self)
    }

    /// Присоединяет полные приобретённые коллекции отчёта до создания сеансов.
    /// `None` означает неизвестность, пустой список — проверенное отсутствие.
    /// Идентичности объектов сохраняются; производные типы частей обновляются.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_report_metadata(
        mut self,
        owner: &str,
        attributes: Option<Vec<DocumentAttribute>>,
        sections: Option<Vec<DocumentTabularSection>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == owner && object.kind() == MetadataKind::Report)
            .ok_or_else(|| CatalogError::UnknownReportOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.report_attributes = attributes.map(Into::into);
        object.report_sections = sections.map(Into::into);
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.report_section_types,
        );
        Ok(self)
    }

    /// Добавляет значения конфигурационных перечислений из того же снимка.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого идентификатора или имени, неизвестного
    /// владельца, владельца другого вида и повторного имени у владельца.
    pub fn with_enumeration_values(
        mut self,
        values: Vec<ConfigurationEnumValue>,
    ) -> Result<Self, CatalogError> {
        let mut names = open_bsl::NameInterner::new();
        let mut member_names = std::collections::HashSet::new();
        for value in &values {
            if value.value_id.is_empty() || value.name.is_empty() {
                return Err(CatalogError::InvalidEnumerationValue);
            }
            let owner = self
                .object_by_id(&value.owner_id)
                .filter(|owner| owner.kind == MetadataKind::Enumeration)
                .ok_or_else(|| CatalogError::UnknownEnumerationOwner(value.owner_id.to_string()))?;
            if !member_names.insert((owner.id.as_ref(), names.intern(&value.name))) {
                return Err(CatalogError::DuplicateEnumerationValue {
                    owner: owner.name.to_string(),
                    name: value.name.to_string(),
                });
            }
        }
        self.enumeration_values = values.into();
        Ok(self)
    }

    /// Присоединяет идентификатор корневого объекта метаданных конфигурации.
    /// Идентичность каталога служит пространством имён типов; права адресуют
    /// самостоятельный идентификатор объекта из приобретённых метаданных.
    ///
    /// # Errors
    /// Возвращает ошибку для пустого идентификатора.
    pub fn with_configuration_object_id(
        mut self,
        id: impl Into<String>,
    ) -> Result<Self, CatalogError> {
        let id = id.into();
        if id.is_empty() {
            return Err(CatalogError::EmptyConfigurationIdentity);
        }
        self.configuration_object_id = Some(id.into());
        Ok(self)
    }

    /// Возвращает приобретённый идентификатор корневого объекта конфигурации.
    /// Идентичность пространства имён каталога не заменяет этот идентификатор.
    #[must_use]
    pub fn configuration_object_id(&self) -> Option<&str> {
        self.configuration_object_id.as_deref()
    }

    /// Добавляет свойства корневого объекта метаданных конфигурации.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для пустого имени, неизвестной основной роли,
    /// объекта другого вида или повторной роли.
    pub fn with_configuration(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        script_variant: ScriptVariant,
        compatibility_mode: CompatibilityMode,
        main_role_ids: impl IntoIterator<Item = String>,
    ) -> Result<Self, CatalogError> {
        let name = name.into();
        if name.is_empty() {
            return Err(CatalogError::EmptyConfigurationName);
        }
        let mut roles = Vec::<Arc<str>>::new();
        for id in main_role_ids {
            let role = self
                .object_by_id(&id)
                .filter(|object| object.kind == MetadataKind::Role)
                .ok_or_else(|| CatalogError::UnknownMainRole(id.clone()))?;
            if roles.iter().any(|known| known.as_ref() == role.id()) {
                return Err(CatalogError::DuplicateMainRole(id));
            }
            roles.push(Arc::clone(&role.id));
        }
        self.configuration = Some(ConfigurationDescription {
            name: name.into(),
            version: Arc::from(version.into()),
            script_variant,
            compatibility_mode,
            main_role_ids: roles.into(),
            primary_language_id: None,
        });
        Ok(self)
    }

    /// Связывает свойства конфигурации с её основным языком.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если идентификатор отсутствует в каталоге или
    /// обозначает объект другого вида.
    pub fn with_primary_language(mut self, id: impl Into<String>) -> Result<Self, CatalogError> {
        let id = id.into();
        let language_id = Arc::clone(
            &self
                .object_by_id(&id)
                .filter(|object| object.kind == MetadataKind::Language)
                .ok_or_else(|| CatalogError::UnknownPrimaryLanguage(id.clone()))?
                .id,
        );
        let description = self
            .configuration
            .as_mut()
            .ok_or(CatalogError::UnknownPrimaryLanguage(id))?;
        description.primary_language_id = Some(language_id);
        Ok(self)
    }

    /// Возвращает идентификатор основного языка конфигурации.
    #[must_use]
    pub fn primary_language_id(&self) -> Option<&str> {
        self.configuration.as_ref()?.primary_language_id.as_deref()
    }

    /// Добавляет полный снимок расширений конфигурации текущего сеанса.
    #[must_use]
    pub fn with_configuration_extensions(
        mut self,
        extensions: ConfigurationExtensionsSnapshot,
    ) -> Self {
        self.configuration_extensions = Some(Arc::new(extensions));
        self
    }

    /// Добавляет признак различия основной конфигурации и конфигурации базы.
    #[must_use]
    pub const fn with_configuration_changed(mut self, changed: bool) -> Self {
        self.configuration_changed = Some(changed);
        self
    }

    /// Сохраняет приобретённый UUID основного интерфейса конфигурации.
    /// Нулевой UUID подтверждает отсутствие интерфейса. Без вызова этого
    /// метода настройка остаётся неизвестной. Чтение назначенного интерфейса
    /// пока завершается явной ошибкой неподдерживаемых метаданных.
    #[must_use]
    pub const fn with_default_interface_id(mut self, id: [u8; 16]) -> Self {
        self.default_interface_id = Some(id);
        self
    }

    #[must_use]
    /// Возвращает идентичность каталога, задающую пространство имён его типов.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    #[must_use]
    /// Возвращает объекты в порядке источника.
    pub fn objects(&self) -> &[MetadataObject] {
        &self.objects
    }

    /// Присоединяет источник подробностей к уже проверенному индексу.
    /// Повторные обращения всех сеансов используют один неизменяемый результат.
    #[must_use]
    pub fn with_object_source(mut self, source: Arc<dyn MetadataObjectSource>) -> Self {
        self.object_details = (0..self.objects.len())
            .map(|_| std::sync::OnceLock::new())
            .collect();
        self.object_source = Some(source);
        self
    }

    /// Возвращает подробности объекта, загружая их при первом обращении.
    /// `None` означает отсутствие идентификатора в индексе, а не ошибку чтения.
    ///
    /// # Errors
    /// Возвращает ошибку источника либо несовпадения подробностей с индексом.
    pub fn materialized_object_by_id(&self, id: &str) -> RtResult<Option<&MetadataObject>> {
        let Some(&index) = self.object_index.get(id) else {
            return Ok(None);
        };
        let header = &self.objects[index];
        let Some(source) = &self.object_source else {
            if let Some(attached) = &self.file_source {
                self.file_validations[index]
                    .get_or_init(|| file_source::validate_object(attached, header))
                    .as_ref()
                    .map_err(|error| RtError::DynamicError(error.clone()))?;
            }
            return Ok(Some(header));
        };
        self.object_details[index]
            .get_or_init(|| {
                let mut object = source.load_object(id)?;
                if object.id() != header.id()
                    || object.kind() != header.kind()
                    || !folded_eq(object.name(), header.name())
                {
                    return Err(format!(
                        "подробности объекта «{id}» не соответствуют индексу конфигурации"
                    ));
                }
                if let Some(attached) = &self.file_source {
                    file_source::validate_object(attached, &object)?;
                    file_source::bind_modules(attached, &mut object);
                }
                Ok(object)
            })
            .as_ref()
            .map(Some)
            .map_err(|message| RtError::DynamicError(message.clone()))
    }

    #[must_use]
    /// Находит объект по устойчивому идентификатору.
    pub fn object_by_id(&self, id: &str) -> Option<&MetadataObject> {
        let &index = self.object_index.get(id)?;
        self.object_details
            .get(index)
            .and_then(std::sync::OnceLock::get)
            .and_then(|result| result.as_ref().ok())
            .or_else(|| self.objects.get(index))
    }

    #[must_use]
    /// Находит объект указанного вида по регистронезависимому имени.
    pub fn object(&self, kind: MetadataKind, name: &str) -> Option<&MetadataObject> {
        self.objects
            .iter()
            .find(|object| object.kind == kind && folded_eq(&object.name, name))
    }

    /// Находит значение перечисления по владельцу и регистронезависимому имени.
    #[must_use]
    pub fn enumeration_value(&self, owner_id: &str, name: &str) -> Option<&ConfigurationEnumValue> {
        self.enumeration_values
            .iter()
            .find(|value| value.owner_id.as_ref() == owner_id && folded_eq(&value.name, name))
    }

    /// Возвращает идентификаторы основных ролей конфигурации.
    /// Пустой итератор также означает, что свойства конфигурации ещё не
    /// переданы загрузчиком.
    pub fn main_role_ids(&self) -> impl Iterator<Item = &str> {
        self.configuration
            .as_ref()
            .into_iter()
            .flat_map(|description| description.main_role_ids.iter().map(AsRef::as_ref))
    }
}
