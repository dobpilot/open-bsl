//! Собственный объект обработки на общей памяти экземпляра конфигурации.
use super::*;
use open_bsl::{ConfigurationInstance, TypeId};

#[derive(Debug)]
struct Data {
    descriptor: &'static TypeDescriptor,
    catalog: Arc<MetadataCatalog>,
    owner: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    attributes: RefCell<Vec<Value>>,
    sections: Vec<Value>,
    instance: ConfigurationInstance,
}

#[derive(Debug, Clone)]
struct Object(Rc<Data>);

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОбработкаОбъект",
    type_display: "Обработка объект",
    type_names: &["DataProcessorObject"],
};

pub(super) static REPORT_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОтчетОбъект",
    type_display: "Отчет объект",
    type_names: &["ReportObject"],
};

impl Object {
    fn metadata(&self) -> RtResult<&MetadataObject> {
        self.0
            .catalog
            .materialized_object_by_id(&self.0.owner)?
            .ok_or(RtError::NotAnObject)
    }
}

impl ObjectProtocol for Object {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.0.descriptor
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.0.ty))
    }
    fn configuration_instance(&self) -> Option<&ConfigurationInstance> {
        Some(&self.0.instance)
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        if self.0.descriptor.name == REPORT_TYPE.name {
            REPORT_PROPERTIES
        } else {
            &[]
        }
    }
    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if self.property_table().iter().any(|property| {
            property
                .names
                .iter()
                .any(|candidate| folded_eq(candidate, name))
        }) {
            return get_property_from_table(
                self.property_table(),
                REPORT_TYPE.name,
                self,
                name,
                context,
            );
        }
        let metadata = self.metadata()?;
        if let Some(index) = metadata
            .executable_sections()
            .into_iter()
            .flatten()
            .position(|section| folded_eq(section.name(), name))
        {
            return Ok(self.0.sections[index].clone());
        }
        let index = metadata
            .executable_attributes()
            .into_iter()
            .flatten()
            .position(|attribute| folded_eq(attribute.name(), name));
        if let Some(index) = index {
            return Ok(self.0.attributes.borrow()[index].clone());
        }
        let module = metadata
            .object_module()
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        context.read_configuration_instance_variable(module, &self.0.instance, name)
    }
    fn set_property(
        &self,
        name: &str,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        if self.property_table().iter().any(|property| {
            property
                .names
                .iter()
                .any(|candidate| folded_eq(candidate, name))
        }) {
            return set_property_from_table(
                self.property_table(),
                REPORT_TYPE.name,
                self,
                name,
                value,
                context,
            );
        }
        let metadata = self.metadata()?;
        if metadata
            .executable_sections()
            .into_iter()
            .flatten()
            .any(|section| folded_eq(section.name(), name))
        {
            return Err(RtError::PropertyReadOnly {
                property: name.into(),
                receiver: TYPE.name,
            });
        }
        let attributes = metadata
            .executable_attributes()
            .ok_or(RtError::NotAnObject)?;
        let Some(index) = attributes
            .iter()
            .position(|attribute| folded_eq(attribute.name(), name))
        else {
            let module = metadata
                .object_module()
                .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
            return context.write_configuration_instance_variable(
                module,
                &self.0.instance,
                name,
                value,
            );
        };
        self.0.attributes.borrow_mut()[index] = document_object::adjust_attribute_value(
            value,
            &attributes[index],
            &self.0.catalog,
            context,
        )?;
        Ok(())
    }
    fn has_method(&self, name: &str) -> bool {
        method_exists(METHODS, name)
            || self
                .metadata()
                .is_ok_and(|metadata| metadata.object_module().is_some())
    }
    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        // Как у объекта документа, проверяем экспорт до новой платформенной
        // декларации. Статическая таблица VM обошла бы эту проверку, поэтому
        // экземпляр использует общий динамический путь `ObjectProtocol`.
        let module_export = if (folded_eq(name, "ПолучитьМакет") || folded_eq(name, "GetTemplate"))
            && let Some(module) = self.metadata()?.object_module()
        {
            context.configuration_has_export(module, name)?
        } else {
            false
        };
        if method_exists(METHODS, name) && !module_export {
            return call_method_from_table(METHODS, TYPE.name, self, name, args, context);
        }
        let module = self
            .metadata()?
            .object_module()
            .ok_or_else(|| RtError::UnknownMethod {
                method: name.into(),
                receiver: TYPE.name,
            })?;
        context
            .call_configuration_instance(
                module,
                &self.0.instance,
                name,
                args.to_vec(),
                Value::new_object(self.clone()),
            )
            .map(|(value, _)| value)
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.0) as usize, 0))
    }
}

pub(super) fn create(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let metadata = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or(RtError::NotAnObject)?;
    let label = if metadata.kind() == MetadataKind::Report {
        "отчёта"
    } else {
        "обработки"
    };
    let attributes = metadata.executable_attributes().ok_or_else(|| {
        RtError::DynamicError(format!(
            "собственные реквизиты {label} неизвестны в снимке конфигурации"
        ))
    })?;
    let sections = metadata.executable_sections().ok_or_else(|| {
        RtError::DynamicError(format!(
            "табличные части {label} неизвестны в снимке конфигурации"
        ))
    })?;
    // Контроль обработки измеряет строку и булево; другие типы не получают
    // правила преобразования по аналогии с документом.
    for attribute in attributes.iter().chain(
        sections
            .iter()
            .flat_map(|section| section.attributes().into_iter().flatten()),
    ) {
        if !matches!(attribute.types(), [column] if matches!(column.id,TypeRef::Native(TypeId::String|TypeId::Boolean)))
        {
            return Err(RtError::DynamicError(format!(
                "тип реквизита обработки «{}» пока не поддерживается",
                attribute.name()
            )));
        }
    }
    let ty = manager
        .catalog
        .data_processor_object_types
        .iter()
        .chain(manager.catalog.report_object_types.iter())
        .find(|(id, _)| id.as_ref() == manager.object_id.as_ref())
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or(RtError::NotAnObject)?;
    let values = attributes
        .iter()
        .map(|attribute| {
            document_object::adjust_attribute_value(
                Value::Undefined,
                attribute,
                &manager.catalog,
                context,
            )
        })
        .collect::<RtResult<Vec<_>>>()?;
    let sections = sections
        .iter()
        .map(|section| {
            document_tabular_section::create(&manager.catalog, &manager.object_id, section)
        })
        .collect::<RtResult<Vec<_>>>()?;
    let object = Object(Rc::new(Data {
        descriptor: if metadata.kind() == MetadataKind::Report {
            &REPORT_TYPE
        } else {
            &TYPE
        },
        catalog: Arc::clone(&manager.catalog),
        owner: Arc::clone(&manager.object_id),
        ty,
        attributes: RefCell::new(values),
        sections,
        instance: ConfigurationInstance::new(),
    }));
    let value = Value::new_object(object.clone());
    if let Some(module) = metadata.object_module() {
        context.initialize_configuration_instance(module, &object.0.instance, value.clone())?;
    }
    // В отличие от документа, нативный объект обработки сохраняет строки,
    // добавленные телом модуля; повторная инициализация частей не нужна.
    Ok(value)
}

fn metadata(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = receiver
        .downcast_ref::<Object>()
        .ok_or(RtError::NotAnObject)?;
    Ok(metadata_object_value(&object.0.catalog, object.metadata()?))
}
pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Метаданные", "Metadata"], Arity::exact(0), metadata),
    // Собственный объект объявляет ту же сигнатуру, что менеджер обработки.
    // До приобретения ресурсов общий обработчик явно отклоняет получение макета.
    DATA_PROCESSOR_MANAGER_METHODS[0],
];

fn settings_composer(_: &dyn ObjectProtocol, context: &mut CallContext<'_>) -> RtResult<Value> {
    // Свойство уже нужно при компиляции обработчиков отчёта. Пока модель
    // настроек не реализована, сохраняем явный отказ существующего конструктора.
    new_data_composition_settings_composer(context, &[])
}

fn composition_schema(_: &dyn ObjectProtocol, context: &mut CallContext<'_>) -> RtResult<Value> {
    // Без приобретённого основного макета отсутствие схемы неизвестно:
    // возвращать `Неопределено` вместо явного отказа нельзя.
    new_data_composition_schema(context, &[])
}

fn set_composition_schema(
    _: &dyn ObjectProtocol,
    _: Value,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    // Платформа разрешает запись; до реализации модели не сохраняем
    // произвольное значение как успешно установленную схему.
    new_data_composition_schema(context, &[]).map(|_| ())
}

pub(super) static REPORT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["КомпоновщикНастроек", "SettingsComposer"],
        get: settings_composer,
        set: None,
    },
    PropertyDescriptor {
        names: &["СхемаКомпоновкиДанных", "DataCompositionSchema"],
        get: composition_schema,
        set: Some(set_composition_schema),
    },
];

/// Возвращает владельца для общего чтения файлового макета.
pub(super) fn file_owner(receiver: &dyn ObjectProtocol) -> Option<(&Arc<MetadataCatalog>, &str)> {
    let object = receiver.downcast_ref::<Object>()?;
    Some((&object.0.catalog, object.0.owner.as_ref()))
}
