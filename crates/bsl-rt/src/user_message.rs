//! `СообщениеПользователю` — по выписке синтакс-помощника 8.3.27:
//! конструктор по умолчанию, записываемые свойства
//! `ИдентификаторНазначения`, `КлючДанных`, `Поле`, `ПутьКДанным`,
//! `Текст`; методы `Сообщить()` и `УстановитьДанные()`. Формной механики
//! показа у open-bsl нет: `Сообщить()` выводит текст тем же путём, что
//! глобальный `Сообщить`, а в сеансе фонового задания попадает в его
//! FIFO-историю (stdout сеанса перехвачен).

use std::cell::RefCell;

use crate::{
    Arity, BslValue, CallContext, MethodDescriptor, ObjectProtocol, PropertyDescriptor, RtResult,
    TypeDescriptor, receiver_of,
};

pub(crate) static USER_MESSAGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СообщениеПользователю",
    type_display: "User message",
    type_names: &["UserMessage"],
};

/// Параметры сообщения; до показа объект изменяем.
pub(crate) struct UserMessageObject {
    pub text: RefCell<String>,
    pub field: RefCell<String>,
    pub data_key: RefCell<BslValue>,
    pub data_path: RefCell<String>,
    pub target_id: RefCell<BslValue>,
}

impl UserMessageObject {
    pub(crate) fn with_text(text: &str) -> Self {
        Self {
            text: RefCell::new(text.to_string()),
            field: RefCell::new(String::new()),
            data_key: RefCell::new(BslValue::Undefined),
            data_path: RefCell::new(String::new()),
            target_id: RefCell::new(BslValue::Undefined),
        }
    }

    /// Владеющий `Send`-снимок всех полей: значения `КлючДанных` и
    /// `ИдентификаторНазначения` сериализуются графами. `budget` — остаток
    /// бюджета сообщений принимающего sink: сериализация ограничивается
    /// им С УЧЁТОМ длин текстовых полей и уже снятых графов, поэтому
    /// сообщение больше остатка не собирается в память вовсе.
    ///
    /// # Errors
    ///
    /// Ловимая ошибка с кодом `ResourceLimit`, когда сообщение не
    /// помещается в `budget`; ошибка сериализации значения, не
    /// переносимого между сеансами.
    pub(crate) fn to_dto(
        &self,
        ctx: &mut CallContext<'_>,
        budget: Option<usize>,
    ) -> RtResult<crate::UserMessageDto> {
        let over_budget = || {
            crate::HostError::new(
                crate::HostErrorCode::ResourceLimit,
                "исчерпан бюджет сообщений задания",
            )
            .raise()
        };
        let text_bytes =
            self.text.borrow().len() + self.field.borrow().len() + self.data_path.borrow().len();
        let mut left = match budget {
            None => None,
            Some(budget) => Some(budget.checked_sub(text_bytes).ok_or_else(over_budget)?),
        };
        let mut capture = |value: &BslValue,
                           ctx: &mut CallContext<'_>|
         -> RtResult<Option<crate::SerializedValueGraph>> {
            if matches!(value, BslValue::Undefined) {
                return Ok(None);
            }
            let limits = crate::GraphLimits {
                max_bytes: left.unwrap_or(crate::GraphLimits::default().max_bytes),
            };
            let graph = crate::SerializedValueGraph::capture(
                std::slice::from_ref(value),
                ctx.runtime_shapes(),
                &limits,
            )
            .map_err(|error| match error {
                crate::RtError::ResourceLimit(_) if left.is_some() => over_budget(),
                other => other,
            })?;
            if let Some(left) = &mut left {
                *left -= graph.byte_size();
            }
            Ok(Some(graph))
        };
        let data_key = capture(&self.data_key.borrow().clone(), ctx)?;
        let target_id = capture(&self.target_id.borrow().clone(), ctx)?;
        Ok(crate::UserMessageDto {
            text: self.text.borrow().clone(),
            field: self.field.borrow().clone(),
            data_path: self.data_path.borrow().clone(),
            data_key,
            target_id,
        })
    }

    /// Объект из DTO — путь чтения истории задания: графы
    /// материализуются в формы читающего сеанса.
    ///
    /// # Errors
    ///
    /// Ошибка материализации сериализованного значения.
    pub(crate) fn from_dto(
        dto: &crate::UserMessageDto,
        ctx: &mut CallContext<'_>,
    ) -> RtResult<Self> {
        let materialize = |graph: &Option<crate::SerializedValueGraph>,
                           ctx: &mut CallContext<'_>|
         -> RtResult<BslValue> {
            match graph {
                None => Ok(BslValue::Undefined),
                Some(graph) => {
                    let mut values = graph.materialize(ctx.runtime_shapes())?;
                    Ok(values.pop().unwrap_or(BslValue::Undefined))
                }
            }
        };
        let data_key = materialize(&dto.data_key, ctx)?;
        let target_id = materialize(&dto.target_id, ctx)?;
        Ok(Self {
            text: RefCell::new(dto.text.clone()),
            field: RefCell::new(dto.field.clone()),
            data_key: RefCell::new(data_key),
            data_path: RefCell::new(dto.data_path.clone()),
            target_id: RefCell::new(target_id),
        })
    }
}

impl std::fmt::Debug for UserMessageObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "СообщениеПользователю {:?}", self.text.borrow())
    }
}

impl ObjectProtocol for UserMessageObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &USER_MESSAGE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        USER_MESSAGE_PROPERTIES
    }

    fn get_property(&self, name: &str, ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(
            USER_MESSAGE_PROPERTIES,
            "СообщениеПользователю",
            self,
            name,
            ctx,
        )
    }

    fn set_property(&self, name: &str, value: BslValue, ctx: &mut CallContext<'_>) -> RtResult<()> {
        crate::set_property_from_table(
            USER_MESSAGE_PROPERTIES,
            "СообщениеПользователю",
            self,
            name,
            value,
            ctx,
        )
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        USER_MESSAGE_METHODS
    }
}

macro_rules! string_property {
    ($get:ident, $set:ident, $field:ident, $name:literal) => {
        fn $get(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
            let message = receiver_of::<UserMessageObject>(receiver, $name)?;
            Ok(BslValue::Str(crate::BslString::from_str(
                &message.$field.borrow(),
            )))
        }
        fn $set(
            receiver: &dyn ObjectProtocol,
            value: BslValue,
            _ctx: &mut CallContext<'_>,
        ) -> RtResult<()> {
            let message = receiver_of::<UserMessageObject>(receiver, $name)?;
            *message.$field.borrow_mut() = value.as_str($name)?.to_string();
            Ok(())
        }
    };
}

macro_rules! value_property {
    ($get:ident, $set:ident, $field:ident, $name:literal) => {
        fn $get(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
            let message = receiver_of::<UserMessageObject>(receiver, $name)?;
            Ok(message.$field.borrow().clone())
        }
        fn $set(
            receiver: &dyn ObjectProtocol,
            value: BslValue,
            _ctx: &mut CallContext<'_>,
        ) -> RtResult<()> {
            let message = receiver_of::<UserMessageObject>(receiver, $name)?;
            *message.$field.borrow_mut() = value;
            Ok(())
        }
    };
}

string_property!(get_text, set_text, text, "Текст");
string_property!(get_field, set_field, field, "Поле");
string_property!(get_data_path, set_data_path, data_path, "ПутьКДанным");
value_property!(get_data_key, set_data_key, data_key, "КлючДанных");
value_property!(
    get_target_id,
    set_target_id,
    target_id,
    "ИдентификаторНазначения"
);

static USER_MESSAGE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Текст", "Text"],
        get: get_text,
        set: Some(set_text),
    },
    PropertyDescriptor {
        names: &["Поле", "Field"],
        get: get_field,
        set: Some(set_field),
    },
    PropertyDescriptor {
        names: &["КлючДанных", "DataKey"],
        get: get_data_key,
        set: Some(set_data_key),
    },
    PropertyDescriptor {
        names: &["ПутьКДанным", "DataPath"],
        get: get_data_path,
        set: Some(set_data_path),
    },
    PropertyDescriptor {
        names: &["ИдентификаторНазначения", "TargetID"],
        get: get_target_id,
        set: Some(set_target_id),
    },
];

fn message_send(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let message = receiver_of::<UserMessageObject>(receiver, "Сообщить")?;
    // Тот же механизм, что у глобального «Сообщить»: сеанс с внедрённым
    // sink получает владеющий DTO со ВСЕМИ полями (в сеансе фонового
    // задания он попадает в FIFO-историю записи реестра); без sink
    // текст уходит в stdout сеанса — прежний путь.
    if let Some(sink) = ctx.message_sink().map(std::rc::Rc::clone) {
        let dto = message.to_dto(ctx, sink.message_bytes_left())?;
        sink.enqueue(&dto).map_err(crate::HostError::raise)?;
        return Ok(BslValue::Undefined);
    }
    let text = message.text.borrow().clone();
    let stdout = ctx.stdout()?;
    writeln!(stdout, "{text}").map_err(|error| crate::RtError::IoError(error.to_string()))?;
    Ok(BslValue::Undefined)
}

fn message_set_data(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let message = receiver_of::<UserMessageObject>(receiver, "УстановитьДанные")?;
    // Формной адресации у open-bsl нет: значение сохраняется как ключ
    // данных, чем и ограничивается наблюдаемый эффект.
    *message.data_key.borrow_mut() = args.first().cloned().unwrap_or(BslValue::Undefined);
    Ok(BslValue::Undefined)
}

static USER_MESSAGE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Сообщить", "Message"], Arity::exact(0), message_send),
    MethodDescriptor::new(
        &["УстановитьДанные", "SetData"],
        Arity::exact(1),
        message_set_data,
    ),
];

/// Глобальная `ПолучитьСообщенияПользователю(УдалятьПолученные = Ложь)`
/// — по выписке синтакс-помощника возвращает `ФиксированныйМассив`
/// «ещё не выведенных» сообщений и НЕ пересекается с историей сообщений
/// фонового задания. open-bsl выводит сообщения немедленно (stdout
/// сеанса), поэтому накопленных нет — возвращается пустой массив;
/// накопление внутри сеанса задания — остаток `JOB.MESSAGES`.
pub(crate) fn get_user_messages(
    _ctx: &mut CallContext<'_>,
    _args: &[BslValue],
) -> RtResult<BslValue> {
    Ok(BslValue::new_object(
        crate::fixed_array::FixedArrayObject::new(Vec::new()),
    ))
}

/// `Новый СообщениеПользователю` — конструктор по умолчанию.
pub(crate) fn construct_user_message(
    _ctx: &mut CallContext<'_>,
    _args: &[BslValue],
) -> RtResult<BslValue> {
    Ok(BslValue::new_object(UserMessageObject::with_text("")))
}
