//! Табличная часть документа: собственные типы поверх общего хранения строк.

use super::*;
use open_bsl::{ColumnType, TypeId};

#[derive(Debug)]
pub(super) struct Types {
    owner: Arc<str>,
    section: Arc<str>,
    pub(super) section_type: Arc<ValueTypeDescriptor>,
    pub(super) row_type: Arc<ValueTypeDescriptor>,
}

pub(super) fn types(identity: &str, objects: &[MetadataObject]) -> Vec<Types> {
    owner_types(identity, objects, MetadataKind::Document)
}

pub(super) fn business_types(identity: &str, objects: &[MetadataObject]) -> Vec<Types> {
    owner_types(identity, objects, MetadataKind::BusinessProcess)
}

pub(super) fn processor_types(identity: &str, objects: &[MetadataObject]) -> Vec<Types> {
    owner_types(identity, objects, MetadataKind::DataProcessor)
}

pub(super) fn report_types(identity: &str, objects: &[MetadataObject]) -> Vec<Types> {
    owner_types(identity, objects, MetadataKind::Report)
}

// Обновление одного владельца сохраняет уже построенные описания остальных.
// Порядок остаётся порядком объектов и частей, как при полной сборке каталога.
pub(super) fn update_types(
    identity: &str,
    objects: &[MetadataObject],
    index: usize,
    types: &mut Vec<Types>,
) {
    let object = &objects[index];
    let kind = object.kind();
    let start = objects[..index]
        .iter()
        .filter(|object| object.kind() == kind)
        .filter_map(|object| sections(object, kind))
        .map(<[DocumentTabularSection]>::len)
        .sum::<usize>();
    let old_len = types[start..]
        .iter()
        .take_while(|entry| entry.owner.as_ref() == object.id())
        .count();
    types.splice(
        start..start + old_len,
        owner_types(identity, std::slice::from_ref(object), kind),
    );
}

fn sections(object: &MetadataObject, kind: MetadataKind) -> Option<&[DocumentTabularSection]> {
    match kind {
        MetadataKind::BusinessProcess => object.business_process_tabular_sections(),
        MetadataKind::DataProcessor => object.data_processor_tabular_sections(),
        MetadataKind::Report => object.report_tabular_sections(),
        _ => object.document_tabular_sections(),
    }
}

fn owner_types(identity: &str, objects: &[MetadataObject], kind: MetadataKind) -> Vec<Types> {
    let (key, ru, en, display) = match kind {
        MetadataKind::BusinessProcess => (
            "business-process",
            "БизнесПроцесс",
            "BusinessProcess",
            "Бизнес-процесс",
        ),
        MetadataKind::Report => ("report", "Отчет", "Report", "Report"),
        MetadataKind::DataProcessor => {
            ("data-processor", "Обработка", "DataProcessor", "Обработка")
        }
        _ => ("document", "Документ", "Document", "Документ"),
    };
    objects
        .iter()
        .filter(|object| object.kind() == kind)
        .flat_map(|object| {
            sections(object, kind)
                .into_iter()
                .flatten()
                .map(move |section| {
                    // НЕ ИЗМЕРЕНО(TYPE.BUSINESS_PROCESS_SECTION_PRESENTATION)
                    // Идентичность и псевдонимы измерены; представление для бизнес-
                    // процесса пока строится по каноническому имени без учёта локали.
                    let section_display = if kind == MetadataKind::Report {
                        format!("Report Tabular section: {}", section.name())
                    } else {
                        format!("{display} табличная часть: {}", section.name())
                    };
                    Types {
                        owner: Arc::clone(&object.id),
                        section: Arc::clone(&section.id),
                        section_type: Arc::new(ValueTypeDescriptor::new(
                            "bsl-orm",
                            format!("{identity}/{key}-section/{}/{}", object.id(), section.id()),
                            section_display,
                            [
                                format!("{ru}ТабличнаяЧасть.{}.{}", object.name(), section.name()),
                                format!("{en}TabularSection.{}.{}", object.name(), section.name()),
                            ],
                        )),
                        row_type: Arc::new(ValueTypeDescriptor::new(
                            "bsl-orm",
                            format!(
                                "{identity}/{key}-section-row/{}/{}",
                                object.id(),
                                section.id()
                            ),
                            if kind == MetadataKind::Report {
                                format!("Report Tabular section Row: {}", section.name())
                            } else {
                                format!("{display} табличная часть строка: {}", section.name())
                            },
                            [
                                format!(
                                    "{ru}ТабличнаяЧастьСтрока.{}.{}",
                                    object.name(),
                                    section.name()
                                ),
                                format!(
                                    "{en}TabularSectionRow.{}.{}",
                                    object.name(),
                                    section.name()
                                ),
                            ],
                        )),
                    }
                })
        })
        .collect()
}

#[derive(Debug)]
struct SectionData {
    section_type: Arc<ValueTypeDescriptor>,
    row_type: Arc<ValueTypeDescriptor>,
    table: Value,
    attributes: Arc<[DocumentAttribute]>,
    business: bool,
    processor: bool,
    modified: Option<Rc<Cell<bool>>>,
}

#[derive(Debug, Clone)]
struct Section(Rc<SectionData>);

#[derive(Debug, Clone)]
struct Row {
    section: Rc<SectionData>,
    value: Value,
}

pub(super) fn create(
    catalog: &MetadataCatalog,
    owner: &str,
    descriptor: &DocumentTabularSection,
) -> RtResult<Value> {
    create_inner(catalog, owner, descriptor, None)
}

pub(super) fn create_tracked(
    catalog: &MetadataCatalog,
    owner: &str,
    descriptor: &DocumentTabularSection,
    modified: Rc<Cell<bool>>,
) -> RtResult<Value> {
    create_inner(catalog, owner, descriptor, Some(modified))
}

fn create_inner(
    catalog: &MetadataCatalog,
    owner: &str,
    descriptor: &DocumentTabularSection,
    modified: Option<Rc<Cell<bool>>>,
) -> RtResult<Value> {
    let attributes = descriptor.attributes().ok_or_else(|| {
        RtError::DynamicError(format!(
            "реквизиты строк табличной части «{}» неизвестны в снимке конфигурации",
            descriptor.name()
        ))
    })?;
    let business = catalog
        .materialized_object_by_id(owner)?
        .is_some_and(|object| object.kind() == MetadataKind::BusinessProcess);
    let processor = catalog
        .materialized_object_by_id(owner)?
        .is_some_and(|object| {
            matches!(
                object.kind(),
                MetadataKind::DataProcessor | MetadataKind::Report
            )
        });
    let registered = if catalog
        .materialized_object_by_id(owner)?
        .is_some_and(|object| object.kind() == MetadataKind::Report)
    {
        &catalog.report_section_types
    } else if processor {
        &catalog.data_processor_section_types
    } else if business {
        &catalog.business_section_types
    } else {
        &catalog.document_section_types
    };
    let types = registered
        .iter()
        .find(|types| types.owner.as_ref() == owner && types.section.as_ref() == descriptor.id())
        .ok_or_else(|| {
            RtError::DynamicError("тип табличной части отсутствует в каталоге конфигурации".into())
        })?;
    let table = Value::new_table();
    let Value::Object(object) = &table else {
        return Err(RtError::NotAnObject);
    };
    let BslObject::ValueTable(data) = object.as_ref() else {
        return Err(RtError::NotAnObject);
    };
    {
        let mut data = data.borrow_mut();
        data.add_constrained_column(
            "LineNumber",
            Some(vec![ColumnType::plain(TypeRef::Native(TypeId::Number))]),
        );
        for attribute in attributes {
            // Первая нативная проба подтверждает собственные Булево и Число.
            // Иные декларации не получают семантику таблицы по предположению.
            if !(business || processor)
                && !matches!(attribute.types(), [column] if matches!(column.id,
                TypeRef::Native(TypeId::Boolean | TypeId::Number)))
            {
                return Err(RtError::DynamicError(format!(
                    "тип реквизита строки «{}» пока не поддерживается",
                    attribute.name()
                )));
            }
            if data
                .add_constrained_column(attribute.name(), Some(attribute.types().to_vec()))
                .is_none()
            {
                return Err(RtError::DynamicError(format!(
                    "повторная колонка строки «{}»",
                    attribute.name()
                )));
            }
        }
    }
    Ok(Value::new_object(Section(Rc::new(SectionData {
        section_type: Arc::clone(&types.section_type),
        row_type: Arc::clone(&types.row_type),
        table,
        attributes: attributes.into(),
        business,
        processor,
        modified,
    }))))
}

impl SectionData {
    fn mark_modified(&self) {
        if let Some(flag) = &self.modified {
            flag.set(true);
        }
    }
}

impl Section {
    fn row(&self, value: Value) -> Value {
        Value::new_object(Row {
            section: Rc::clone(&self.0),
            value,
        })
    }

    fn renumber(&self) -> RtResult<()> {
        let Value::Object(object) = &self.0.table else {
            return Err(RtError::NotAnObject);
        };
        let BslObject::ValueTable(data) = object.as_ref() else {
            return Err(RtError::NotAnObject);
        };
        for (index, value) in data.borrow_mut().columns[0].iter_mut().enumerate() {
            *value = Value::Number(BslNumber::from_i64(index as i64 + 1));
        }
        Ok(())
    }
}

impl ObjectProtocol for Section {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.0.section_type))
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        if self.0.business {
            BUSINESS_METHODS
        } else {
            METHODS
        }
    }
    fn get_index(&self, index: &Value) -> RtResult<Value> {
        // Таблица индексируется по числу и не читает интернер имён структур.
        Ok(self.row(self.0.table.get_index(index, &Default::default())?))
    }
    fn collection_len(&self) -> RtResult<usize> {
        self.0.table.collection_len()
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.0) as usize, 0))
    }
}

impl ObjectProtocol for Row {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ROW_TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.section.row_type))
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        ROW_PROPERTIES
    }
    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        self.value
            .get_field_by_name(if folded_eq(name, "НомерСтроки") {
                "LineNumber"
            } else {
                name
            })
    }
    fn set_property(&self, name: &str, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
        if folded_eq(name, "НомерСтроки") || folded_eq(name, "LineNumber") {
            return Err(RtError::DynamicError(
                "номер строки табличной части доступен только для чтения".into(),
            ));
        }
        // Нативная проба присваивает значения объявленных примитивных типов.
        // Неизмеренные преобразования не выполняются неявно.
        let attribute = self
            .section
            .attributes
            .iter()
            .find(|attribute| folded_eq(attribute.name(), name))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        if self.section.business || self.section.processor {
            let value = match (&attribute.types()[0].id, value) {
                (TypeRef::Native(TypeId::Boolean), value) => {
                    Value::Boolean(open_bsl::convert_to_boolean(&value)?)
                }
                (_, value) => value,
            };
            self.value.set_field_by_name(name, value)?;
            self.section.mark_modified();
            return Ok(());
        }
        if !matches!(
            (&attribute.types()[0].id, &value),
            (TypeRef::Native(TypeId::Boolean), Value::Boolean(_))
                | (TypeRef::Native(TypeId::Number), Value::Number(_))
        ) {
            return Err(RtError::DynamicError(
                "присваивание реквизита строки из этого типа пока не поддерживается".into(),
            ));
        }
        self.value.set_field_by_name(name, value)
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        let Value::Object(object) = &self.value else {
            return None;
        };
        let BslObject::TableRow(data, id) = object.as_ref() else {
            return None;
        };
        Some((Rc::as_ptr(data) as usize, usize::try_from(*id).ok()?))
    }
}

fn section(receiver: &dyn ObjectProtocol) -> RtResult<&Section> {
    receiver
        .downcast_ref::<Section>()
        .ok_or(RtError::NotAnObject)
}

pub(super) fn copy_rows(
    source: &Value,
    target: &Value,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    let source = source
        .object_ref()
        .and_then(|value| value.downcast_ref::<Section>())
        .ok_or(RtError::NotAnObject)?;
    let target = target
        .object_ref()
        .and_then(|value| value.downcast_ref::<Section>())
        .ok_or(RtError::NotAnObject)?;
    // Новые строки имеют собственное хранилище; изменение копии не затрагивает
    // строки исходной табличной части.
    for index in 0..source.collection_len()? {
        let row = source.0.table.collection_item(
            &Value::Number(BslNumber::from_i64(index as i64)),
            &Default::default(),
        )?;
        let new_row = add(target, &[], context)?;
        for attribute in source.0.attributes.iter() {
            let value = row.get_field_by_name(attribute.name())?;
            new_row
                .object_ref()
                .ok_or(RtError::NotAnObject)?
                .set_property(attribute.name(), value, context)?;
        }
    }
    Ok(())
}

fn line_number(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    receiver
        .downcast_ref::<Row>()
        .ok_or(RtError::NotAnObject)?
        .value
        .get_field_by_name("LineNumber")
}

fn add(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    let section = section(receiver)?;
    let row = section.0.table.table_add_row()?;
    row.set_field_by_name(
        "LineNumber",
        Value::Number(BslNumber::from_i64(section.collection_len()? as i64)),
    )?;
    for attribute in section.0.attributes.iter() {
        let value = match attribute.types()[0].id {
            TypeRef::Native(TypeId::String) if section.0.business || section.0.processor => {
                Value::Str("".into())
            }
            TypeRef::Native(TypeId::Boolean) => Value::Boolean(false),
            TypeRef::Native(TypeId::Number) => Value::Number(BslNumber::from_i64(0)),
            _ => return Err(RtError::NotAnObject),
        };
        row.set_field_by_name(attribute.name(), value)?;
    }
    section.0.mark_modified();
    Ok(section.row(row))
}

fn count(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Number(BslNumber::from_i64(
        section(receiver)?.collection_len()? as i64,
    )))
}

fn unload(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    section(receiver)?.0.table.table_copy_by_filter(
        &arguments[0],
        &Value::Undefined,
        &context.runtime_shapes().names,
    )
}

fn delete(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let section = section(receiver)?;
    section.0.table.delete_element(&arguments[0])?;
    section.renumber()?;
    Ok(Value::Undefined)
}

fn clear(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    let section = section(receiver)?;
    section.0.table.clear_collection()?;
    section.0.mark_modified();
    Ok(Value::Undefined)
}

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ДокументТабличнаяЧасть",
    type_display: "Документ табличная часть",
    type_names: &["DocumentTabularSection"],
};
pub(super) static ROW_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ДокументТабличнаяЧастьСтрока",
    type_display: "Документ табличная часть строка",
    type_names: &["DocumentTabularSectionRow"],
};
pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Добавить", "Add"], Arity::exact(0), add),
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Выгрузить", "Unload"], Arity::exact(1), unload),
    MethodDescriptor::new(&["Удалить", "Delete"], Arity::exact(1), delete),
    MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), clear),
];
pub(super) static ROW_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["НомерСтроки", "LineNumber"],
    get: line_number,
    set: None,
}];

// Только операции, подтверждённые отдельным контролем бизнес-процесса.
static BUSINESS_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Добавить", "Add"], Arity::exact(0), add),
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), clear),
];

#[cfg(test)]
#[path = "document_tabular_section_update_tests.rs"]
mod update_tests;
