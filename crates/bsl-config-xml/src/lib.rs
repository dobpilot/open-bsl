//! Чтение нативной файловой конфигурации с приобретением ресурсов по обращению.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use bsl_config::{
    ConfigurationArtifact, ConfigurationArtifactKind, ConfigurationArtifacts,
    ConfigurationDescription, ConfigurationIndex, ConfigurationModuleDeclaration,
    ConfigurationProperty, ConfigurationSource, ModuleExecution, ModuleKind,
};

mod resource;
mod tree;
use resource::Resource;
use tree::Node;

/// Формат нативного каталога; EDT использует тот же XML-парсер и кэш ресурсов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationFileFormat {
    /// Выгрузка Конфигуратора с `Configuration.xml`.
    ConfiguratorXml,
    /// Проект EDT с `Configuration/Configuration.mdo` внутри `src`.
    Edt,
}

enum ModuleBody {
    Text(Arc<Resource>),
    Protected(Arc<Resource>),
    Missing(PathBuf),
}

struct Entry {
    declaration: ConfigurationDescription,
    descriptor: Arc<Resource>,
    inline: bool,
    properties: OnceLock<Result<Arc<ConfigurationProperty>, String>>,
    payload: Option<(
        ConfigurationArtifactKind,
        String,
        Result<Arc<Resource>, String>,
    )>,
    artifact: OnceLock<Result<Arc<ConfigurationArtifact>, String>>,
}

/// Один индекс файловой конфигурации. БД и изменяемые сеансы ему не принадлежат.
pub struct XmlConfigurationSource {
    index: ConfigurationIndex,
    root_id: String,
    format: ConfigurationFileFormat,
    descriptions: Vec<ConfigurationDescription>,
    entries: HashMap<String, Entry>,
    modules: HashMap<String, ModuleBody>,
}

impl XmlConfigurationSource {
    /// Индексирует выгрузку Конфигуратора, не читая тела модулей и макеты.
    ///
    /// # Errors
    /// Возвращает ошибку каталога, нативного описания или повторной идентичности.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, String> {
        Self::open_format(root, ConfigurationFileFormat::ConfiguratorXml)
    }

    /// Общая точка файлового движка для адаптера EDT.
    ///
    /// # Errors
    /// Возвращает ошибку каталога, нативного описания или повторной идентичности.
    pub fn open_format(
        root: impl AsRef<Path>,
        format: ConfigurationFileFormat,
    ) -> Result<Self, String> {
        let mut root = root
            .as_ref()
            .canonicalize()
            .map_err(|error| format!("{}: {error}", root.as_ref().display()))?;
        if format == ConfigurationFileFormat::Edt
            && !root.join("Configuration/Configuration.mdo").is_file()
        {
            root = root
                .join("src")
                .canonicalize()
                .map_err(|error| error.to_string())?;
        }
        let root_path = root.join(if format == ConfigurationFileFormat::Edt {
            "Configuration/Configuration.mdo"
        } else {
            "Configuration.xml"
        });
        let mut builder = Builder {
            root,
            format,
            descriptions: Vec::new(),
            entries: HashMap::new(),
            modules: Vec::new(),
            bodies: HashMap::new(),
        };
        let resource = builder.resource(&root_path)?;
        let node = builder.node(&resource)?;
        let config = if format == ConfigurationFileFormat::Edt {
            if node.name != "Configuration" || node.uri != "http://g5.1c.ru/v8/dt/metadata/mdclass"
            {
                return Err("неподдержанное описание конфигурации EDT".into());
            }
            &node
        } else {
            if node.name != "MetaDataObject" || node.uri != "http://v8.1c.ru/8.3/MDClasses" {
                return Err("неподдержанная выгрузка Конфигуратора".into());
            }
            if node.attr("version") != Some("2.20") {
                return Err(format!(
                    "неподдержанная версия XML-выгрузки: {:?}",
                    node.attr("version")
                ));
            }
            node.child("Configuration")
                .ok_or("нет корневого объекта Configuration")?
        };
        let root_id = uuid(config)?.to_ascii_lowercase();
        builder.entry(config, None, "Configuration", resource.clone(), false, None)?;
        if format == ConfigurationFileFormat::Edt {
            // EDT помещает языки непосредственно в описание конфигурации,
            // в отличие от отдельных `Languages/*.xml` Конфигуратора.
            for language in config
                .children
                .iter()
                .filter(|node| node.name == "languages")
            {
                builder.entry(language, None, "Language", resource.clone(), true, None)?;
            }
        }
        let session_path = builder
            .root
            .join(if format == ConfigurationFileFormat::Edt {
                "Configuration/SessionModule.bsl"
            } else {
                "Ext/SessionModule.bsl"
            });
        builder.module(
            &root_id,
            "МодульСеанса",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
            &session_path,
            false,
        )?;
        let references: Vec<(String, String)> = if format == ConfigurationFileFormat::Edt {
            config
                .children
                .iter()
                .filter_map(|child| {
                    let (kind, name) = child.text.trim().split_once('.')?;
                    (plural(kind).eq_ignore_ascii_case(&child.name))
                        .then(|| (kind.to_string(), name.to_string()))
                })
                .collect()
        } else {
            config
                .child("ChildObjects")
                .map(|children| {
                    children
                        .children
                        .iter()
                        .map(|child| (child.name.clone(), child.text.trim().to_string()))
                        .collect()
                })
                .unwrap_or_default()
        };
        for (kind, name) in references {
            let directory = builder.root.join(plural(&kind));
            builder.object(&kind, &name, &directory, None)?;
        }
        let index = ConfigurationIndex::new(&root_id, builder.modules)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            index,
            root_id,
            format,
            descriptions: builder.descriptions,
            entries: builder.entries,
            modules: builder.bodies,
        })
    }
}

impl ConfigurationSource for XmlConfigurationSource {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        Some(self)
    }
    fn module_image(&self, id: &str) -> Result<Option<Arc<[u8]>>, String> {
        match self.modules.get(id) {
            Some(ModuleBody::Protected(resource)) => resource.read().map(Some),
            Some(ModuleBody::Text(resource)) => resource
                .module_image()
                .map_err(|error| format!("{} [{id}]: {error}", resource.path.display())),
            _ => Ok(None),
        }
    }
    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        match self
            .modules
            .get(id)
            .ok_or_else(|| format!("модуль «{id}» отсутствует в файловом источнике"))?
        {
            ModuleBody::Text(resource) => resource
                .module_text()
                .map_err(|error| format!("{} [{id}]: {error}", resource.path.display())),
            ModuleBody::Missing(path) => {
                Err(format!("нет BSL-текста модуля «{id}»: {}", path.display()))
            }
            ModuleBody::Protected(resource) => Err(format!(
                "нет BSL-текста модуля «{id}»: {}",
                resource.path.display()
            )),
        }
    }
}

impl ConfigurationArtifacts for XmlConfigurationSource {
    fn root_identity(&self) -> &str {
        &self.root_id
    }
    fn descriptions(&self) -> &[ConfigurationDescription] {
        &self.descriptions
    }
    fn description(&self, id: &str) -> Result<Option<Arc<ConfigurationProperty>>, String> {
        let Some(entry) = self.entries.get(id) else {
            return Ok(None);
        };
        entry
            .properties
            .get_or_init(|| {
                let node = tree::parse(&entry.descriptor.index_text()?).map_err(|error| {
                    format!("{} [{id}]: {error}", entry.descriptor.path.display())
                })?;
                let node = if entry.inline {
                    node.find_uuid(id)
                        .ok_or_else(|| format!("описание «{id}» исчезло из источника"))?
                } else if node.name == "MetaDataObject" {
                    node.children.first().ok_or("пустой MetaDataObject")?
                } else {
                    &node
                };
                let mut properties = node.description();
                if let Some((ConfigurationArtifactKind::Form, _, payload)) = &entry.payload {
                    let payload = payload.as_ref().map_err(Clone::clone)?;
                    let content = tree::parse(&resource::decode(&payload.read()?)?)
                        .map_err(|error| format!("{} [{id}]: {error}", payload.path.display()))?;
                    let expected_uri = match self.format {
                        ConfigurationFileFormat::ConfiguratorXml => {
                            "http://v8.1c.ru/8.3/xcf/logform"
                        }
                        ConfigurationFileFormat::Edt => "http://g5.1c.ru/v8/dt/form",
                    };
                    if content.name != "Form" || content.uri != expected_uri {
                        return Err(format!(
                            "{}: неподдержанное описание формы",
                            payload.path.display()
                        ));
                    }
                    let mut children = properties.children.to_vec();
                    children.extend(content.description().children.iter().cloned());
                    properties.children = children.into();
                }
                Ok(Arc::new(properties))
            })
            .clone()
            .map(Some)
    }
    fn artifact(&self, id: &str) -> Result<Option<Arc<ConfigurationArtifact>>, String> {
        let Some(entry) = self.entries.get(id) else {
            return Ok(None);
        };
        let Some((kind, representation, resource)) = &entry.payload else {
            return Ok(None);
        };
        if *kind == ConfigurationArtifactKind::Form {
            self.description(id)?;
        }
        entry
            .artifact
            .get_or_init(|| {
                Ok(Arc::new(ConfigurationArtifact {
                    description: entry.declaration.clone(),
                    kind: *kind,
                    representation: representation.as_str().into(),
                    bytes: resource.as_ref().map_err(Clone::clone)?.read()?,
                }))
            })
            .clone()
            .map(Some)
    }
}

struct Builder {
    root: PathBuf,
    format: ConfigurationFileFormat,
    descriptions: Vec<ConfigurationDescription>,
    entries: HashMap<String, Entry>,
    modules: Vec<ConfigurationModuleDeclaration>,
    bodies: HashMap<String, ModuleBody>,
}
impl Builder {
    fn resource(&self, path: &Path) -> Result<Arc<Resource>, String> {
        Resource::new(&self.root, path)
    }
    fn node(&self, resource: &Resource) -> Result<Node, String> {
        tree::parse(&resource.index_text()?)
            .map_err(|error| format!("{}: {error}", resource.path.display()))
    }
    fn entry(
        &mut self,
        node: &Node,
        owner: Option<&str>,
        kind: &str,
        descriptor: Arc<Resource>,
        inline: bool,
        payload: Option<(
            ConfigurationArtifactKind,
            String,
            Result<Arc<Resource>, String>,
        )>,
    ) -> Result<(), String> {
        let id = uuid(node)?.to_ascii_lowercase();
        let properties = node.child("Properties").unwrap_or(node);
        let name = properties
            .value("Name")
            .ok_or_else(|| format!("нет имени объекта «{id}»"))?;
        let declaration = ConfigurationDescription {
            id: id.as_str().into(),
            owner: owner.map(Arc::from),
            kind: kind.into(),
            name: name.into(),
        };
        if self.entries.contains_key(&id) {
            return Err(format!("повтор идентичности «{id}» в файловом источнике"));
        }
        self.descriptions.push(declaration.clone());
        self.entries.insert(
            id,
            Entry {
                declaration,
                descriptor,
                inline,
                properties: OnceLock::new(),
                payload,
                artifact: OnceLock::new(),
            },
        );
        Ok(())
    }
    fn module(
        &mut self,
        owner: &str,
        name: &str,
        kind: ModuleKind,
        execution: ModuleExecution,
        path: &Path,
        required: bool,
    ) -> Result<(), String> {
        let body = if path.is_file() {
            ModuleBody::Text(self.resource(path)?)
        } else if path.with_extension("bin").is_file() {
            ModuleBody::Protected(self.resource(&path.with_extension("bin"))?)
        } else if required {
            // Отсутствующий текст объявленного общего модуля не подменяется
            // заглушкой. Ошибка возникает при приобретении выбранного тела.
            ModuleBody::Missing(path.to_path_buf())
        } else {
            return Ok(());
        };
        let id = format!("{owner}/{kind:?}");
        let module = ConfigurationModuleDeclaration::new(&id, name, kind, execution);
        let module = if kind == ModuleKind::Session || kind == ModuleKind::Common {
            module
        } else {
            module.with_owner(owner)
        };
        self.modules.push(module);
        self.bodies.insert(id, body);
        Ok(())
    }
    fn object(
        &mut self,
        kind: &str,
        name: &str,
        directory: &Path,
        parent: Option<&str>,
    ) -> Result<(), String> {
        component(name)?;
        let base = directory.join(name);
        let path = if self.format == ConfigurationFileFormat::Edt {
            base.join(format!("{name}.mdo"))
        } else {
            base.with_extension("xml")
        };
        let resource = self.resource(&path)?;
        let parsed = self.node(&resource)?;
        let node = if self.format == ConfigurationFileFormat::Edt {
            &parsed
        } else {
            parsed
                .child(kind)
                .ok_or_else(|| format!("{}: нет описания {kind}", path.display()))?
        };
        if node.name != kind {
            return Err(format!(
                "{}: вид объекта не совпадает с {kind}",
                path.display()
            ));
        }
        self.object_node(kind, node, &base, resource, parent)
    }
    fn object_node(
        &mut self,
        kind: &str,
        node: &Node,
        base: &Path,
        resource: Arc<Resource>,
        parent: Option<&str>,
    ) -> Result<(), String> {
        let id = uuid(node)?.to_ascii_lowercase();
        self.entry(node, parent, kind, resource.clone(), false, None)?;
        let props = node.child("Properties").unwrap_or(node);
        let name = props.value("Name").ok_or("нет имени объекта")?;
        let ext = if self.format == ConfigurationFileFormat::Edt {
            base.to_path_buf()
        } else {
            base.join("Ext")
        };
        if kind == "CommonModule" {
            let execution = ModuleExecution::from_properties(&node.description())
                .map_err(|error| format!("{id}: {error}"))?;
            self.module(
                &id,
                name,
                ModuleKind::Common,
                execution,
                &ext.join("Module.bsl"),
                props.value("ObjectBelonging") != Some("Adopted"),
            )?;
        } else if let Some(prefix) = module_prefix(kind) {
            for (file, module_kind, label) in [
                ("ManagerModule.bsl", ModuleKind::Manager, "МодульМенеджера"),
                (
                    if kind == "Constant" {
                        "ValueManagerModule.bsl"
                    } else {
                        "ObjectModule.bsl"
                    },
                    ModuleKind::Object,
                    "МодульОбъекта",
                ),
                (
                    "RecordSetModule.bsl",
                    ModuleKind::RecordSet,
                    "МодульНабораЗаписей",
                ),
            ] {
                self.module(
                    &id,
                    &format!("{prefix}.{name}.{label}"),
                    module_kind,
                    ModuleExecution::new().with_server(true),
                    &ext.join(file),
                    false,
                )?;
            }
        }
        let children = node.child("ChildObjects");
        if self.format == ConfigurationFileFormat::Edt {
            for child in &node.children {
                if kind == "Subsystem" && child.name == "subsystems" {
                    // Вложенная подсистема в нативном EDT хранится коротким
                    // именем; корневые ссылки содержат квалификатор вида объекта.
                    let reference = child.text.trim();
                    let name = reference
                        .strip_prefix("Subsystem.")
                        .unwrap_or(reference)
                        .rsplit('.')
                        .next()
                        .ok_or("пустая ссылка подсистемы EDT")?;
                    self.object("Subsystem", name, &base.join("Subsystems"), Some(&id))?;
                    continue;
                }
                let member_kind = match child.name.as_str() {
                    "forms" => "Form",
                    "templates" => "Template",
                    _ => continue,
                };
                let member_name = child.value("name").ok_or("нет имени вложенного объекта")?;
                component(member_name)?;
                self.member(
                    member_kind,
                    child,
                    &base.join(plural(member_kind)).join(member_name),
                    resource.clone(),
                    true,
                    &id,
                )?;
            }
        } else if let Some(children) = children {
            for child in &children.children {
                if child.name == "Form" || child.name == "Template" {
                    let member_name = child.text.trim();
                    component(member_name)?;
                    let member_base = base.join(plural(&child.name)).join(member_name);
                    let member_resource = self.resource(&member_base.with_extension("xml"))?;
                    let member_node = self.node(&member_resource)?;
                    let member_node = member_node
                        .child(&child.name)
                        .ok_or("нет описания вложенного объекта")?;
                    self.member(
                        &child.name,
                        member_node,
                        &member_base,
                        member_resource.clone(),
                        false,
                        &id,
                    )?;
                } else if child.name == "Subsystem" {
                    self.object(
                        "Subsystem",
                        child.text.trim(),
                        &base.join("Subsystems"),
                        Some(&id),
                    )?;
                }
            }
        }
        if kind == "CommonForm" || kind == "CommonTemplate" {
            self.common_payload(kind, &id, props, base)?;
        }
        Ok(())
    }
    fn member(
        &mut self,
        kind: &str,
        node: &Node,
        base: &Path,
        resource: Arc<Resource>,
        inline: bool,
        owner: &str,
    ) -> Result<(), String> {
        let id = uuid(node)?.to_ascii_lowercase();
        self.entry(node, Some(owner), kind, resource, inline, None)?;
        let props = node.child("Properties").unwrap_or(node);
        self.common_payload(kind, &id, props, base)
    }
    fn common_payload(
        &mut self,
        kind: &str,
        id: &str,
        props: &Node,
        base: &Path,
    ) -> Result<(), String> {
        let ext = if self.format == ConfigurationFileFormat::Edt {
            base.to_path_buf()
        } else {
            base.join("Ext")
        };
        let (artifact_kind, representation, candidates) = if kind == "Form" || kind == "CommonForm"
        {
            (
                ConfigurationArtifactKind::Form,
                "Form".to_string(),
                vec![if self.format == ConfigurationFileFormat::Edt {
                    "Form.form".to_string()
                } else {
                    "Form.xml".to_string()
                }],
            )
        } else {
            // Нативный импорт EDT 2026.1 опускает `templateType` у табличного
            // макета; парная XML/EDT-фикстура закрепляет это значение по умолчанию.
            let representation = props
                .value("TemplateType")
                .or_else(|| {
                    (self.format == ConfigurationFileFormat::Edt).then_some("SpreadsheetDocument")
                })
                .ok_or_else(|| format!("{id}: не указан вид макета"))?
                .to_string();
            let suffix = match representation.as_str() {
                "TextDocument" | "FileAwareTextDocument" => "txt",
                "SpreadsheetDocument" => "mxlx",
                "DataCompositionSchema" => "dcs",
                "BinaryData" => "bin",
                "HTMLDocument" | "HtmlDocument" => "htmldoc",
                "ActiveDocument" => "axdt",
                "DataCompositionAppearanceTemplate" => "dcsat",
                "GraphicalSchema" => "scheme",
                "GeographicalSchema" => "geos",
                "AddIn" => "addin",
                _ => {
                    return Err(format!(
                        "{id}: неподдержанный вид макета «{representation}»"
                    ));
                }
            };
            let file = if self.format == ConfigurationFileFormat::Edt {
                format!("Template.{suffix}")
            } else {
                match representation.as_str() {
                    "TextDocument" | "FileAwareTextDocument" => "Template.txt".into(),
                    "BinaryData" => "Template.bin".into(),
                    _ => "Template.xml".into(),
                }
            };
            (
                ConfigurationArtifactKind::Template,
                representation,
                vec![file],
            )
        };
        // Ошибка отсутствующего содержимого относится к приобретению ресурса,
        // а не к индексу: нативная выгрузка может объявлять неиспользуемый макет
        // без файла. Ошибка сохраняется и не превращается в пустое значение.
        let payload = candidates
            .iter()
            .map(|name| ext.join(name))
            .find(|path| path.is_file())
            .ok_or_else(|| format!("{id}: нет содержимого ресурса в {}", ext.display()))
            .and_then(|path| self.resource(&path));
        self.entries
            .get_mut(id)
            .expect("объявление добавлено")
            .payload = Some((artifact_kind, representation, payload));
        if artifact_kind == ConfigurationArtifactKind::Form {
            let declaration = self.entries[id].declaration.clone();
            self.module(
                id,
                &format!(
                    "Форма.{}.{}",
                    declaration.owner.as_deref().unwrap_or("Общая"),
                    declaration.name
                ),
                ModuleKind::Form,
                ModuleExecution::new().with_client(true),
                &ext.join(if self.format == ConfigurationFileFormat::ConfiguratorXml {
                    "Form/Module.bsl"
                } else {
                    "Module.bsl"
                }),
                false,
            )?;
        }
        Ok(())
    }
}

fn uuid(node: &Node) -> Result<&str, String> {
    node.attr("uuid")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| format!("у {} отсутствует UUID", node.name))
}
fn component(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return Err(format!("недопустимое имя компонента пути «{name}»"));
    }
    Ok(())
}
fn plural(kind: &str) -> String {
    match kind {
        "BusinessProcess" => "BusinessProcesses".into(),
        "FilterCriterion" => "FilterCriteria".into(),
        "ChartOfAccounts" => "ChartsOfAccounts".into(),
        "ChartOfCharacteristicTypes" => "ChartsOfCharacteristicTypes".into(),
        "ChartOfCalculationTypes" => "ChartsOfCalculationTypes".into(),
        _ => format!("{kind}s"),
    }
}
fn module_prefix(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "Catalog" => "Справочник",
        "Document" => "Документ",
        "Constant" => "Константа",
        "InformationRegister" => "РегистрСведений",
        "AccumulationRegister" => "РегистрНакопления",
        "AccountingRegister" => "РегистрБухгалтерии",
        "CalculationRegister" => "РегистрРасчета",
        "ChartOfAccounts" => "ПланСчетов",
        "ChartOfCharacteristicTypes" => "ПланВидовХарактеристик",
        "ChartOfCalculationTypes" => "ПланВидовРасчета",
        "ExchangePlan" => "ПланОбмена",
        "DocumentJournal" => "ЖурналДокументов",
        "Report" => "Отчет",
        "DataProcessor" => "Обработка",
        "BusinessProcess" => "БизнесПроцесс",
        "Task" => "Задача",
        "SettingsStorage" => "ХранилищеНастроек",
        _ => return None,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
