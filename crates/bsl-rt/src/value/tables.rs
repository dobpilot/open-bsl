//! Табличные операции значения BSL; алгоритмы принадлежат `table`.

use super::BslValue;
use crate::{BslNumber, BslObject, NameInterner, RtError, RtResult, ValueTableData, table};
use std::rc::Rc;

impl BslValue {
    pub fn new_table() -> Self {
        BslValue::Object(Rc::new(BslObject::ValueTable(ValueTableData::new())))
    }

    /// `ТаблицаЗначений.Добавить()` -> новая строка.
    pub fn table_add_row(&self) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    let row_id = data.borrow_mut().add_row()?;
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(
                        data.clone(),
                        row_id,
                    ))))
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Таблица.Колонки.Добавить(Имя[, ТипЗначения])`.
    pub fn table_add_column(&self, name: &BslValue, value_type: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::TableColumns(data) => {
                    let name = name.as_str("Колонки.Добавить")?.to_string();
                    let value_types = match value_type {
                        BslValue::Undefined => None,
                        BslValue::Object(value) => match &**value {
                            BslObject::TypeDescription(types) => Some(types.clone()),
                            _ => {
                                return Err(RtError::TypeError {
                                    expected: "ОписаниеТипов",
                                    op: "Колонки.Добавить",
                                });
                            }
                        },
                        _ => {
                            return Err(RtError::TypeError {
                                expected: "ОписаниеТипов",
                                op: "Колонки.Добавить",
                            });
                        }
                    };
                    data.borrow_mut().add_typed_column(&name, value_types);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    // --- ТаблицаЗначений, волна 2 ----------------------------------------

    /// Общий доступ к данным таблицы для методов волны 2 — все они
    /// применимы только к самой `ТаблицаЗначений`, не к строке и не к
    /// коллекции колонок.
    fn as_table(&self, method: &'static str) -> RtResult<&Rc<std::cell::RefCell<ValueTableData>>> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => Ok(data),
                _ => Err(RtError::MethodNotApplicable {
                    method,
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: self.type_name(),
            }),
        }
    }

    /// Разбор списка колонок `"Кол1, Кол2"` в индексы. Пустая строка (или
    /// отсутствующий аргумент) — пустой список, что для `Найти` значит
    /// «искать во всех колонках».
    fn column_indices(data: &ValueTableData, spec: &str) -> RtResult<Vec<usize>> {
        spec.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|name| {
                data.column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))
            })
            .collect()
    }

    /// `Найти(Значение[, Колонки])` -> `СтрокаТаблицыЗначений` либо
    /// `Неопределено`, если ничего не нашлось (не ошибка — это штатный
    /// способ проверить наличие).
    pub fn table_find(&self, value: &BslValue, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Найти")?;
        let cols = match columns {
            BslValue::Undefined => Vec::new(),
            other => {
                let spec = other.as_str("Найти")?.to_string();
                Self::column_indices(&data.borrow(), &spec)?
            }
        };
        let found = data.borrow().find(value, &cols);
        Ok(match found {
            Some(row_id) => BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))),
            None => BslValue::Undefined,
        })
    }

    /// `НайтиСтроки(СтруктураПоиска)` -> `Массив` строк таблицы.
    ///
    /// Имена полей структуры — это имена колонок, поэтому нужен интернер:
    /// поля хранятся `NameId`, а колонки — строками (они заведены в
    /// рантайме через `.Колонки.Добавить`, см. `get_field_by_name`).
    pub fn table_find_rows(&self, criteria: &BslValue, names: &NameInterner) -> RtResult<BslValue> {
        let data = self.as_table("НайтиСтроки")?;
        let BslValue::Object(o) = criteria else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "НайтиСтроки",
            });
        };
        let BslObject::Structure(s) = &**o else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "НайтиСтроки",
            });
        };

        let pairs = {
            let s = s.borrow();
            let d = data.borrow();
            let mut pairs = Vec::with_capacity(s.len());
            for i in 0..s.len() {
                let (field, want) = s.entry_at(i).ok_or(RtError::NotAnObject)?;
                let name = names.name(field).ok_or(RtError::UnknownField(field))?;
                let col = d
                    .column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                pairs.push((col, want));
            }
            pairs
        };

        let ids = data.borrow().find_rows(&pairs);
        Ok(BslValue::new_array(
            ids.into_iter()
                .map(|id| BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), id))))
                .collect(),
        ))
    }

    /// `Сортировать("Кол1 Возр, Кол2 Убыв")`. Живые объекты
    /// `СтрокаТаблицыЗначений` переживают сортировку — см.
    /// `ValueTableData::sort`.
    pub fn table_sort(&self, spec: &BslValue, comparison: &BslValue) -> RtResult<()> {
        let data = self.as_table("Сортировать")?;
        if !matches!(comparison, BslValue::Undefined)
            && !matches!(
                comparison,
                BslValue::Object(value) if matches!(&**value, BslObject::ValueComparison)
            )
        {
            return Err(RtError::TypeError {
                expected: "СравнениеЗначений",
                op: "Сортировать",
            });
        }
        let spec = spec.as_str("Сортировать")?.to_string();
        let keys = {
            let d = data.borrow();
            table::parse_sort_spec(&spec, |name| d.column_index(name))
                .map_err(RtError::UnknownColumn)?
        };
        data.borrow_mut().sort(&keys);
        Ok(())
    }

    /// `ЗаполнитьЗначения(Значение[, Колонки])`.
    pub fn table_fill_values(&self, value: &BslValue, columns: &BslValue) -> RtResult<()> {
        let data = self.as_table("ЗаполнитьЗначения")?;
        let cols = match columns {
            BslValue::Undefined => Vec::new(),
            other => {
                let spec = other.as_str("ЗаполнитьЗначения")?.to_string();
                Self::column_indices(&data.borrow(), &spec)?
            }
        };
        data.borrow_mut().fill_values(value, &cols);
        Ok(())
    }

    /// `Итог("Колонка")` -> `Число`. Про нечисловые значения см.
    /// `ValueTableData::total` (`НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC)`).
    pub fn table_total(&self, column: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Итог")?;
        let name = column.as_str("Итог")?.to_string();
        let d = data.borrow();
        let col = d
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        Ok(BslValue::Number(d.total(col)?))
    }

    // --- ТаблицаЗначений, волна 3 ----------------------------------------

    /// Строка ЭТОЙ таблицы: разворачивает объект `СтрокаТаблицыЗначений` в
    /// текущую позицию. Строка чужой таблицы — ошибка метода, а не «не
    /// найдено»: спутать таблицы легко, и молчаливый `-1` в ответ прятал бы
    /// эту ошибку до самого конца.
    fn row_position(
        data: &Rc<std::cell::RefCell<ValueTableData>>,
        row: &BslValue,
        method: &'static str,
    ) -> RtResult<usize> {
        let BslValue::Object(o) = row else {
            return Err(RtError::TypeError {
                expected: "СтрокаТаблицыЗначений",
                op: method,
            });
        };
        let BslObject::TableRow(owner, row_id) = &**o else {
            return Err(RtError::TypeError {
                expected: "СтрокаТаблицыЗначений",
                op: method,
            });
        };
        if !Rc::ptr_eq(owner, data) {
            return Err(RtError::MethodNotApplicable {
                method,
                receiver: "СтрокаТаблицыЗначений другой таблицы",
            });
        }
        data.borrow().pos_of(*row_id).ok_or(RtError::RowInvalidated)
    }

    /// Список колонок `"Кол1, Кол2"` -> индексы; `Неопределено` или пустая
    /// строка -> ВСЕ колонки в их порядке. Разворачивать «все» здесь, а не
    /// в `ValueTableData`, нарочно: слой данных не должен знать, что пустой
    /// список для `Скопировать` значит «все», а для `Найти` — «любая».
    fn columns_or_all(
        data: &ValueTableData,
        spec: &BslValue,
        method: &'static str,
    ) -> RtResult<Vec<usize>> {
        let all = || (0..data.column_names.len()).collect::<Vec<usize>>();
        match spec {
            BslValue::Undefined => Ok(all()),
            other => {
                let spec = other.as_str(method)?.to_string();
                if spec.trim().is_empty() {
                    return Ok(all());
                }
                Self::column_indices(data, &spec)
            }
        }
    }

    /// `Скопировать([Строки], [Колонки])` -> новая `ТаблицаЗначений`.
    ///
    /// `Строки` — `Массив` строк ЭТОЙ таблицы (порядок массива и есть
    /// порядок строк копии) либо `Неопределено` — тогда все строки в
    /// текущем порядке.
    pub fn table_copy(&self, rows: &BslValue, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Скопировать")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "Скопировать")?;
        let positions: Vec<usize> = match rows {
            BslValue::Undefined => (0..data.borrow().row_count()).collect(),
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => {
                    let items = items.borrow();
                    let mut out = Vec::with_capacity(items.len());
                    for row in items.iter() {
                        out.push(Self::row_position(data, row, "Скопировать")?);
                    }
                    out
                }
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Массив",
                        op: "Скопировать",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: "Скопировать",
                });
            }
        };
        let copy = data.borrow().copy_of(&positions, &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// Перегрузка `Скопировать(Отбор, Колонки)`, где `Отбор` — структура
    /// с именами колонок и требуемыми значениями.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при неверном типе отбора или неизвестной колонке.
    pub fn table_copy_by_filter(
        &self,
        criteria: &BslValue,
        columns: &BslValue,
        names: &NameInterner,
    ) -> RtResult<BslValue> {
        let data = self.as_table("Скопировать")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "Скопировать")?;
        let BslValue::Object(criteria_object) = criteria else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "Скопировать",
            });
        };
        let BslObject::Structure(criteria_data) = &**criteria_object else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "Скопировать",
            });
        };

        let pairs = {
            let criteria_data = criteria_data.borrow();
            let table_data = data.borrow();
            let mut pairs = Vec::with_capacity(criteria_data.len());
            for i in 0..criteria_data.len() {
                let (field, value) = criteria_data.entry_at(i).ok_or(RtError::NotAnObject)?;
                let name = names.name(field).ok_or(RtError::UnknownField(field))?;
                let col = table_data
                    .column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                pairs.push((col, value));
            }
            pairs
        };
        let positions: Vec<usize> = {
            let table_data = data.borrow();
            table_data
                .find_rows(&pairs)
                .into_iter()
                .filter_map(|row_id| table_data.pos_of(row_id))
                .collect()
        };
        let copy = data.borrow().copy_of(&positions, &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// `СкопироватьКолонки([Колонки])` -> пустая таблица той же структуры.
    /// Это `Скопировать` без единой строки, а не отдельный алгоритм.
    pub fn table_copy_columns(&self, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("СкопироватьКолонки")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "СкопироватьКолонки")?;
        let copy = data.borrow().copy_of(&[], &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// `ВыгрузитьКолонку(Колонка)` -> `Массив` значений в текущем порядке
    /// строк.
    pub fn table_unload_column(&self, column: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("ВыгрузитьКолонку")?;
        let name = column.as_str("ВыгрузитьКолонку")?.to_string();
        let d = data.borrow();
        let col = d
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        Ok(BslValue::new_array(d.unload_column(col)))
    }

    /// `ЗагрузитьКолонку(Массив, Колонка)`. Про несовпадение длин — см.
    /// `ValueTableData::load_column`
    /// (`НЕ ИЗМЕРЕНО(TABLE.LOAD_COLUMN.LENGTH_MISMATCH)`).
    pub fn table_load_column(&self, values: &BslValue, column: &BslValue) -> RtResult<()> {
        let data = self.as_table("ЗагрузитьКолонку")?;
        let name = column.as_str("ЗагрузитьКолонку")?.to_string();
        let BslValue::Object(o) = values else {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "ЗагрузитьКолонку",
            });
        };
        let BslObject::Array(items) = &**o else {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "ЗагрузитьКолонку",
            });
        };
        let col = data
            .borrow()
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        let values = items.borrow().clone();
        data.borrow_mut().load_column(col, &values);
        Ok(())
    }

    /// `Сдвинуть(Строка, Смещение)` — `Строка` — это либо объект строки, либо
    /// её индекс. Целевая позиция вне таблицы — `IndexOutOfBounds`, а не
    /// зажатие в границы.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.MOVE.OUT_OF_RANGE)`: падает ли платформа или молча
    /// зажимает. Взята ошибка: `Сдвинуть(ПерваяСтрока, -1)`, тихо ничего не
    /// сделавший, — та же категория беды, что и `Сортировать("Опечатка")`,
    /// молча ничего не отсортировавшая.
    pub fn table_move(&self, row: &BslValue, offset: &BslValue) -> RtResult<()> {
        let data = self.as_table("Сдвинуть")?;
        let from = match row {
            BslValue::Number(_) => {
                let i = Self::index_as_usize(row)?;
                let len = data.borrow().row_count();
                if i >= len {
                    return Err(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    });
                }
                i
            }
            other => Self::row_position(data, other, "Сдвинуть")?,
        };
        let BslValue::Number(n) = offset else {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "Сдвинуть",
            });
        };
        let offset = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        let len = data.borrow().row_count();
        data.borrow_mut()
            .move_row(from, offset)
            .map(|_| ())
            .ok_or(RtError::IndexOutOfBounds {
                index: from as i64 + offset,
                len,
            })
    }

    /// `Индекс(Строка)` -> `Число`, позиция строки (с нуля, как у
    /// `Получить`/`Удалить`).
    pub fn table_index_of(&self, row: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Индекс")?;
        let pos = Self::row_position(data, row, "Индекс")?;
        Ok(BslValue::Number(BslNumber::from_i64(pos as i64)))
    }

    /// `Свернуть(КолонкиГруппировки[, КолонкиСуммирования])` — группировка
    /// на месте. Три неизмеренных решения (судьба прочих колонок, порядок
    /// строк, нечисловые значения) описаны у `ValueTableData::collapse`.
    pub fn table_collapse(&self, group: &BslValue, sum: &BslValue) -> RtResult<()> {
        let data = self.as_table("Свернуть")?;
        let (group_cols, sum_cols) = {
            let d = data.borrow();
            let group_cols = Self::column_indices(&d, &group.as_str("Свернуть")?.to_string())?;
            let sum_cols = match sum {
                BslValue::Undefined => Vec::new(),
                other => Self::column_indices(&d, &other.as_str("Свернуть")?.to_string())?,
            };
            (group_cols, sum_cols)
        };
        data.borrow_mut().collapse(&group_cols, &sum_cols)?;
        Ok(())
    }
}
