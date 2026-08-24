//! Объектный протокол окон в документ: тождество, свойства, методы.

use super::*;

// --- объектный протокол -----------------------------------------------------

/// Адрес состояния как ключ тождества: обёртки строятся на каждое
/// обращение, а равенство у окон в документ — «то же состояние, то же
/// место» (измерено, см. ключи ниже).
pub(crate) fn state_addr<T>(state: &Rc<RefCell<T>>) -> usize {
    Rc::as_ptr(state) as usize
}

/// Получатель нужного типа: чужой получает ту же ошибку «метод не
/// применим», что и прежний строковый путь.
macro_rules! receiver_of {
    ($name:ident, $ty:ty, $type_name:expr) => {
        fn $name<'r>(receiver: &'r dyn ObjectProtocol, method: &'static str) -> RtResult<&'r $ty> {
            receiver
                .downcast_ref::<$ty>()
                .ok_or(RtError::MethodNotApplicable {
                    method,
                    receiver: $type_name,
                })
        }
    };
}

receiver_of!(document_of, DocumentObject, DOCUMENT_TYPE.name);
receiver_of!(pages_of, PagesObject, PAGES_TYPE.name);
receiver_of!(page_of, PageObject, PAGE_TYPE.name);
receiver_of!(attachments_of, AttachmentsObject, ATTACHMENTS_TYPE.name);
receiver_of!(attachment_of, AttachmentObject, ATTACHMENT_TYPE.name);

// --- «ДокументPDF» -----------------------------------------------------------

fn document_attachments(
    receiver: &dyn ObjectProtocol,
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    document_property(document_of(receiver, "Вложения")?, "Вложения")
}

fn document_pages(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    document_property(document_of(receiver, "Страницы")?, "Страницы")
}

static DOCUMENT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Вложения", "Attachments"],
        get: document_attachments,
        set: None,
    },
    PropertyDescriptor {
        names: &["Страницы", "Pages"],
        get: document_pages,
        set: None,
    },
];

fn document_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    read(document_of(receiver, "Прочитать")?, arguments)?;
    Ok(BslValue::Undefined)
}

fn document_write(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write(document_of(receiver, "Записать")?, arguments)?;
    Ok(BslValue::Undefined)
}

static DOCUMENT_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Прочитать", "Read"], Arity::range(1, 2), document_read),
    MethodDescriptor::new(&["Записать", "Write"], Arity::range(1, 2), document_write),
];

impl ObjectProtocol for DocumentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOCUMENT_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        DOCUMENT_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DOCUMENT_METHODS
    }

    // `ЗначениеЗаполнено(Новый ДокументPDF)` — измеренная ошибка «Проверка
    // мутабельных значений на заполненность не поддерживается»; её отдаёт
    // реализация протокола по умолчанию.
}

// --- коллекция страниц --------------------------------------------------------

fn pages_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    page_count(pages_of(receiver, "Количество")?).map(|len| BslValue::number_from_i64(len as i64))
}

fn pages_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let pages = pages_of(receiver, "Получить")?;
    match arguments {
        [index] => page_get(pages, index),
        _ => Err(RtError::MethodNotApplicable {
            method: "Получить",
            receiver: PAGES_TYPE.name,
        }),
    }
}

fn pages_index_of(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let pages = pages_of(receiver, "Индекс")?;
    match arguments {
        [page] => page_index_of(pages, page),
        _ => Err(RtError::MethodNotApplicable {
            method: "Индекс",
            receiver: PAGES_TYPE.name,
        }),
    }
}

static PAGES_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), pages_count),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), pages_get),
    MethodDescriptor::new(&["Индекс", "IndexOf"], Arity::exact(1), pages_index_of),
];

impl ObjectProtocol for PagesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PAGES_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        PAGES_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        page_at(self, page_index_arg(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        page_count(self)
    }

    // Коллекция страниц судится ПО ДЛИНЕ (измерено): три страницы дали
    // «Да», пустое дерево — «Нет».
    fn is_filled(&self) -> RtResult<bool> {
        Ok(page_count(self)? > 0)
    }

    // Два отдельных чтения `Док.Страницы` равны (измерено) — окно в тот же
    // документ.
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.state), 0))
    }
}

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn page_index_arg(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}

// --- страница -----------------------------------------------------------------

fn page_number(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    page_property(page_of(receiver, "Номер")?, "Номер")
}

fn page_width(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    page_property(page_of(receiver, "Ширина")?, "Ширина")
}

fn page_height(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    page_property(page_of(receiver, "Высота")?, "Высота")
}

fn page_orientation(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    page_property(page_of(receiver, "Ориентация")?, "Ориентация")
}

// Поля печати приходят из `/TrimBox`; правило — в `margins_of`.
fn page_left_margin(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    page_property(page_of(receiver, "ПолеСлева")?, "ПолеСлева")
}

fn page_right_margin(
    receiver: &dyn ObjectProtocol,
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    page_property(page_of(receiver, "ПолеСправа")?, "ПолеСправа")
}

fn page_top_margin(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
    page_property(page_of(receiver, "ПолеСверху")?, "ПолеСверху")
}

fn page_bottom_margin(
    receiver: &dyn ObjectProtocol,
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    page_property(page_of(receiver, "ПолеСнизу")?, "ПолеСнизу")
}

static PAGE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Номер", "Number"],
        get: page_number,
        set: None,
    },
    PropertyDescriptor {
        names: &["Ширина", "Width"],
        get: page_width,
        set: None,
    },
    PropertyDescriptor {
        names: &["Высота", "Height"],
        get: page_height,
        set: None,
    },
    PropertyDescriptor {
        names: &["Ориентация", "Orientation"],
        get: page_orientation,
        set: None,
    },
    PropertyDescriptor {
        names: &["ПолеСлева", "LeftMargin"],
        get: page_left_margin,
        set: None,
    },
    PropertyDescriptor {
        names: &["ПолеСправа", "RightMargin"],
        get: page_right_margin,
        set: None,
    },
    PropertyDescriptor {
        names: &["ПолеСверху", "TopMargin"],
        get: page_top_margin,
        set: None,
    },
    PropertyDescriptor {
        names: &["ПолеСнизу", "BottomMargin"],
        get: page_bottom_margin,
        set: None,
    },
];

impl ObjectProtocol for PageObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PAGE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PAGE_PROPERTIES
    }

    // Страница — ССЫЛКА на место в документе: `Страницы[0] = Страницы[0]`
    // — «Да», `Страницы[0] = Страницы[1]` — «Нет» (измерено).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.state), self.index))
    }
}

// --- коллекция вложений --------------------------------------------------------

fn attachments_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attachment_count(attachments_of(receiver, "Количество")?)
        .map(|len| BslValue::number_from_i64(len as i64))
}

fn attachments_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = attachments_of(receiver, "Получить")?;
    match arguments {
        [index] => attachment_get(items, index),
        _ => Err(RtError::MethodNotApplicable {
            method: "Получить",
            receiver: ATTACHMENTS_TYPE.name,
        }),
    }
}

fn attachments_find(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attachment_find(attachments_of(receiver, "Найти")?, arguments)
}

fn attachments_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attachment_add(attachments_of(receiver, "Добавить")?, arguments)?;
    Ok(BslValue::Undefined)
}

fn attachments_delete(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    attachment_delete(attachments_of(receiver, "Удалить")?, arguments)?;
    Ok(BslValue::Undefined)
}

fn attachments_clear(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = attachments_of(receiver, "Очистить")?;
    if !arguments.is_empty() {
        return Err(RtError::MethodNotApplicable {
            method: "Очистить",
            receiver: ATTACHMENTS_TYPE.name,
        });
    }
    attachment_clear(items)?;
    Ok(BslValue::Undefined)
}

fn attachments_index_of(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let items = attachments_of(receiver, "Индекс")?;
    match arguments {
        [item] => attachment_index_of(items, item),
        _ => Err(RtError::MethodNotApplicable {
            method: "Индекс",
            receiver: ATTACHMENTS_TYPE.name,
        }),
    }
}

static ATTACHMENTS_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), attachments_count),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), attachments_get),
    MethodDescriptor::new(&["Найти", "Find"], Arity::exact(1), attachments_find),
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(2, 4), attachments_add),
    MethodDescriptor::new(&["Удалить", "Delete"], Arity::exact(1), attachments_delete),
    MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), attachments_clear),
    MethodDescriptor::new(
        &["Индекс", "IndexOf"],
        Arity::exact(1),
        attachments_index_of,
    ),
];

impl ObjectProtocol for AttachmentsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ATTACHMENTS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ATTACHMENTS_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        attachment_at(self, page_index_arg(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        attachment_count(self)
    }

    // Коллекция вложений — тоже по длине: измерено, что у документа с
    // пятью вложениями `ЗначениеЗаполнено` даёт «Да».
    fn is_filled(&self) -> RtResult<bool> {
        Ok(attachment_count(self)? > 0)
    }

    // Коллекция и её элементы держат тот же `Rc`, поэтому `Док.Вложения =
    // Док.Вложения` — «Да» (измерено).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.items), 0))
    }
}

// --- вложение -------------------------------------------------------------------

macro_rules! attachment_property_pair {
    ($get:ident, $set:ident, $canonical:expr) => {
        fn $get(receiver: &dyn ObjectProtocol, _c: &mut CallContext<'_>) -> RtResult<BslValue> {
            attachment_property(attachment_of(receiver, $canonical)?, $canonical)
        }

        fn $set(
            receiver: &dyn ObjectProtocol,
            value: BslValue,
            _c: &mut CallContext<'_>,
        ) -> RtResult<()> {
            set_attachment_property(attachment_of(receiver, $canonical)?, $canonical, &value)
        }
    };
}

attachment_property_pair!(
    attachment_get_file_name,
    attachment_set_file_name,
    "ИмяФайла"
);
attachment_property_pair!(attachment_get_mime, attachment_set_mime, "ТипСодержимого");
attachment_property_pair!(attachment_get_content, attachment_set_content, "Содержимое");
attachment_property_pair!(attachment_get_relation, attachment_set_relation, "ТипСвязи");

static ATTACHMENT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ИмяФайла", "FileName"],
        get: attachment_get_file_name,
        set: Some(attachment_set_file_name),
    },
    PropertyDescriptor {
        names: &["ТипСодержимого", "MIMEType"],
        get: attachment_get_mime,
        set: Some(attachment_set_mime),
    },
    PropertyDescriptor {
        names: &["Содержимое", "Content"],
        get: attachment_get_content,
        set: Some(attachment_set_content),
    },
    PropertyDescriptor {
        names: &["ТипСвязи", "RelationshipType"],
        get: attachment_get_relation,
        set: Some(attachment_set_relation),
    },
];

impl ObjectProtocol for AttachmentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ATTACHMENT_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        ATTACHMENT_PROPERTIES
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.items), self.index))
    }
}
