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
    let text = message.text.borrow().clone();
    // Тот же канал, что у глобального «Сообщить»: в сеансе фонового
    // задания stdout перехвачен в FIFO-историю записи реестра.
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
