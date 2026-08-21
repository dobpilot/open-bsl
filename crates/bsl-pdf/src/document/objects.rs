//! Объектный протокол окон в документ: тождество, свойства, методы.

use super::*;

// --- объектный протокол -----------------------------------------------------

/// Адрес состояния как ключ тождества: обёртки строятся на каждое
/// обращение, а равенство у окон в документ — «то же состояние, то же
/// место» (измерено, см. ключи ниже).
pub(crate) fn state_addr<T>(state: &Rc<RefCell<T>>) -> usize {
    Rc::as_ptr(state) as usize
}

impl ObjectProtocol for DocumentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOCUMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        document_property(self, name)
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Прочитать") || name.eq_ignore_ascii_case("Read") {
            read(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Записать") || name.eq_ignore_ascii_case("Write")
        {
            write(self, arguments)?;
            Ok(BslValue::Undefined)
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: DOCUMENT_TYPE.name,
            })
        }
    }

    // `ЗначениеЗаполнено(Новый ДокументPDF)` — измеренная ошибка «Проверка
    // мутабельных значений на заполненность не поддерживается»; её отдаёт
    // реализация протокола по умолчанию.
}

impl ObjectProtocol for PagesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PAGES_TYPE
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Количество") || name.eq_ignore_ascii_case("Count") {
            page_count(self).map(|len| BslValue::number_from_i64(len as i64))
        } else if name.eq_ignore_ascii_case("Получить") || name.eq_ignore_ascii_case("Get")
        {
            match arguments {
                [index] => page_get(self, index),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: PAGES_TYPE.name,
                }),
            }
        } else if name.eq_ignore_ascii_case("Индекс") || name.eq_ignore_ascii_case("IndexOf")
        {
            match arguments {
                [page] => page_index_of(self, page),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Индекс",
                    receiver: PAGES_TYPE.name,
                }),
            }
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: PAGES_TYPE.name,
            })
        }
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

impl ObjectProtocol for PageObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PAGE_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        page_property(self, name)
    }

    // Страница — ССЫЛКА на место в документе: `Страницы[0] = Страницы[0]`
    // — «Да», `Страницы[0] = Страницы[1]` — «Нет» (измерено).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.state), self.index))
    }
}

impl ObjectProtocol for AttachmentsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ATTACHMENTS_TYPE
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Количество") || name.eq_ignore_ascii_case("Count") {
            attachment_count(self).map(|len| BslValue::number_from_i64(len as i64))
        } else if name.eq_ignore_ascii_case("Получить") || name.eq_ignore_ascii_case("Get")
        {
            match arguments {
                [index] => attachment_get(self, index),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: ATTACHMENTS_TYPE.name,
                }),
            }
        } else if name.eq_ignore_ascii_case("Найти") || name.eq_ignore_ascii_case("Find") {
            attachment_find(self, arguments)
        } else if name.eq_ignore_ascii_case("Добавить") || name.eq_ignore_ascii_case("Add")
        {
            attachment_add(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Удалить") || name.eq_ignore_ascii_case("Delete")
        {
            attachment_delete(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Очистить") || name.eq_ignore_ascii_case("Clear")
        {
            if !arguments.is_empty() {
                return Err(RtError::MethodNotApplicable {
                    method: "Очистить",
                    receiver: ATTACHMENTS_TYPE.name,
                });
            }
            attachment_clear(self)?;
            Ok(BslValue::Undefined)
        } else if name.eq_ignore_ascii_case("Индекс") || name.eq_ignore_ascii_case("IndexOf")
        {
            match arguments {
                [item] => attachment_index_of(self, item),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Индекс",
                    receiver: ATTACHMENTS_TYPE.name,
                }),
            }
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: ATTACHMENTS_TYPE.name,
            })
        }
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

impl ObjectProtocol for AttachmentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ATTACHMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        attachment_property(self, name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_attachment_property(self, name, &value)
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((state_addr(&self.items), self.index))
    }
}
