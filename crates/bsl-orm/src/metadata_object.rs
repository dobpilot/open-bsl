//! Неизменяемое описание объекта метаданных: вид, стандартные реквизиты,
//! нумерация, реквизиты и табличные части.

use super::*;

/// Вид объекта метаданных, нужный исполняемому серверному коду.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetadataKind {
    /// Подсистема конфигурации.
    Subsystem,
    /// Язык конфигурации.
    Language,
    /// Перечисление конфигурации.
    Enumeration,
    /// Общая картинка.
    CommonPicture,
    /// Общий реквизит конфигурации.
    CommonAttribute,
    /// Справочник.
    Catalog,
    /// Документ.
    Document,
    /// Журнал документов.
    DocumentJournal,
    /// Регистр сведений.
    InformationRegister,
    /// Регистр накопления.
    AccumulationRegister,
    /// Регистр бухгалтерии.
    AccountingRegister,
    /// Регистр расчёта.
    CalculationRegister,
    /// План счетов.
    ChartOfAccounts,
    /// План видов характеристик.
    ChartOfCharacteristicTypes,
    /// План видов расчёта.
    ChartOfCalculationTypes,
    /// План обмена.
    ExchangePlan,
    /// Константа.
    Constant,
    /// Отчёт.
    Report,
    /// Обработка.
    DataProcessor,
    /// Бизнес-процесс.
    BusinessProcess,
    /// Задача.
    Task,
    /// Хранилище настроек.
    SettingsStorage,
    /// Последовательность.
    Sequence,
    /// Функциональная опция.
    FunctionalOption,
    /// Элемент стиля.
    StyleItem,
    /// Роль.
    Role,
    /// Параметр сеанса.
    SessionParameter,
    /// Общий модуль.
    CommonModule,
    /// Регламентное задание.
    ScheduledJob,
    /// Корень приобретённой конфигурации; отдельной коллекции у него нет.
    Configuration,
}

impl MetadataKind {
    /// Платформенные методы собственного получателя модуля менеджера.
    pub fn receiver_methods(self) -> &'static [MethodDescriptor] {
        manager_methods(self)
    }

    /// Возвращает русское имя коллекции этого вида метаданных.
    #[must_use]
    pub fn collection_name(self) -> Option<&'static str> {
        metadata_collections()
            .iter()
            .find(|(_, _, kind)| *kind == self)
            .map(|(name, _, _)| *name)
    }
}

/// Тип стандартного кода справочника в метаданных конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogCodeType {
    /// Числовой код.
    Number,
    /// Строковый код.
    String,
}

/// Основное представление справочника, приобретённое из метаданных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogDefaultPresentation {
    /// Представление по коду.
    AsCode,
    /// Представление по наименованию.
    AsDescription,
}

/// Приобретённый вид иерархии, независимый от признака иерархичности.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogHierarchyType {
    /// Иерархия групп и элементов.
    FoldersAndItems,
    /// Иерархия только элементов.
    Items,
}

/// Приобретённые настройки стандартных реквизитов справочника.
/// Нулевая длина сохраняется как настройка и сама по себе не означает,
/// что соответствующее свойство отсутствует в объектной модели BSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogStandardAttributes {
    /// Тип кода.
    pub code_type: CatalogCodeType,
    /// Настроенная длина кода.
    pub code_length: u32,
    /// Настроенная длина наименования.
    pub description_length: u32,
}

/// Приобретённые настройки нового объекта плана видов характеристик.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacteristicSettings {
    /// Настроенная длина строкового кода.
    pub code_length: u32,
    /// Настроенная длина наименования.
    pub description_length: u32,
    /// Полный приобретённый состав типов характеристики с квалификаторами.
    /// Пустой список означает известный пустой состав, а не неизвестные данные.
    pub value_types: Vec<open_bsl::ColumnType>,
}

/// Приобретённый тип номера документа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNumberType {
    /// Строковый номер.
    String,
    /// Числовой номер.
    Number,
}

/// Известные настройки номера из метаданных документа.
/// Отсутствие описания отличается от известной нулевой длины номера.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentNumberSettings {
    /// Настроенный тип номера.
    pub number_type: DocumentNumberType,
    /// Настроенная длина номера; ноль означает отключённую нумерацию.
    pub number_length: u32,
}

/// Приобретённая периодичность нумерации документа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNumberPeriodicity {
    /// Нумерация без разделения по периодам.
    Nonperiodical,
    /// Нумерация в пределах года.
    Year,
    /// Нумерация в пределах квартала.
    Quarter,
    /// Нумерация в пределах месяца.
    Month,
    /// Нумерация в пределах дня.
    Day,
}

/// Полностью приобретённая область нумерации, без состояния резервирования.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNumberingSettings {
    /// Периодичность нумерации из конфигурации.
    pub periodicity: DocumentNumberPeriodicity,
    /// Идентичность объекта нумератора; отсутствие означает известную пустую
    /// ссылку, а не неизвестную форму описателя.
    pub numerator: Option<Arc<str>>,
}

/// Приобретённая настройка проверки заполнения реквизита.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillChecking {
    /// Автоматическая проверка отключена.
    DontCheck,
    /// При незаполненном значении требуется сообщение об ошибке.
    ShowError,
}

/// Приобретённая настройка стандартного реквизита справочника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogStandardAttributeFillChecking {
    /// Каноническое имя стандартного реквизита.
    pub name: String,
    /// Известный режим проверки заполнения.
    pub fill_checking: FillChecking,
}

/// Неизменяемое описание собственного реквизита документа.
/// Значение заполнения хранится снимком, без памяти объектов живого сеанса.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentAttribute {
    id: Arc<str>,
    name: Arc<str>,
    pub(crate) types: Arc<[open_bsl::ColumnType]>,
    fill_from_filling_value: Option<bool>,
    fill_checking: Option<FillChecking>,
    synonyms: Arc<[(Arc<str>, Arc<str>)]>,
    fill_value: Option<Arc<SerializedValueGraph>>,
}

impl DocumentAttribute {
    /// Создаёт описание из идентификатора, имени и приобретённых типов.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        types: Vec<open_bsl::ColumnType>,
    ) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            types: types.into(),
            fill_from_filling_value: None,
            fill_checking: None,
            synonyms: Arc::from([]),
            fill_value: None,
        }
    }

    /// Сохраняет известную настройку заполнения из переданного значения.
    #[must_use]
    pub const fn with_fill_from_filling_value(mut self, enabled: bool) -> Self {
        self.fill_from_filling_value = Some(enabled);
        self
    }

    /// Сохраняет приобретённый режим проверки заполнения.
    #[must_use]
    pub const fn with_fill_checking(mut self, setting: FillChecking) -> Self {
        self.fill_checking = Some(setting);
        self
    }

    /// Возвращает режим проверки; `None` означает неизвестную настройку.
    #[must_use]
    pub const fn fill_checking(&self) -> Option<FillChecking> {
        self.fill_checking
    }

    /// Сохраняет приобретённый синоним реквизита для указанного кода языка.
    #[must_use]
    pub fn with_synonym(mut self, language: impl Into<String>, text: impl Into<String>) -> Self {
        let mut synonyms = self.synonyms.to_vec();
        synonyms.push((Arc::from(language.into()), Arc::from(text.into())));
        self.synonyms = synonyms.into();
        self
    }

    /// Возвращает синоним указанного языка либо каноническое имя реквизита.
    #[must_use]
    pub fn presentation(&self, language: &str) -> &str {
        self.synonyms
            .iter()
            .find(|(code, text)| folded_eq(code, language) && !text.is_empty())
            .map_or(self.name(), |(_, text)| text)
    }

    /// Сохраняет приобретённый снимок значения заполнения.
    /// Снимок должен содержать один корень; исполнение проверяет его при чтении.
    #[must_use]
    pub fn with_fill_value(mut self, value: SerializedValueGraph) -> Self {
        self.fill_value = Some(Arc::new(value));
        self
    }

    /// Устойчивый идентификатор реквизита в конфигурации.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Каноническое имя реквизита.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Приобретённые типы и квалификаторы реквизита.
    #[must_use]
    pub fn types(&self) -> &[open_bsl::ColumnType] {
        &self.types
    }
    /// Настройка заполнения; `None` означает отсутствие сведений.
    #[must_use]
    pub const fn fill_from_filling_value(&self) -> Option<bool> {
        self.fill_from_filling_value
    }
    /// Снимок значения заполнения; `None` отличается от снимка `Неопределено`.
    #[must_use]
    pub fn fill_value(&self) -> Option<&SerializedValueGraph> {
        self.fill_value.as_deref()
    }
}

/// Собственная табличная часть документа с известной идентичностью.
/// Неизвестные реквизиты строк отличаются от приобретённой пустой коллекции.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentTabularSection {
    pub(crate) id: Arc<str>,
    name: Arc<str>,
    attributes: Option<Arc<[DocumentAttribute]>>,
    fill_checking: Option<FillChecking>,
}

impl DocumentTabularSection {
    /// Создаёт описание части без предположений о реквизитах её строк.
    #[must_use]
    pub fn new(id: impl Into<Arc<str>>, name: impl Into<Arc<str>>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            attributes: None,
            fill_checking: None,
        }
    }

    /// Сохраняет приобретённую проверку самой части, отдельно от её столбцов.
    #[must_use]
    pub const fn with_fill_checking(mut self, setting: FillChecking) -> Self {
        self.fill_checking = Some(setting);
        self
    }

    /// Возвращает проверку самой части; `None` означает неизвестную настройку.
    #[must_use]
    pub const fn fill_checking(&self) -> Option<FillChecking> {
        self.fill_checking
    }

    /// Сохраняет полный список реквизитов строки в порядке конфигурации.
    #[must_use]
    pub fn with_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.attributes = Some(attributes.into());
        self
    }

    /// Возвращает приобретённый идентификатор части.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Возвращает имя части в конфигурации.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Возвращает полный список; `None` означает неизвестные реквизиты строк.
    #[must_use]
    pub fn attributes(&self) -> Option<&[DocumentAttribute]> {
        self.attributes.as_deref()
    }
}

/// Неизменяемое описание одного объекта метаданных.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataObject {
    pub(crate) id: Arc<str>,
    pub(crate) name: Arc<str>,
    pub(crate) reference_type_presentation: Option<Arc<str>>,
    pub(crate) kind: MetadataKind,
    pub(crate) declared_types: Option<Arc<[open_bsl::ColumnType]>>,
    pub(crate) exchange_plan_attributes: Option<Arc<[DocumentAttribute]>>,
    pub(crate) manager_module: Option<Arc<str>>,
    pub(crate) object_module: Option<Arc<str>>,
    pub(crate) object_module_is_extension: bool,
    pub(crate) own_extension_object_module_safe_mode: Option<bool>,
    catalog_standard_attributes: Option<CatalogStandardAttributes>,
    pub(crate) characteristic_settings: Option<Arc<CharacteristicSettings>>,
    pub(crate) route_points: Option<route_points::Composition>,
    pub(crate) information_register: Option<information_register_get::Settings>,
    pub(crate) accumulation_register_kind: Option<AccumulationRegisterKind>,
    pub(crate) accumulation_dimensions: Option<Arc<[DocumentAttribute]>>,
    pub(crate) accumulation_resources: Option<Arc<[DocumentAttribute]>>,
    pub(crate) characteristic_attributes: Option<Arc<[DocumentAttribute]>>,
    catalog_standard_fill_checking: Option<Arc<[CatalogStandardAttributeFillChecking]>>,
    pub(crate) business_process_standard_fill_checking:
        Option<Arc<[CatalogStandardAttributeFillChecking]>>,
    catalog_owners: Option<Arc<[String]>>,
    pub(crate) catalog_common_attributes: Option<Arc<[DocumentAttribute]>>,
    pub(crate) common_attribute_auto_use: Option<bool>,
    pub(crate) common_attribute_data_separation: Option<bool>,
    pub(crate) common_attribute_separation_parameters:
        Option<Arc<CommonAttributeSeparationParameters>>,
    pub(crate) common_attribute_content: Option<Arc<[CommonAttributeContentEntry]>>,
    pub(crate) common_attribute_condition: Option<CommonAttributeCondition>,
    pub(crate) functional_option_settings: Option<Arc<FunctionalOptionSettings>>,
    pub(crate) catalog_attributes: Option<Arc<[DocumentAttribute]>>,
    catalog_hierarchy_type: Option<CatalogHierarchyType>,
    pub(crate) business_process_number: Option<BusinessProcessNumberSettings>,
    pub(crate) business_process_task: Option<Arc<str>>,
    pub(crate) business_process_attributes: Option<Arc<[DocumentAttribute]>>,
    pub(crate) business_process_sections: Option<Arc<[DocumentTabularSection]>>,
    pub(crate) data_processor_attributes: Option<Arc<[DocumentAttribute]>>,
    pub(crate) report_attributes: Option<Arc<[DocumentAttribute]>>,
    pub(crate) report_sections: Option<Arc<[DocumentTabularSection]>>,
    pub(crate) data_processor_sections: Option<Arc<[DocumentTabularSection]>>,
    document_number_settings: Option<DocumentNumberSettings>,
    document_numbering_settings: Option<DocumentNumberingSettings>,
    document_number_fixed_length: bool,
    pub(crate) document_attributes: Option<Arc<[DocumentAttribute]>>,
    document_standard_fill_checking: Option<Arc<[CatalogStandardAttributeFillChecking]>>,
    pub(crate) document_tabular_sections: Option<Arc<[DocumentTabularSection]>>,
    document_registers: Option<Arc<[String]>>,
    catalog_default_presentation: Option<CatalogDefaultPresentation>,
    catalog_attribute_names: Option<Arc<[String]>>,
    catalog_tabular_section_names: Option<Arc<[String]>>,
    catalog_predefined_names: Arc<[String]>,
    catalog_hierarchical: Option<bool>,
    reference_storage_number: Option<u32>,
    pub(crate) subsystem_parent: Option<Arc<str>>,
    pub(crate) subsystem_command_interface: Option<bool>,
    pub(crate) string_properties: Arc<[(Arc<str>, Arc<str>)]>,
}

/// Неизменяемое описание значения конфигурационного перечисления.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationEnumValue {
    pub(crate) owner_id: Arc<str>,
    pub(crate) value_id: Arc<str>,
    pub(crate) name: Arc<str>,
    pub(crate) synonyms: Arc<[(Arc<str>, Arc<str>)]>,
}

impl ConfigurationEnumValue {
    /// Создаёт описание из устойчивых идентификаторов приобретённого снимка.
    #[must_use]
    pub fn new(
        owner_id: impl Into<String>,
        value_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            owner_id: Arc::from(owner_id.into()),
            value_id: Arc::from(value_id.into()),
            name: Arc::from(name.into()),
            synonyms: Arc::from([]),
        }
    }

    /// Добавляет приобретённый синоним значения для указанного кода языка.
    /// При отсутствии синонима основного языка используется имя значения.
    #[must_use]
    pub fn with_synonym(mut self, language: impl Into<String>, text: impl Into<String>) -> Self {
        let mut synonyms = self.synonyms.to_vec();
        synonyms.push((Arc::from(language.into()), Arc::from(text.into())));
        self.synonyms = synonyms.into();
        self
    }

    /// Возвращает идентификатор объекта-перечисления.
    #[must_use]
    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    /// Возвращает идентификатор значения перечисления.
    #[must_use]
    pub fn value_id(&self) -> &str {
        &self.value_id
    }

    /// Возвращает имя значения в конфигурации.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl MetadataObject {
    /// Сохраняет приобретённый состав типов самого объекта, например константы.
    /// `None` при создании объекта сохраняет отличие неизвестных типов от пустых.
    #[must_use]
    pub fn with_declared_types(mut self, types: Vec<open_bsl::ColumnType>) -> Self {
        self.declared_types = Some(types.into());
        self
    }

    /// Возвращает полный приобретённый состав типов; `None` означает неизвестность.
    #[must_use]
    pub fn declared_types(&self) -> Option<&[open_bsl::ColumnType]> {
        self.declared_types.as_deref()
    }

    /// Возвращает известные связи параметров разделителя; `None` — неизвестные связи.
    #[must_use]
    pub fn common_attribute_separation_parameters(
        &self,
    ) -> Option<&CommonAttributeSeparationParameters> {
        self.common_attribute_separation_parameters.as_deref()
    }

    /// Сохраняет явно известный режим разделения данных общего реквизита.
    /// Неизвестный режим нельзя подменять выключенным разделением.
    #[must_use]
    pub const fn with_common_attribute_data_separation(mut self, enabled: bool) -> Self {
        self.common_attribute_data_separation = Some(enabled);
        self
    }

    /// Сохраняет приобретённую настройку автоиспользования общего реквизита.
    /// Без этой настройки значение остаётся неизвестным, а не выключенным.
    #[must_use]
    pub const fn with_common_attribute_auto_use(mut self, enabled: bool) -> Self {
        self.common_attribute_auto_use = Some(enabled);
        self
    }

    /// Создаёт объект без связанного модуля менеджера.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: MetadataKind) -> Self {
        Self {
            id: Arc::from(id.into()),
            name: Arc::from(name.into()),
            reference_type_presentation: None,
            kind,
            declared_types: None,
            exchange_plan_attributes: None,
            manager_module: None,
            object_module: None,
            object_module_is_extension: false,
            own_extension_object_module_safe_mode: None,
            catalog_standard_attributes: None,
            characteristic_settings: None,
            route_points: None,
            information_register: None,
            accumulation_register_kind: None,
            accumulation_dimensions: None,
            accumulation_resources: None,
            characteristic_attributes: None,
            catalog_standard_fill_checking: None,
            business_process_standard_fill_checking: None,
            catalog_owners: None,
            catalog_common_attributes: None,
            common_attribute_auto_use: None,
            common_attribute_data_separation: None,
            common_attribute_separation_parameters: None,
            common_attribute_content: None,
            common_attribute_condition: None,
            functional_option_settings: None,
            catalog_attributes: None,
            catalog_hierarchy_type: None,
            business_process_number: None,
            business_process_task: None,
            business_process_attributes: None,
            business_process_sections: None,
            data_processor_attributes: None,
            report_attributes: None,
            report_sections: None,
            data_processor_sections: None,
            document_number_settings: None,
            document_numbering_settings: None,
            document_number_fixed_length: false,
            document_attributes: None,
            document_standard_fill_checking: None,
            document_tabular_sections: None,
            document_registers: None,
            catalog_default_presentation: None,
            catalog_attribute_names: None,
            catalog_tabular_section_names: None,
            catalog_predefined_names: Arc::from([]),
            catalog_hierarchical: None,
            reference_storage_number: None,
            subsystem_parent: None,
            subsystem_command_interface: None,
            string_properties: Arc::from([]),
        }
    }

    /// Сохраняет приобретённый основной номер объекта из DBNames.
    #[must_use]
    pub const fn with_reference_storage_number(mut self, number: u32) -> Self {
        self.reference_storage_number = Some(number);
        self
    }

    /// Возвращает приобретённый основной номер объекта из DBNames.
    #[must_use]
    pub const fn reference_storage_number(&self) -> Option<u32> {
        self.reference_storage_number
    }

    /// Связывает объект с именованным модулем менеджера конфигурации.
    #[must_use]
    pub fn with_manager_module(mut self, module: impl Into<String>) -> Self {
        self.manager_module = Some(Arc::from(module.into()));
        self
    }

    /// Связывает объект с именованным модулем объекта конфигурации.
    /// Прежняя политика источника расширения снимается.
    #[must_use]
    pub fn with_object_module(mut self, module: impl Into<String>) -> Self {
        self.object_module = Some(Arc::from(module.into()));
        self.object_module_is_extension = false;
        self.own_extension_object_module_safe_mode = None;
        self
    }

    /// Связывает объект с окончательным текстом модуля из расширения.
    /// Даже у заимствованного объекта имя модуля может остаться прежним.
    /// Отдельная политика собственного события при этом снимается.
    #[must_use]
    pub fn with_extension_object_module(mut self, module: impl Into<String>) -> Self {
        self.object_module = Some(Arc::from(module.into()));
        self.object_module_is_extension = true;
        self.own_extension_object_module_safe_mode = None;
        self
    }

    /// Связывает собственный объект расширения с модулем и приобретённой
    /// настройкой безопасности его источника. Политика применяется к событию
    /// выдачи нового кода, а не ко всем методам объекта.
    #[must_use]
    pub fn with_own_extension_object_module(
        mut self,
        module: impl Into<String>,
        safe_mode: bool,
    ) -> Self {
        self.object_module = Some(Arc::from(module.into()));
        self.object_module_is_extension = true;
        self.own_extension_object_module_safe_mode = Some(safe_mode);
        self
    }

    /// Возвращает известную политику события собственного объекта расширения.
    /// Отсутствие политики не подменяется выключенным безопасным режимом.
    #[must_use]
    pub const fn own_extension_object_module_safe_mode(&self) -> Option<bool> {
        self.own_extension_object_module_safe_mode
    }

    /// Сохраняет известные настройки стандартных реквизитов справочника.
    /// Отсутствие настройки отличается от нулевой длины реквизита.
    #[must_use]
    pub const fn with_catalog_standard_attributes(
        mut self,
        attributes: CatalogStandardAttributes,
    ) -> Self {
        self.catalog_standard_attributes = Some(attributes);
        self
    }

    /// Сохраняет полные приобретённые настройки плана видов характеристик.
    /// Не подставляет настройки справочника или состав типов по умолчанию.
    #[must_use]
    pub fn with_characteristic_settings(mut self, settings: CharacteristicSettings) -> Self {
        self.characteristic_settings = Some(Arc::new(settings));
        self
    }

    /// Возвращает настройки плана; `None` означает неизвестные данные.
    #[must_use]
    pub fn characteristic_settings(&self) -> Option<&CharacteristicSettings> {
        self.characteristic_settings.as_deref()
    }

    /// Сохраняет приобретённые настройки номера документа.
    /// Не вычисляет их по SQL-хранилищу и не подставляет значения по умолчанию.
    #[must_use]
    pub const fn with_document_number_settings(mut self, settings: DocumentNumberSettings) -> Self {
        self.document_number_settings = Some(settings);
        self
    }

    /// Возвращает настройки номера; `None` означает неизвестную форму снимка.
    #[must_use]
    pub const fn document_number_settings(&self) -> Option<DocumentNumberSettings> {
        self.document_number_settings
    }

    /// Сохраняет полностью приобретённую область нумерации документа.
    /// Состояние резервов сеанса в неизменяемые метаданные не включается.
    #[must_use]
    pub fn with_document_numbering_settings(mut self, settings: DocumentNumberingSettings) -> Self {
        self.document_numbering_settings = Some(settings);
        self
    }

    /// Возвращает известную область нумерации; `None` означает неизвестную
    /// или неполную форму, включая неизвестную ссылку на нумератор.
    #[must_use]
    pub fn document_numbering_settings(&self) -> Option<&DocumentNumberingSettings> {
        self.document_numbering_settings.as_ref()
    }

    /// Задаёт фиксированную допустимую длину строкового номера.
    #[must_use]
    pub const fn with_document_number_fixed_length(mut self, fixed: bool) -> Self {
        self.document_number_fixed_length = fixed;
        self
    }

    /// Возвращает признак фиксированной длины строкового номера.
    #[must_use]
    pub const fn document_number_fixed_length(&self) -> bool {
        self.document_number_fixed_length
    }

    /// Сохраняет полный приобретённый список собственных реквизитов документа.
    /// Пустой список отличается от неизвестной или неполной коллекции.
    #[must_use]
    pub fn with_document_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.document_attributes = Some(attributes.into());
        self
    }

    /// Возвращает полную коллекцию; `None` означает, что она не приобретена.
    #[must_use]
    pub fn document_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.document_attributes.as_deref()
    }

    /// Сохраняет полный приобретённый список проверок стандартных полей документа.
    #[must_use]
    pub fn with_document_standard_fill_checking(
        mut self,
        settings: Vec<CatalogStandardAttributeFillChecking>,
    ) -> Self {
        self.document_standard_fill_checking = Some(settings.into());
        self
    }

    /// Возвращает полный список; `None` означает неизвестные настройки.
    #[must_use]
    pub fn document_standard_fill_checking(
        &self,
    ) -> Option<&[CatalogStandardAttributeFillChecking]> {
        self.document_standard_fill_checking.as_deref()
    }

    /// Сохраняет полный список собственных табличных частей документа.
    #[must_use]
    pub fn with_document_tabular_sections(mut self, sections: Vec<DocumentTabularSection>) -> Self {
        self.document_tabular_sections = Some(sections.into());
        self
    }

    /// Возвращает полный список; `None` означает неизвестный состав частей.
    #[must_use]
    pub fn document_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.document_tabular_sections.as_deref()
    }

    /// Сохраняет полный список идентификаторов регистров движений документа.
    /// Порядок задан конфигурацией; пустой список отличается от неизвестного.
    #[must_use]
    pub fn with_document_registers(mut self, registers: Vec<String>) -> Self {
        self.document_registers = Some(registers.into());
        self
    }

    /// Возвращает приобретённый список; `None` означает неизвестные связи.
    #[must_use]
    pub fn document_registers(&self) -> Option<&[String]> {
        self.document_registers.as_deref()
    }

    /// Сохраняет известную настройку основного представления справочника.
    /// Она не выводится из типа кода или длины наименования.
    #[must_use]
    pub const fn with_catalog_default_presentation(
        mut self,
        presentation: CatalogDefaultPresentation,
    ) -> Self {
        self.catalog_default_presentation = Some(presentation);
        self
    }

    /// Сохраняет полный приобретённый список собственных реквизитов справочника.
    /// Пустой список означает известное отсутствие реквизитов; поля табличных
    /// частей в этот список не включаются. Типы и значения здесь не задаются.
    #[must_use]
    pub fn with_catalog_attribute_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.catalog_attribute_names =
            Some(names.into_iter().map(Into::into).collect::<Vec<_>>().into());
        self
    }

    /// Сохраняет полный приобретённый список собственных табличных частей.
    /// Имена реквизитов строк сюда не включаются. Пустой список означает
    /// известное отсутствие частей; значения строк здесь не задаются.
    #[must_use]
    pub fn with_catalog_tabular_section_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.catalog_tabular_section_names =
            Some(names.into_iter().map(Into::into).collect::<Vec<_>>().into());
        self
    }

    /// Сохраняет полный приобретённый список собственных реквизитов отчёта.
    #[must_use]
    pub fn with_report_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.report_attributes = Some(attributes.into());
        self
    }

    /// Возвращает реквизиты отчёта; `None` означает неизвестный состав.
    pub fn report_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.report_attributes.as_deref()
    }

    /// Сохраняет полный приобретённый список табличных частей отчёта.
    #[must_use]
    pub fn with_report_tabular_sections(mut self, sections: Vec<DocumentTabularSection>) -> Self {
        self.report_sections = Some(sections.into());
        self
    }

    /// Возвращает табличные части отчёта; `None` означает неизвестный состав.
    pub fn report_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.report_sections.as_deref()
    }

    pub(crate) fn executable_attributes(&self) -> Option<&[DocumentAttribute]> {
        if self.kind == MetadataKind::Report {
            self.report_attributes()
        } else {
            self.data_processor_attributes()
        }
    }

    pub(crate) fn executable_sections(&self) -> Option<&[DocumentTabularSection]> {
        if self.kind == MetadataKind::Report {
            self.report_tabular_sections()
        } else {
            self.data_processor_tabular_sections()
        }
    }

    /// Сохраняет полный список собственных реквизитов обработки.
    #[must_use]
    pub fn with_data_processor_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.data_processor_attributes = Some(attributes.into());
        self
    }

    /// Возвращает полный список реквизитов обработки; `None` означает неизвестность.
    #[must_use]
    pub fn data_processor_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.data_processor_attributes.as_deref()
    }

    /// Сохраняет полный список собственных табличных частей обработки.
    #[must_use]
    pub fn with_data_processor_tabular_sections(
        mut self,
        sections: Vec<DocumentTabularSection>,
    ) -> Self {
        self.data_processor_sections = Some(sections.into());
        self
    }

    /// Возвращает полный список частей обработки; `None` означает неизвестность.
    #[must_use]
    pub fn data_processor_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.data_processor_sections.as_deref()
    }

    /// Запоминает приобретённый признак иерархического справочника.
    /// Он определяет доступность записи родителя нового элемента;
    /// отсутствие сведений не подменяется значением `false`.
    #[must_use]
    pub const fn with_catalog_hierarchical(mut self, hierarchical: bool) -> Self {
        self.catalog_hierarchical = Some(hierarchical);
        self
    }

    /// Сохраняет приобретённые имена предопределённых элементов справочника
    /// или плана видов характеристик.
    /// Имена описывают свойства менеджера; UUID записей здесь не задаются.
    #[must_use]
    pub fn with_catalog_predefined_names(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.catalog_predefined_names =
            names.into_iter().map(Into::into).collect::<Vec<_>>().into();
        self
    }

    /// Возвращает приобретённые имена предопределённых элементов менеджера.
    #[must_use]
    pub fn catalog_predefined_names(&self) -> &[String] {
        &self.catalog_predefined_names
    }

    /// Указывает родительскую подсистему для вложенного объекта.
    #[must_use]
    pub fn with_subsystem_parent(mut self, parent: impl Into<String>) -> Self {
        self.subsystem_parent = Some(Arc::from(parent.into()));
        self
    }

    /// Запоминает признак включения подсистемы в командный интерфейс.
    #[must_use]
    pub fn with_subsystem_command_interface(mut self, included: bool) -> Self {
        self.subsystem_command_interface = Some(included);
        self
    }

    /// Задаёт приобретённое представление типа ссылки этого объекта.
    /// Идентичность типа и квалифицированные имена поиска сохраняются.
    /// При отсутствии значения используется прежнее имя объекта.
    #[must_use]
    pub fn with_reference_type_presentation(mut self, presentation: impl Into<String>) -> Self {
        self.reference_type_presentation = Some(Arc::from(presentation.into()));
        self
    }

    /// Добавляет строковое свойство объекта метаданных.
    #[must_use]
    pub fn with_string_property(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let mut properties = self.string_properties.to_vec();
        properties.push((Arc::from(name.into()), Arc::from(value.into())));
        self.string_properties = properties.into();
        self
    }

    #[must_use]
    /// Возвращает устойчивый идентификатор объекта.
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    /// Возвращает имя объекта в конфигурации.
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Возвращает вид объекта.
    pub const fn kind(&self) -> MetadataKind {
        self.kind
    }

    #[must_use]
    /// Возвращает имя связанного модуля менеджера.
    pub fn manager_module(&self) -> Option<&str> {
        self.manager_module.as_deref()
    }

    /// Возвращает имя связанного модуля объекта.
    #[must_use]
    pub fn object_module(&self) -> Option<&str> {
        self.object_module.as_deref()
    }

    /// Возвращает признак окончательного текста модуля объекта из расширения.
    /// При отсутствии модуля возвращает `false`.
    #[must_use]
    pub const fn object_module_is_extension(&self) -> bool {
        self.object_module_is_extension
    }

    /// Возвращает известные настройки стандартных реквизитов справочника.
    /// `None` означает отсутствие сведений, а не стандартные значения.
    #[must_use]
    pub const fn catalog_standard_attributes(&self) -> Option<CatalogStandardAttributes> {
        self.catalog_standard_attributes
    }

    /// Сохраняет полную таблицу проверок стандартных реквизитов в её порядке.
    /// Пустой список означает известное отсутствие настроек.
    #[must_use]
    pub fn with_catalog_standard_fill_checking(
        mut self,
        settings: Vec<CatalogStandardAttributeFillChecking>,
    ) -> Self {
        self.catalog_standard_fill_checking = Some(settings.into());
        self
    }

    /// Возвращает полную таблицу проверок; `None` означает неизвестную форму.
    #[must_use]
    pub fn catalog_standard_fill_checking(
        &self,
    ) -> Option<&[CatalogStandardAttributeFillChecking]> {
        self.catalog_standard_fill_checking.as_deref()
    }

    /// Сохраняет полный приобретённый список владельцев справочника в его порядке.
    /// Пустой список означает известное отсутствие владельцев.
    #[must_use]
    pub fn with_catalog_owners(mut self, owners: Vec<String>) -> Self {
        self.catalog_owners = Some(owners.into());
        self
    }

    /// Возвращает приобретённые идентификаторы владельцев справочника.
    /// `None` означает неизвестный список, а не отсутствие владельцев.
    #[must_use]
    pub fn catalog_owners(&self) -> Option<&[String]> {
        self.catalog_owners.as_deref()
    }

    /// Возвращает полный применимый состав общих реквизитов справочника.
    /// `None` означает неизвестный состав; собственные реквизиты сюда не входят.
    #[must_use]
    pub fn catalog_common_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.catalog_common_attributes.as_deref()
    }

    /// Возвращает полный приобретённый состав реквизитов плана видов характеристик.
    /// `None` означает неизвестный состав, а не пустой список.
    #[must_use]
    pub fn characteristic_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.characteristic_attributes.as_deref()
    }

    /// Сохраняет полный состав реквизитов плана видов характеристик.
    /// Использует тот же неизменяемый описатель, что реквизиты документов и справочников.
    #[must_use]
    pub fn with_characteristic_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.characteristic_attributes = Some(attributes.into());
        self
    }

    /// Возвращает полные собственные реквизиты справочника с типами и настройками.
    /// `None` означает неизвестный состав, а не пустую коллекцию.
    #[must_use]
    pub fn catalog_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.catalog_attributes.as_deref()
    }

    /// Сохраняет собственные реквизиты отдельно от применимых общих реквизитов.
    #[must_use]
    pub fn with_catalog_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.catalog_attributes = Some(attributes.into());
        self
    }

    /// Сохраняет полный применимый состав общих реквизитов отдельно от собственных.
    #[must_use]
    pub fn with_catalog_common_attributes(mut self, attributes: Vec<DocumentAttribute>) -> Self {
        self.catalog_common_attributes = Some(attributes.into());
        self
    }

    /// Возвращает приобретённую настройку основного представления.
    /// `None` означает отсутствие сведений, а не выбор по наименованию.
    #[must_use]
    pub const fn catalog_default_presentation(&self) -> Option<CatalogDefaultPresentation> {
        self.catalog_default_presentation
    }

    /// Возвращает собственные имена реквизитов без полей табличных частей.
    /// `None` означает, что полный список не приобретён; известный пустой
    /// список возвращается как `Some` с пустым срезом.
    #[must_use]
    pub fn catalog_attribute_names(&self) -> Option<&[String]> {
        self.catalog_attribute_names.as_deref()
    }

    /// Возвращает собственные имена табличных частей без реквизитов строк.
    /// `None` означает, что полный список не приобретён.
    #[must_use]
    pub fn catalog_tabular_section_names(&self) -> Option<&[String]> {
        self.catalog_tabular_section_names.as_deref()
    }

    /// Возвращает известный признак иерархии справочника.
    /// `None` означает отсутствие приобретённых сведений.
    #[must_use]
    pub const fn catalog_hierarchical(&self) -> Option<bool> {
        self.catalog_hierarchical
    }

    /// Сохраняет приобретённый вид иерархии, даже если иерархия выключена.
    #[must_use]
    pub const fn with_catalog_hierarchy_type(mut self, kind: CatalogHierarchyType) -> Self {
        self.catalog_hierarchy_type = Some(kind);
        self
    }

    /// Возвращает приобретённый вид; `None` означает неизвестные сведения.
    #[must_use]
    pub const fn catalog_hierarchy_type(&self) -> Option<CatalogHierarchyType> {
        self.catalog_hierarchy_type
    }

    /// Возвращает идентификатор родительской подсистемы, если она есть.
    #[must_use]
    pub fn subsystem_parent(&self) -> Option<&str> {
        self.subsystem_parent.as_deref()
    }

    /// Возвращает приобретённый признак включения подсистемы.
    #[must_use]
    pub const fn subsystem_command_interface(&self) -> Option<bool> {
        self.subsystem_command_interface
    }
}

/// Источник подробностей объектов одного неизменяемого индекса.
pub trait MetadataObjectSource: Send + Sync + std::fmt::Debug {
    /// Приобретает подробности достигнутой группы перед отдельными обращениями.
    /// По умолчанию источник загружает каждый объект отдельно.
    ///
    /// # Errors
    /// Возвращает ошибку приобретения подробностей или их зависимостей.
    fn prepare_objects(&self, _ids: &[&str]) -> Result<(), String> {
        Ok(())
    }

    /// Загружает подробности одного объекта и необходимые ему зависимости.
    ///
    /// # Errors
    /// Возвращает описание ошибки источника или неполного описания объекта.
    fn load_object(&self, id: &str) -> Result<MetadataObject, String>;
}
