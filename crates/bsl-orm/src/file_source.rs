//! Файловые описания дополняют ORM, но не подменяют схему и права базы.
use super::*;
use open_bsl::{ConfigurationProperty, ConfigurationSource, ModuleKind, TypeRef};

pub(super) struct Attached {
    pub source: Arc<dyn ConfigurationSource>,
    modules: HashMap<Arc<str>, Vec<open_bsl::ConfigurationModuleDeclaration>>,
    defined_types: HashMap<String, Arc<str>>,
    members: HashMap<Arc<str>, HashMap<Arc<str>, Vec<usize>>>,
    reference_families: HashMap<String, Vec<TypeRef>>,
}
impl Attached {
    fn new(
        source: Arc<dyn ConfigurationSource>,
        references: &[(Arc<str>, Arc<ValueTypeDescriptor>)],
    ) -> Self {
        // Состав семейства ссылок берётся из тех же дескрипторов ORM,
        // которые регистрируются в движке; второго набора типов здесь нет.
        let mut reference_families: HashMap<String, Vec<TypeRef>> = HashMap::new();
        for (_, descriptor) in references {
            for name in descriptor.names() {
                if let Some((family, _)) = name.split_once('.') {
                    reference_families
                        .entry(family.to_owned())
                        .or_default()
                        .push(TypeRef::Value(Arc::clone(descriptor)));
                }
            }
        }
        // Пустое семейство не добавляет типов; непустое использует
        // зарегистрированные ссылочные дескрипторы соответствующих владельцев.
        for family in [
            "ExchangePlanRef",
            "BusinessProcessRef",
            "DocumentRef",
            "ChartOfCalculationTypesRef",
            "TaskRef",
            "ChartOfCharacteristicTypesRef",
            "ChartOfAccountsRef",
            "CatalogRef",
            "EnumRef",
        ] {
            reference_families.entry(family.into()).or_default();
        }
        let mut modules: HashMap<Arc<str>, Vec<_>> = HashMap::new();
        for module in source.index().modules() {
            if let Some(owner) = module.owner() {
                modules
                    .entry(owner.into())
                    .or_default()
                    .push(module.clone());
            }
        }
        let defined_types = source
            .artifacts()
            .into_iter()
            .flat_map(|a| a.descriptions())
            .filter(|d| d.kind.as_ref() == "DefinedType")
            .map(|d| (d.name.to_uppercase(), Arc::clone(&d.id)))
            .collect();
        let mut members: HashMap<Arc<str>, HashMap<Arc<str>, Vec<usize>>> = HashMap::new();
        let root: Arc<str> = Arc::from("");
        if let Some(artifacts) = source.artifacts() {
            for (index, declaration) in artifacts.descriptions().iter().enumerate() {
                members
                    .entry(Arc::clone(declaration.owner.as_ref().unwrap_or(&root)))
                    .or_default()
                    .entry(Arc::clone(&declaration.kind))
                    .or_default()
                    .push(index);
            }
        }
        Self {
            source,
            modules,
            defined_types,
            members,
            reference_families,
        }
    }
}
impl std::fmt::Debug for Attached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FileSource")
            .field(&self.source.index().identity())
            .finish()
    }
}

impl MetadataCatalog {
    /// Присоединяет файловые описания после проверки нативной идентичности.
    /// Физические свойства и права остаются приобретёнными из БД.
    ///
    /// # Errors
    /// Возвращает ошибку корня, вида или владельца постоянного объекта.
    pub fn with_file_source(
        mut self,
        source: Arc<dyn ConfigurationSource>,
    ) -> Result<Self, String> {
        let artifacts = source.artifacts().ok_or("у источника нет описаний")?;
        if !self
            .configuration_object_id()
            .is_some_and(|id| id.eq_ignore_ascii_case(artifacts.root_identity()))
        {
            return Err("UUID корня файловой конфигурации не совпадает с UUID базы".into());
        }
        for declaration in artifacts
            .descriptions()
            .iter()
            .filter(|d| persistent_kind(&d.kind).is_some())
        {
            let expected = persistent_kind(&declaration.kind).expect("вид проверен");
            let object = self.object_by_id(&declaration.id).ok_or_else(|| {
                format!(
                    "файловый объект «{}» ({}) отсутствует в базе",
                    declaration.name, declaration.id
                )
            })?;
            if object.kind != expected || !folded_eq(object.name(), &declaration.name) {
                return Err(format!(
                    "файловый объект «{}» не совместим с объектом базы «{}»",
                    declaration.name,
                    object.name()
                ));
            }
        }
        let attached = Attached::new(source, &self.reference_types);
        // Эти типы уже обязательны для открытия сеанса и разделения данных.
        // Проверка файловой декларации не приобретает остальные объекты ORM.
        for declaration in attached
            .source
            .artifacts()
            .ok_or("нет файловых описаний")?
            .descriptions()
            .iter()
            .filter(|d| matches!(d.kind.as_ref(), "SessionParameter" | "CommonAttribute"))
        {
            if let Some(object) = self.object_by_id(&declaration.id)
                && object.declared_types().is_some()
            {
                validate_object(&attached, object)?;
            }
        }
        for object in Arc::make_mut(&mut self.objects) {
            bind_modules(&attached, object);
        }
        self.file_validations = (0..self.objects.len())
            .map(|_| std::sync::OnceLock::new())
            .collect();
        self.file_source = Some(attached);
        Ok(self)
    }
}

fn persistent_kind(kind: &str) -> Option<MetadataKind> {
    let plural = match kind {
        "ChartOfAccounts" => "ChartsOfAccounts".into(),
        "ChartOfCharacteristicTypes" => "ChartsOfCharacteristicTypes".into(),
        "ChartOfCalculationTypes" => "ChartsOfCalculationTypes".into(),
        "BusinessProcess" => "BusinessProcesses".into(),
        _ => format!("{kind}s"),
    };
    // Кодовые общие модули и формы не создают постоянного объекта хранилища.
    if matches!(
        kind,
        "CommonModule"
            | "Role"
            | "Subsystem"
            | "Language"
            | "FunctionalOption"
            | "StyleItem"
            | "ScheduledJob"
    ) {
        return None;
    }
    collection_kind(&plural)
}

pub(super) fn bind_modules(attached: &Attached, object: &mut MetadataObject) {
    object.manager_module = None;
    object.object_module = None;
    object.object_module_is_extension = false;
    object.own_extension_object_module_safe_mode = None;
    for declaration in attached.modules.get(object.id()).into_iter().flatten() {
        match declaration.kind() {
            ModuleKind::Manager => object.manager_module = Some(declaration.name().into()),
            ModuleKind::Object => {
                object.object_module = Some(declaration.name().into());
                object.object_module_is_extension = declaration.source_is_extension();
                object.own_extension_object_module_safe_mode =
                    declaration.own_extension_safe_mode();
            }
            _ => {}
        }
    }
}

pub(super) fn validate_object(attached: &Attached, object: &MetadataObject) -> Result<(), String> {
    let Some(artifacts) = attached.source.artifacts() else {
        return Ok(());
    };
    let Some(properties) = artifacts.description(object.id())? else {
        return Err(format!(
            "объект «{}» ({}) отсутствует в выбранном файловом источнике",
            object.name(),
            object.id()
        ));
    };
    if let (Some(ty), Some(expected)) = (properties.child("type"), object.declared_types()) {
        validate_types(attached, object.name(), ty, expected)?;
    }
    let expected = match object.kind {
        MetadataKind::Catalog => object.catalog_attributes.as_deref(),
        MetadataKind::Document => object.document_attributes.as_deref(),
        MetadataKind::DataProcessor => object.data_processor_attributes.as_deref(),
        MetadataKind::Report => object.report_attributes.as_deref(),
        MetadataKind::ChartOfCharacteristicTypes => object.characteristic_attributes.as_deref(),
        MetadataKind::BusinessProcess => object.business_process_attributes.as_deref(),
        MetadataKind::ExchangePlan => object.exchange_plan_attributes.as_deref(),
        _ => None,
    };
    if let Some(fields) = properties.child("attributes") {
        validate_fields(attached, object.name(), fields, expected)?;
    }
    for (group, expected) in [
        ("dimensions", object.accumulation_dimensions.as_deref()),
        ("resources", object.accumulation_resources.as_deref()),
    ] {
        let expected = if object.kind == MetadataKind::InformationRegister {
            object
                .information_register
                .as_ref()
                .and_then(|settings| settings.file_fields(group))
        } else {
            expected
        };
        if let Some(fields) = properties.child(group) {
            validate_fields(attached, object.name(), fields, expected)?;
        }
    }
    Ok(())
}

fn validate_fields(
    attached: &Attached,
    owner: &str,
    fields: &ConfigurationProperty,
    expected: Option<&[DocumentAttribute]>,
) -> Result<(), String> {
    if fields.children.is_empty() {
        return Ok(());
    }
    let expected = expected.ok_or_else(|| {
        format!(
            "{owner}: модель БД не предоставляет состав «{}»",
            fields.name
        )
    })?;
    let by_id: HashMap<_, _> = expected.iter().map(|field| (field.id(), field)).collect();
    for field in fields.children.iter() {
        let name = field
            .child("name")
            .map(|p| p.value.as_ref())
            .ok_or("нет имени файлового реквизита")?;
        let id = field
            .child("uuid")
            .map(|p| p.value.as_ref())
            .ok_or_else(|| format!("{owner}: нет UUID постоянного поля «{name}»"))?;
        let known = by_id
            .get(id)
            .ok_or_else(|| format!("{owner}: реквизит «{name}» ({id}) отсутствует в модели БД"))?;
        if !folded_eq(known.name(), name) {
            return Err(format!(
                "{owner}: имя поля «{name}» ({id}) не совпадает с моделью БД"
            ));
        }
        if let Some(ty) = field.child("type") {
            validate_types(attached, &format!("{owner}.{name}"), ty, &known.types)?;
        }
    }
    Ok(())
}

fn compatible_qualifiers(file: &open_bsl::ColumnType, column: &open_bsl::ColumnType) -> bool {
    if file.quals.is_empty() || file.quals == column.quals {
        return true;
    }
    // Нативные пары DEMO закрепляют явные умолчания XML для кратких
    // типов Config: `{"S"}` — строка нулевой переменной длины, `{"D"}` —
    // дата со временем. Иные ограничения не становятся совместимыми.
    column.quals.is_empty()
        && match column.id.native() {
            Some(open_bsl::TypeId::String) => file.quals == ["0", "1"],
            Some(open_bsl::TypeId::Date) => file.quals == ["DT"],
            _ => false,
        }
}

fn validate_types(
    attached: &Attached,
    name: &str,
    ty: &ConfigurationProperty,
    expected: &[open_bsl::ColumnType],
) -> Result<(), String> {
    let types = source_types(
        attached,
        ty,
        &|name| {
            expected
                .iter()
                .find(|column| type_matches(&column.id, name))
                .map(|column| column.id.clone())
        },
        &mut HashSet::new(),
    )
    .map_err(|error| format!("{name}: {error}"))?;
    if types.len() != expected.len()
        || types.iter().any(|file| {
            !expected
                .iter()
                .any(|column| column.id == file.id && compatible_qualifiers(file, column))
        })
    {
        let missing = types
            .iter()
            .filter(|file| {
                !expected
                    .iter()
                    .any(|column| column.id == file.id && compatible_qualifiers(file, column))
            })
            .take(3)
            .map(|column| column.id.name())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "тип «{name}» не совпадает с моделью БД: файловых типов {}, типов БД {}; отсутствуют в модели: {missing}",
            types.len(),
            expected.len()
        ));
    }
    Ok(())
}

fn type_matches(ty: &TypeRef, name: &str) -> bool {
    match ty {
        TypeRef::Native(id) => open_bsl::TypeId::lookup(name) == Some(*id),
        TypeRef::Value(descriptor) => descriptor
            .names()
            .iter()
            .any(|known| folded_eq(known, name)),
        TypeRef::Object(descriptor) => {
            folded_eq(descriptor.name, name)
                || descriptor
                    .type_names
                    .iter()
                    .any(|known| folded_eq(known, name))
        }
    }
}
fn type_names(node: &ConfigurationProperty) -> Vec<String> {
    let mut names = Vec::new();
    fn visit(node: &ConfigurationProperty, out: &mut Vec<String>) {
        if matches!(
            node.name.as_ref(),
            "type" | "types" | "typeSet" | "typeSets"
        ) && node.children.is_empty()
            && !node.value.is_empty()
        {
            let name = node.value.rsplit(':').next().unwrap_or(&node.value);
            let name = match name {
                "string" => "String",
                "decimal" | "double" => "Number",
                "boolean" => "Boolean",
                "dateTime" => "Date",
                other => other,
            };
            if !out.iter().any(|known| known == name) {
                out.push(name.into());
            }
        }
        for child in node.children.iter() {
            visit(child, out);
        }
    }
    visit(node, &mut names);
    names
}

fn source_types(
    attached: &Attached,
    node: &ConfigurationProperty,
    resolve: &impl Fn(&str) -> Option<TypeRef>,
    seen: &mut HashSet<String>,
) -> Result<Vec<open_bsl::ColumnType>, String> {
    let mut columns = Vec::new();
    for name in type_names(node) {
        if let Some(types) = attached.reference_families.get(&name) {
            for id in types {
                let column = open_bsl::ColumnType {
                    id: id.clone(),
                    quals: Vec::new(),
                };
                columns.push(column);
            }
            continue;
        }
        if let Some(alias) = name.strip_prefix("DefinedType.") {
            let id = attached
                .defined_types
                .get(&alias.to_uppercase())
                .ok_or_else(|| format!("нет определяемого типа «{alias}»"))?;
            if !seen.insert(id.to_string()) {
                return Err(format!("цикл определяемого типа «{alias}»"));
            }
            let description = attached
                .source
                .artifacts()
                .ok_or("нет файловых описаний")?
                .description(id)?
                .ok_or("нет описания определяемого типа")?;
            let ty = description
                .child("type")
                .ok_or("нет состава определяемого типа")?;
            columns.extend(source_types(attached, ty, resolve, seen)?);
            seen.remove(id.as_ref());
            continue;
        }
        let id = resolve(&name)
            .ok_or_else(|| format!("файловый тип «{name}» не найден в текущей модели"))?;
        let value = |qualifiers: &ConfigurationProperty, name: &str, default: &str| {
            qualifiers
                .child(name)
                .map_or_else(|| default.to_string(), |p| p.value.to_string())
        };
        let quals = match id.native() {
            Some(open_bsl::TypeId::String) => node
                .child("stringQualifiers")
                .map(|q| {
                    let policy = match value(q, "allowedLength", "Variable").as_str() {
                        "Variable" => Ok("1"),
                        "Fixed" => Ok("0"),
                        other => Err(format!("неподдержанная длина строки «{other}»")),
                    }?;
                    Ok::<_, String>(vec![value(q, "length", "0"), policy.into()])
                })
                .transpose()?,
            Some(open_bsl::TypeId::Number) => node
                .child("numberQualifiers")
                .map(|q| {
                    let policy = match value(q, "allowedSign", "Any").as_str() {
                        "Any" => Ok("0"),
                        "Nonnegative" => Ok("1"),
                        other => Err(format!("неподдержанный знак числа «{other}»")),
                    }?;
                    Ok::<_, String>(vec![
                        value(q, "digits", "0"),
                        value(q, "fractionDigits", "0"),
                        policy.into(),
                    ])
                })
                .transpose()?,
            Some(open_bsl::TypeId::Date) => node
                .child("dateQualifiers")
                .map(|q| {
                    let value = match value(q, "dateFractions", "DateTime").as_str() {
                        "DateTime" => Ok("DT"),
                        "Date" => Ok("D"),
                        "Time" => Ok("T"),
                        other => Err(format!("неподдержанный состав даты «{other}»")),
                    }?;
                    Ok::<_, String>(vec![value.into()])
                })
                .transpose()?,
            _ => None,
        }
        .unwrap_or_default();
        let column = open_bsl::ColumnType { id, quals };
        columns.push(column);
    }
    let mut unique = HashSet::new();
    columns.retain(|column| unique.insert((column.id.clone(), column.quals.clone())));
    Ok(columns)
}

#[derive(Debug, Clone)]
pub(super) struct View {
    catalog: Arc<MetadataCatalog>,
    id: Arc<str>,
    name: Arc<str>,
    kind: Arc<str>,
    root: Arc<std::sync::OnceLock<Result<Arc<ConfigurationProperty>, String>>>,
    path: Vec<usize>,
}
impl View {
    fn node(&self) -> RtResult<&ConfigurationProperty> {
        let root = self
            .root
            .get_or_init(|| {
                let source = &self
                    .catalog
                    .file_source
                    .as_ref()
                    .ok_or("не присоединён файловый источник")?
                    .source;
                source
                    .artifacts()
                    .ok_or("нет поверхности описаний")?
                    .description(&self.id)?
                    .ok_or_else(|| "нет файлового описания".into())
            })
            .as_ref()
            .map_err(|error| RtError::DynamicError(error.clone()))?;
        let mut node = root.as_ref();
        for &index in &self.path {
            node = &node.children[index];
        }
        Ok(node)
    }
    fn child(&self, index: usize) -> RtResult<Self> {
        let mut view = self.clone();
        view.name = self.node()?.children[index]
            .child("name")
            .map_or(Arc::from(""), |p| Arc::clone(&p.value));
        view.kind = Arc::clone(&self.node()?.children[index].name);
        view.path.push(index);
        Ok(view)
    }
    fn value(&self, context: &mut CallContext<'_>) -> RtResult<Value> {
        let node = self.node()?;
        if node.name.as_ref() == "type" {
            let attached = self
                .catalog
                .file_source
                .as_ref()
                .ok_or(RtError::NotAnObject)?;
            let runtime = &*context.runtime_shapes();
            let columns = source_types(
                attached,
                node,
                &|name| runtime.resolve_type(name),
                &mut HashSet::new(),
            )
            .map_err(RtError::DynamicError)?;
            if columns.is_empty() {
                return Err(RtError::DynamicError(
                    "не приобретён состав типов файлового реквизита".into(),
                ));
            }
            return Ok(Value::new_qualified_type_description(columns));
        }
        if matches!(
            node.name.as_ref(),
            "attributes" | "commands" | "tabularSections" | "dimensions" | "resources"
        ) {
            return Ok(Value::new_object(Collection(
                (0..node.children.len())
                    .map(|i| self.child(i))
                    .collect::<RtResult<Vec<_>>>()?,
            )));
        }
        if node.children.is_empty() {
            return Ok(match (node.name.as_ref(), node.value.as_ref()) {
                (
                    "main"
                    | "global"
                    | "server"
                    | "externalConnection"
                    | "privileged"
                    | "serverCall"
                    | "clientManagedApplication"
                    | "clientOrdinaryApplication"
                    | "savedData",
                    "true",
                ) => Value::Boolean(true),
                (
                    "main"
                    | "global"
                    | "server"
                    | "externalConnection"
                    | "privileged"
                    | "serverCall"
                    | "clientManagedApplication"
                    | "clientOrdinaryApplication"
                    | "savedData",
                    "false",
                ) => Value::Boolean(false),
                _ => Value::Str(node.value.as_ref().into()),
            });
        }
        Ok(Value::new_object(self.clone()))
    }
    fn name(&self) -> &str {
        &self.name
    }
}
impl ObjectProtocol for View {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_VALUE_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_VALUE_METHODS
    }
    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if folded_eq(name, "Имя") || folded_eq(name, "Name") {
            return Ok(Value::Str(self.name.as_ref().into()));
        }
        let wanted = property_name(name);
        if self.kind.as_ref() == "CommonModule"
            && matches!(
                wanted,
                "server"
                    | "global"
                    | "privileged"
                    | "externalConnection"
                    | "serverCall"
                    | "clientManagedApplication"
                    | "clientOrdinaryApplication"
            )
        {
            let flag = self
                .node()?
                .child(wanted)
                .map_or("false", |p| p.value.trim());
            return match flag {
                "true" => Ok(Value::Boolean(true)),
                "false" => Ok(Value::Boolean(false)),
                _ => Err(RtError::DynamicError(format!(
                    "некорректный флаг общего модуля «{name}»"
                ))),
            };
        }
        let index = self
            .node()?
            .children
            .iter()
            .position(|node| folded_eq(&node.name, wanted))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        self.child(index)?.value(context)
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((
            Arc::as_ptr(&self.catalog) as usize,
            if self.path.is_empty() {
                self.id.as_ptr() as usize
            } else {
                self.node().ok()? as *const ConfigurationProperty as usize
            },
        ))
    }
}
#[derive(Debug)]
pub(super) struct Collection(Vec<View>);
impl Collection {
    pub(super) fn find(&self, name: &str) -> Option<Value> {
        self.0
            .iter()
            .find(|view| folded_eq(view.name(), name))
            .map(|view| Value::new_object(view.clone()))
    }
}
impl ObjectProtocol for Collection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_COLLECTION_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_COLLECTION_METHODS
    }
    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.0.len())
    }
    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        self.find(name)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }
    fn get_index(&self, index: &Value) -> RtResult<Value> {
        if let Value::Str(name) = index {
            return self
                .find(&name.to_string())
                .ok_or_else(|| RtError::UnknownProperty(name.to_string()));
        }
        let Value::Number(number) = index else {
            return Err(RtError::BadIndex);
        };
        let index = number
            .trunc_to_scale(0)
            .to_i64_exact()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or(RtError::BadIndex)?;
        self.0
            .get(index)
            .map(|v| Value::new_object(v.clone()))
            .ok_or(RtError::IndexOutOfBounds {
                index: index as i64,
                len: self.0.len(),
            })
    }
}
fn property_name(name: &str) -> &str {
    for (russian, canonical) in [
        ("Имя", "name"),
        ("Комментарий", "comment"),
        ("Синоним", "synonym"),
        ("Реквизиты", "attributes"),
        ("Тип", "type"),
        ("Команды", "commands"),
        ("ТабличныеЧасти", "tabularSections"),
        ("ПутьКДанным", "dataPath"),
        ("Основной", "main"),
        ("Версия", "version"),
        ("Глобальный", "global"),
        ("Сервер", "server"),
        ("ВнешнееСоединение", "externalConnection"),
        ("Привилегированный", "privileged"),
        ("ВызовСервера", "serverCall"),
        ("КлиентУправляемоеПриложение", "clientManagedApplication"),
        ("КлиентОбычноеПриложение", "clientOrdinaryApplication"),
    ] {
        if folded_eq(name, russian) || folded_eq(name, canonical) {
            return canonical;
        }
    }
    name
}

pub(super) fn collection(
    catalog: &Arc<MetadataCatalog>,
    owner: Option<&str>,
    kind: &str,
) -> RtResult<Option<Value>> {
    let Some(attached) = &catalog.file_source else {
        return Ok(None);
    };
    let source = &attached.source;
    let artifacts = source.artifacts().ok_or(RtError::NotAnObject)?;
    let mut views = Vec::new();
    for &index in attached
        .members
        .get(owner.unwrap_or(""))
        .and_then(|kinds| kinds.get(kind))
        .into_iter()
        .flatten()
    {
        let declaration = &artifacts.descriptions()[index];
        views.push(View {
            catalog: Arc::clone(catalog),
            id: Arc::clone(&declaration.id),
            name: Arc::clone(&declaration.name),
            kind: Arc::clone(&declaration.kind),
            root: Arc::new(std::sync::OnceLock::new()),
            path: Vec::new(),
        });
    }
    Ok(Some(Value::new_object(Collection(views))))
}

pub(super) fn property(
    catalog: &Arc<MetadataCatalog>,
    id: &str,
    name: &str,
    context: &mut CallContext<'_>,
) -> RtResult<Option<Value>> {
    let Some(attached) = &catalog.file_source else {
        return Ok(None);
    };
    let source = &attached.source;
    let kind = if folded_eq(name, "Формы") || folded_eq(name, "Forms") {
        Some("Form")
    } else if folded_eq(name, "Макеты") || folded_eq(name, "Templates") {
        Some("Template")
    } else {
        None
    };
    if let Some(kind) = kind {
        return collection(catalog, Some(id), kind);
    }
    let Some(root) = source
        .artifacts()
        .ok_or(RtError::NotAnObject)?
        .description(id)
        .map_err(RtError::DynamicError)?
    else {
        return Ok(None);
    };
    let wanted = property_name(name);
    if matches!(
        wanted,
        "attributes" | "tabularSections" | "dimensions" | "resources" | "type"
    ) && catalog.object_by_id(id).is_some()
    {
        catalog.materialized_object_by_id(id)?;
    }
    let Some(index) = root
        .children
        .iter()
        .position(|node| folded_eq(&node.name, wanted))
    else {
        return Ok(None);
    };
    let cache = std::sync::OnceLock::new();
    cache.set(Ok(root)).expect("новый кэш пуст");
    View {
        catalog: Arc::clone(catalog),
        id: id.into(),
        name: "".into(),
        kind: "property".into(),
        root: Arc::new(cache),
        path: vec![index],
    }
    .value(context)
    .map(Some)
}

pub(super) fn template(
    catalog: &Arc<MetadataCatalog>,
    owner: Option<&str>,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Option<Value>> {
    let Some(attached) = &catalog.file_source else {
        return Ok(None);
    };
    let source = &attached.source;
    let Some(Value::Str(name)) = arguments.first() else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПолучитьМакет",
        });
    };
    let kind = if owner.is_none() {
        "CommonTemplate"
    } else {
        "Template"
    };
    let artifacts = source.artifacts().ok_or(RtError::NotAnObject)?;
    let declaration = attached
        .members
        .get(owner.unwrap_or(""))
        .and_then(|kinds| kinds.get(kind))
        .into_iter()
        .flatten()
        .map(|&index| &artifacts.descriptions()[index])
        .find(|d| folded_eq(&d.name, &name.to_string()))
        .ok_or_else(|| {
            RtError::DynamicError(format!("макет «{name}» отсутствует в файловом источнике"))
        })?;
    let artifact = artifacts
        .artifact(&declaration.id)
        .map_err(RtError::DynamicError)?
        .ok_or_else(|| RtError::DynamicError("нет содержимого файлового макета".into()))?;
    open_bsl::configuration_template_value(&artifact, context).map(Some)
}

pub(super) fn receiver_template(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Option<Value>> {
    let owner = if let Some(manager) = receiver.downcast_ref::<ManagerValue>() {
        Some((&manager.catalog, manager.object_id.as_ref()))
    } else {
        document_object::file_owner(receiver)
            .or_else(|| catalog_object::file_owner(receiver))
            .or_else(|| data_processor_object::file_owner(receiver))
    };
    let Some((catalog, id)) = owner else {
        return Ok(None);
    };
    template(catalog, Some(id), arguments, context)
}

pub(super) fn forms(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    collection(&value.catalog, Some(&value.object_id), "Form")?
        .ok_or_else(|| RtError::UnknownProperty("Формы".into()))
}
pub(super) fn templates(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    collection(&value.catalog, Some(&value.object_id), "Template")?
        .ok_or_else(|| RtError::UnknownProperty("Макеты".into()))
}
pub(super) fn common_forms(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    collection(&value.catalog, None, "CommonForm")?
        .ok_or_else(|| RtError::UnknownProperty("ОбщиеФормы".into()))
}
pub(super) fn common_templates(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    collection(&value.catalog, None, "CommonTemplate")?
        .ok_or_else(|| RtError::UnknownProperty("ОбщиеМакеты".into()))
}

pub(super) fn root_enum(catalog: &Arc<MetadataCatalog>, name: &str) -> RtResult<Option<Value>> {
    let Some(attached) = &catalog.file_source else {
        return Ok(None);
    };
    let (property, kind) = if folded_eq(name, "ВариантВстроенногоЯзыка")
        || folded_eq(name, "ScriptVariant")
    {
        ("scriptVariant", EnumKind::ScriptVariant)
    } else if folded_eq(name, "РежимСовместимости") || folded_eq(name, "CompatibilityMode")
    {
        ("compatibilityMode", EnumKind::CompatibilityMode)
    } else {
        return Ok(None);
    };
    let artifacts = attached.source.artifacts().ok_or(RtError::NotAnObject)?;
    let root = artifacts
        .description(artifacts.root_identity())
        .map_err(RtError::DynamicError)?
        .ok_or_else(|| RtError::DynamicError("нет файлового описания конфигурации".into()))?;
    let value = root.child(property).ok_or_else(|| {
        RtError::DynamicError(format!(
            "в файловой конфигурации не приобретено свойство «{name}»"
        ))
    })?;
    let value = value.value.trim();
    let canonical = if property == "compatibilityMode" && value.starts_with("8.") {
        format!("Version{}", value.replace('.', "_"))
    } else {
        value.into()
    };
    lookup_member(kind, &canonical)
        .map(Value::Enum)
        .map(Some)
        .ok_or_else(|| {
            RtError::DynamicError(format!("не поддержано файловое свойство «{name}»: {value}"))
        })
}

#[cfg(test)]
#[path = "file_source_qualifier_tests.rs"]
mod qualifier_tests;
