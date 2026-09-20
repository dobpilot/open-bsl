//! Описание объектного оповещения; доставкой управляет исполнитель.

use crate::{
    BslObject, BslValue, BuiltinMethod, CallContext, ObjectMembersDescriptor, ObjectProtocol,
    PropertyDescriptor, RtError, RtResult, TypeDescriptor, receiver_of,
};

pub(crate) static NOTIFICATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОписаниеОповещения",
    type_display: "Notification description",
    type_names: &["NotifyDescription"],
};

/// Проверенное описание обработчиков с исходными ссылками на получателей.
#[derive(Debug, Clone)]
pub struct NotificationDescription {
    // Порядок соответствует пяти аргументам непустой полной формы.
    fields: [BslValue; 5],
    invalid_module_properties: bool,
}

impl NotificationDescription {
    pub(crate) fn from_value(value: &BslValue) -> RtResult<Self> {
        value
            .object_ref()
            .and_then(|object| object.downcast_ref::<Self>())
            .cloned()
            .ok_or(RtError::TypeError {
                expected: "ОписаниеОповещения",
                op: "Файл.Begin",
            })
    }

    /// Имя успешного обработчика и его получатель; пустое описание не имеет обработчика.
    #[must_use]
    pub fn handler(&self) -> Option<(&crate::BslString, &BslValue)> {
        self.named_handler(0, 1)
    }

    /// Имя обработчика ошибки и его получатель.
    #[must_use]
    pub fn error_handler(&self) -> Option<(&crate::BslString, &BslValue)> {
        self.named_handler(3, 4)
    }

    /// Дополнительные данные без копирования изменяемого объекта.
    #[must_use]
    pub fn data(&self) -> &BslValue {
        &self.fields[2]
    }

    fn named_handler(
        &self,
        name: usize,
        receiver: usize,
    ) -> Option<(&crate::BslString, &BslValue)> {
        match &self.fields[name] {
            BslValue::Str(name) if name.len_utf16() != 0 => Some((name, &self.fields[receiver])),
            _ => None,
        }
    }
}

impl ObjectProtocol for NotificationDescription {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &NOTIFICATION_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
}

fn validate_handler(
    context: &CallContext<'_>,
    name: &BslValue,
    receiver: &BslValue,
) -> RtResult<BslValue> {
    // НЕ ИЗМЕРЕНО(NOTIFY.EDGE_ARGUMENTS): непрозрачные/служебные
    // получатели и точные тексты ошибок вне проб.
    let name = context.format_value(name, None)?;
    if name.is_empty() {
        return Err(RtError::TypeError {
            expected: "непустое имя обработчика",
            op: "ОписаниеОповещения",
        });
    }
    let BslValue::Object(object) = receiver else {
        return Err(RtError::TypeError {
            expected: "объект-получатель оповещения",
            op: "ОписаниеОповещения",
        });
    };
    let exists = match &**object {
        BslObject::Uuid(_) | BslObject::VstrOpaque(_) => {
            return Err(RtError::TypeError {
                expected: "объект-получатель оповещения",
                op: "ОписаниеОповещения",
            });
        }
        BslObject::Extension(object) => {
            if crate::value_table_indexes::is_index(object) {
                return Err(RtError::TypeError {
                    expected: "объект-получатель оповещения",
                    op: "ОписаниеОповещения",
                });
            }
            object.has_method(&name)
        }
        object => BuiltinMethod::lookup(&name).is_some_and(|method| method.declared_on(object)),
    };
    if !exists {
        return Err(RtError::UnknownMethod {
            method: name,
            receiver: receiver.type_name(),
        });
    }
    Ok(BslValue::Str(name.as_str().into()))
}

pub(crate) fn construct_notification(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let mut fields = [
        BslValue::Str("".into()),
        BslValue::Undefined,
        BslValue::Undefined,
        BslValue::Str("".into()),
        BslValue::Undefined,
    ];
    let mut invalid_module_properties = false;
    match arguments {
        [] => {}
        [name, receiver] | [name, receiver, _] | [name, receiver, _, _, _] => {
            fields[0] = validate_handler(context, name, receiver)?;
            fields[1] = receiver.clone();
            if let Some(data) = arguments.get(2) {
                fields[2] = data.clone();
            }
            if let [_, _, _, error_name, error_receiver] = arguments {
                if matches!(
                    error_name,
                    BslValue::Enum(value) if value.kind() == crate::EnumKind::TextEncoding
                ) {
                    fields[3] = validate_handler(context, &fields[0], error_receiver)?;
                    invalid_module_properties = true;
                } else {
                    fields[3] = validate_handler(context, error_name, error_receiver)?;
                }
                fields[4] = error_receiver.clone();
            }
        }
        _ => {
            return Err(RtError::InvalidBytecode(
                "неверная арность конструктора ОписаниеОповещения",
            ));
        }
    }
    Ok(BslValue::new_object(NotificationDescription {
        fields,
        invalid_module_properties,
    }))
}

fn field(receiver: &dyn ObjectProtocol, index: usize) -> RtResult<BslValue> {
    let description = receiver_of::<NotificationDescription>(receiver, "ОписаниеОповещения")?;
    if description.invalid_module_properties && matches!(index, 1 | 4) {
        return Err(RtError::TypeError {
            expected: "объект-получатель оповещения",
            op: "ОписаниеОповещения",
        });
    }
    Ok(description.fields[index].clone())
}

static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ИмяПроцедуры", "ProcedureName"],
        get: |receiver, _| field(receiver, 0),
        set: None,
    },
    PropertyDescriptor {
        names: &["Модуль", "Module"],
        get: |receiver, _| field(receiver, 1),
        set: None,
    },
    PropertyDescriptor {
        names: &["ДополнительныеПараметры", "AdditionalParameters"],
        get: |receiver, _| field(receiver, 2),
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяПроцедурыОбработкиОшибки", "ErrorHandlerProcedureName"],
        get: |receiver, _| field(receiver, 3),
        set: None,
    },
    PropertyDescriptor {
        names: &["МодульОбработкиОшибки", "ErrorHandlerModule"],
        get: |receiver, _| field(receiver, 4),
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&NOTIFICATION_TYPE).with_properties(PROPERTIES)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arity, MethodDescriptor, RuntimeShapes};

    #[derive(Debug)]
    struct HostReceiver;

    impl ObjectProtocol for HostReceiver {
        fn type_descriptor(&self) -> &'static TypeDescriptor {
            static TYPE: TypeDescriptor = TypeDescriptor::new("test", "Получатель");
            &TYPE
        }

        fn has_method(&self, name: &str) -> bool {
            crate::folded_eq(name, "Динамический")
                || crate::find_method_from_table(self.method_table(), name).is_some()
        }

        fn method_table(&self) -> &'static [MethodDescriptor] {
            static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
                &["Статический", "Static"],
                Arity::exact(9),
                |_, _, _| panic!("конструктор не исполняет статический метод"),
            )];
            METHODS
        }

        fn call_method(
            &self,
            _: &str,
            _: &[BslValue],
            _: &mut CallContext<'_>,
        ) -> RtResult<BslValue> {
            panic!("конструктор не исполняет динамический метод")
        }
    }

    fn formatter(value: &BslValue, _: Option<&str>) -> RtResult<String> {
        let BslValue::Str(value) = value else {
            panic!("в этом тесте форматируются только имена")
        };
        Ok(value.to_string())
    }

    #[test]
    fn constructor_preserves_host_receivers_without_calling_or_checking_arity() {
        let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut context = CallContext::minimal(&mut shapes, formatter);
        let receiver = BslValue::new_object(HostReceiver);
        let data = BslValue::new_array(vec![BslValue::number_from_i64(7)]);
        let arguments = [
            BslValue::Str("ДИНАМИЧЕСКИЙ".into()),
            receiver.clone(),
            data.clone(),
            BslValue::Str("static".into()),
            receiver.clone(),
        ];
        let description = construct_notification(&mut context, &arguments).unwrap();
        let object = description.object_ref().unwrap();
        assert_eq!(
            object.get_property("Module", &mut context).unwrap(),
            receiver
        );
        assert_eq!(
            object
                .get_property("ErrorHandlerModule", &mut context)
                .unwrap(),
            receiver
        );
        assert_eq!(
            object
                .get_property("AdditionalParameters", &mut context)
                .unwrap(),
            data
        );
        assert_eq!(
            object.get_property("ProcedureName", &mut context).unwrap(),
            arguments[0]
        );
        for property in PROPERTIES {
            for name in property.names {
                assert!(matches!(
                    object.set_property(name, BslValue::Undefined, &mut context),
                    Err(RtError::PropertyReadOnly { .. })
                ));
            }
        }
        for count in [1, 4, 6] {
            assert!(
                construct_notification(&mut context, &vec![BslValue::Undefined; count]).is_err()
            );
        }
        for name in ["НетМетода", " Static", "Динамический "] {
            for error_role in [false, true] {
                let mut invalid = arguments.clone();
                invalid[if error_role { 3 } else { 0 }] = BslValue::Str(name.into());
                assert!(matches!(
                    construct_notification(&mut context, &invalid),
                    Err(RtError::UnknownMethod { .. })
                ));
            }
        }
    }

    #[test]
    fn text_encoding_error_name_reuses_the_success_name_and_hides_modules() {
        let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut context = CallContext::minimal(&mut shapes, formatter);
        let receiver = BslValue::new_object(HostReceiver);
        let arguments = [
            BslValue::Str("Динамический".into()),
            receiver.clone(),
            BslValue::Undefined,
            BslValue::Enum(crate::EnumValue::TextEncodingUtf8),
            receiver.clone(),
        ];
        let description = construct_notification(&mut context, &arguments).unwrap();
        let object = description.object_ref().unwrap();
        assert_eq!(
            object
                .get_property("ErrorHandlerProcedureName", &mut context)
                .unwrap(),
            arguments[0]
        );
        for property in ["Module", "ErrorHandlerModule"] {
            assert!(matches!(
                object.get_property(property, &mut context),
                Err(RtError::TypeError { .. })
            ));
        }
        let stored = object.downcast_ref::<NotificationDescription>().unwrap();
        assert_eq!(stored.handler().unwrap().0.to_string(), "Динамический");
        assert_eq!(
            stored.error_handler().unwrap().0.to_string(),
            "Динамический"
        );

        let wrong_receiver = BslValue::new_array(Vec::new());
        let mut invalid = arguments.clone();
        invalid[4] = wrong_receiver;
        assert!(matches!(
            construct_notification(&mut context, &invalid),
            Err(RtError::UnknownMethod { .. })
        ));
    }

    #[test]
    fn base_method_declarations_match_native_receiver_matrices() {
        use std::{cell::RefCell, collections::HashMap, rc::Rc};
        let core = include_str!(
            "../../../tests/conformance/measure/filesystem/notification-core-methods-client.platform.txt"
        );
        let extra = include_str!(
            "../../../tests/conformance/measure/filesystem/notification-extra-receivers.platform.txt"
        );
        let table = crate::table::ValueTableData::new();
        let receivers = [
            ("array", BslObject::Array(RefCell::new(vec![]))),
            (
                "structure",
                BslObject::Structure(RefCell::new(crate::StructureStorage::Dictionary {
                    order: vec![],
                    values: HashMap::new(),
                })),
            ),
            (
                "map",
                BslObject::Map(RefCell::new(crate::map::MapData::new())),
            ),
            ("table", BslObject::ValueTable(table.clone())),
            ("table.columns", BslObject::TableColumns(table.clone())),
            (
                "table.column",
                BslObject::TableColumn(table.clone(), "НетКолонки".into()),
            ),
            ("table.row", BslObject::TableRow(table, 0)),
            ("types", BslObject::TypeDescription(vec![])),
            ("binary", BslObject::BinaryData(Rc::from([]))),
            (
                "buffer",
                BslObject::BinaryBuffer(RefCell::new(crate::bindata::BinBufData::new(
                    vec![],
                    crate::bindata::ByteOrder::Little,
                ))),
            ),
            ("comparison", BslObject::ValueComparison),
            (
                "pair",
                BslObject::KeyValuePair(BslValue::Undefined, BslValue::Undefined),
            ),
            ("writer", BslObject::TextWriter(RefCell::new(None))),
        ];
        for raw in [core, extra] {
            let names = raw
                .lines()
                .find_map(|line| line.strip_prefix("methods\t"))
                .unwrap();
            assert_eq!(
                names.split(',').collect::<Vec<_>>(),
                crate::BUILTIN_METHOD_NAMES
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
            );
        }
        for (id, receiver) in receivers {
            let prefix = format!("{id}\t");
            let mask = core
                .lines()
                .chain(extra.lines())
                .find_map(|line| line.strip_prefix(&prefix))
                .unwrap()
                .split_once('|')
                .unwrap()
                .1;
            assert_eq!(mask.len(), crate::BUILTIN_METHOD_NAMES.len());
            for ((name, method), bit) in crate::BUILTIN_METHOD_NAMES.iter().zip(mask.bytes()) {
                assert!(matches!(bit, b'0' | b'1'));
                assert_eq!(method.declared_on(&receiver), bit == b'1', "{id}.{name}");
            }
        }
    }
}
