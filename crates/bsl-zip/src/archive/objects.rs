//! Объектный протокол читателей, писателей и их коллекций.

use super::*;

// --- объектный протокол -----------------------------------------------------

impl ObjectProtocol for ReaderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        // У читателя свойств ровно два, и оба измерены; `Кодировка`,
        // `Формат`, `ИмяФайла` и `РазмерАрхива` платформа не знает.
        if name.eq_ignore_ascii_case("Элементы") || name.eq_ignore_ascii_case("Items") {
            entries(self)
        } else if name.eq_ignore_ascii_case("Комментарий") || name.eq_ignore_ascii_case("Comment")
        {
            comment(self)
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if eq(name, "Открыть", "Open") {
            open(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "Закрыть", "Close") {
            close(self)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "Извлечь", "Extract") {
            extract(&self.state, self.descriptor().name, arguments)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "ИзвлечьВсе", "ExtractAll") {
            extract_all(&self.state, self.descriptor().name, arguments)?;
            Ok(BslValue::Undefined)
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: self.descriptor().name,
            })
        }
    }

    // Измерено: `ЗначениеЗаполнено` от читателя — «Да»; ошибка на закрытом
    // архиве важнее, чем ноль элементов.
    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for EntriesObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if eq(name, "Количество", "Count") {
            count(self).map(|len| BslValue::number_from_i64(len as i64))
        } else if eq(name, "Получить", "Get") {
            match arguments {
                [index] => get(self, entry_index(index)?),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: self.descriptor().name,
                }),
            }
        } else if eq(name, "Найти", "Find") {
            // Аргумент РОВНО один: измерено, что `Элементы.Найти("шум.bin",
            // 1)` платформа отвергает — «Слишком много фактических
            // параметров».
            match arguments {
                [name] => find(self, name),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Найти",
                    receiver: self.descriptor().name,
                }),
            }
        } else if eq(name, "Извлечь", "Extract") {
            extract(&self.state, self.descriptor().name, arguments)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "ИзвлечьВсе", "ExtractAll") {
            extract_all(&self.state, self.descriptor().name, arguments)?;
            Ok(BslValue::Undefined)
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: self.descriptor().name,
            })
        }
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        get(self, entry_index(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        count(self)
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn entry_index(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}

impl ObjectProtocol for EntryObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        entry_prop(self, name)
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        // Статус-кво прежней диспетчеризации: `Извлечь`/`ИзвлечьВсе`
        // принимали любой из трёх объектов чтения, включая элемент.
        if eq(name, "Извлечь", "Extract") {
            extract(&self.state, self.descriptor().name, arguments)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "ИзвлечьВсе", "ExtractAll") {
            extract_all(&self.state, self.descriptor().name, arguments)?;
            Ok(BslValue::Undefined)
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: self.descriptor().name,
            })
        }
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for WriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor()
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if eq(name, "Открыть", "Open") {
            writer_open(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "Добавить", "Add") {
            writer_add(self, arguments)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "Записать", "Write") {
            // Аргументов нет: измерено, что `Записать(имя)` платформа
            // встречает «Слишком много фактических параметров».
            if !arguments.is_empty() {
                return Err(RtError::MethodNotApplicable {
                    method: "Записать",
                    receiver: self.descriptor().name,
                });
            }
            writer_write(self)?;
            Ok(BslValue::Undefined)
        } else if eq(name, "ПолучитьДвоичныеДанные", "GetBinaryData") {
            writer_binary_data(self)
        } else {
            Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: self.descriptor().name,
            })
        }
    }

    // `ЗначениеЗаполнено` от писателя — измеренная ошибка «Проверка
    // мутабельных значений на заполненность не поддерживается»; её отдаёт
    // реализация протокола по умолчанию.
}
