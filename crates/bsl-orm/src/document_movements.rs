//! Коллекция движений нового документа и принадлежащие ему наборы записей.

use super::*;

#[derive(Debug)]
struct CollectionData {
    names: Vec<String>,
    items: Value,
}

#[derive(Debug, Clone)]
struct Collection(Rc<CollectionData>);

#[derive(Debug)]
struct RecordSetData {
    ty: Arc<ValueTypeDescriptor>,
    records: RefCell<Vec<Value>>,
    write: Cell<bool>,
}

#[derive(Debug, Clone)]
struct RecordSet(Rc<RecordSetData>);

pub(super) fn create(catalog: &MetadataCatalog, registers: &[String]) -> RtResult<Value> {
    let mut names = Vec::with_capacity(registers.len());
    let mut items = Vec::with_capacity(registers.len());
    for (index, id) in registers.iter().enumerate() {
        if registers[..index].contains(id) {
            return Err(RtError::DynamicError(format!(
                "повторный регистр движений «{id}»"
            )));
        }
        let register = catalog.materialized_object_by_id(id)?.ok_or_else(|| {
            RtError::DynamicError(format!(
                "регистр движений «{id}» отсутствует в каталоге конфигурации"
            ))
        })?;
        if register.kind() != MetadataKind::InformationRegister {
            return Err(RtError::DynamicError(format!(
                "класс регистра движений «{}» пока не поддерживается",
                register.name()
            )));
        }
        let ty = catalog
            .information_record_set_types
            .iter()
            .find(|(owner, _)| owner.as_ref() == id)
            .map(|(_, ty)| Arc::clone(ty))
            .ok_or_else(|| {
                RtError::DynamicError(
                    "тип набора записей отсутствует в каталоге конфигурации".into(),
                )
            })?;
        names.push(register.name().to_owned());
        // Новому документу принадлежат новые пустые наборы. Создание не
        // является чтением регистра и не затрагивает прикладные строки базы.
        items.push(Value::new_object(RecordSet(Rc::new(RecordSetData {
            ty,
            records: RefCell::new(vec![]),
            write: Cell::new(false),
        }))));
    }
    Ok(Value::new_object(Collection(Rc::new(CollectionData {
        names,
        items: Value::new_array(items),
    }))))
}

impl ObjectProtocol for Collection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let index = self
            .0
            .names
            .iter()
            .position(|candidate| folded_eq(candidate, name))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        self.get_index(&Value::Number(BslNumber::from_i64(index as i64)))
    }
    fn get_index(&self, index: &Value) -> RtResult<Value> {
        // Массив здесь индексируется числом и не читает имена структуры.
        self.0.items.get_index(index, &Default::default())
    }
    fn collection_len(&self) -> RtResult<usize> {
        self.0.items.collection_len()
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.0) as usize, 0))
    }
}

impl ObjectProtocol for RecordSet {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &RECORD_SET_TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.0.ty))
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        RECORD_SET_METHODS
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        RECORD_SET_PROPERTIES
    }
    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.0.records.borrow().len())
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.0) as usize, 0))
    }
}

fn count(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Number(BslNumber::from_i64(
        receiver.collection_len()? as i64,
    )))
}

fn record_set(receiver: &dyn ObjectProtocol) -> RtResult<&RecordSet> {
    receiver
        .downcast_ref::<RecordSet>()
        .ok_or(RtError::NotAnObject)
}

fn write_flag(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(record_set(receiver)?.0.write.get()))
}

fn set_write_flag(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let Value::Boolean(value) = value else {
        return Err(RtError::DynamicError(
            "присваивание Записывать из этого типа пока не поддерживается".into(),
        ));
    };
    record_set(receiver)?.0.write.set(value);
    Ok(())
}

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "КоллекцияДвижений",
    type_display: "Коллекция движений",
    type_names: &["RegisterRecordsCollection"],
};
pub(super) static RECORD_SET_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "РегистрСведенийНаборЗаписей",
    type_display: "Регистр сведений набор записей",
    type_names: &["InformationRegisterRecordSet"],
};
pub(super) static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Количество", "Count"],
    Arity::exact(0),
    count,
)];
pub(super) static RECORD_SET_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Количество", "Count"],
    Arity::exact(0),
    count,
)];
pub(super) static RECORD_SET_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Записывать", "Write"],
    get: write_flag,
    set: Some(set_write_flag),
}];
