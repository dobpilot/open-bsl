//! Окружения импортов с общим неизменяемым хранилищем.

use std::{collections::HashMap, ops::Range, sync::Arc};

use crate::ModuleImport;

#[derive(Debug, Clone)]
struct Part {
    entries: Arc<[ModuleImport]>,
    range: Range<usize>,
}

/// Упорядоченные импорты программы. Участки общего окружения конфигурации
/// разделяют хранилище; исключение собственного модуля не копирует соседей.
#[derive(Debug, Clone, Default)]
pub struct ModuleImports {
    parts: Vec<Part>,
    len: usize,
}

impl ModuleImports {
    /// Число логических записей, независимо от разделения хранилища.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Проверяет отсутствие записей.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn shared_alias_table(&self) -> Option<&Arc<[ModuleImport]>> {
        let first = self.parts.first()?;
        let mut end = 0;
        for part in &self.parts {
            if !Arc::ptr_eq(&first.entries, &part.entries) || part.range.start < end {
                return None;
            }
            end = part.range.end;
        }
        Some(&first.entries)
    }

    /// Обходит записи в порядке разрешения имён.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &ModuleImport> {
        self.parts
            .iter()
            .flat_map(|part| part.entries[part.range.clone()].iter())
    }

    /// Добавляет собственную запись, не изменяя общее хранилище.
    pub fn push(&mut self, import: ModuleImport) {
        self.append_part(Arc::from([import]), 0..1);
    }

    fn append_part(&mut self, entries: Arc<[ModuleImport]>, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        self.len += range.len();
        if let Some(last) = self.parts.last_mut()
            && Arc::ptr_eq(&last.entries, &entries)
            && last.range.end == range.start
        {
            last.range.end = range.end;
        } else {
            self.parts.push(Part { entries, range });
        }
    }

    /// Добавляет диапазон другого окружения, сохраняя общее хранилище.
    ///
    /// # Panics
    /// Паникует, если диапазон выходит за пределы исходного окружения.
    pub fn extend_shared(&mut self, source: &Self, range: Range<usize>) {
        assert!(range.start <= range.end && range.end <= source.len);
        let mut offset = 0;
        for part in &source.parts {
            let start = range.start.saturating_sub(offset);
            let end = range.end.saturating_sub(offset).min(part.range.len());
            if start < end {
                self.append_part(
                    Arc::clone(&part.entries),
                    part.range.start + start..part.range.start + end,
                );
            }
            offset += part.range.len();
            if offset >= range.end {
                break;
            }
        }
    }

    /// Строит окружение, разделяя совпадающие записи с общим каталогом.
    /// Порядок, повторы и собственные псевдонимы сохраняются без изменений.
    pub fn sharing(entries: impl IntoIterator<Item = ModuleImport>, common: &Self) -> Self {
        let index: HashMap<_, _> = common
            .iter()
            .enumerate()
            .map(|(i, import)| ((import.alias.as_str(), import.module, import.global), i))
            .collect();
        let mut result = Self::default();
        let mut own = Vec::new();
        let flush = |result: &mut Self, own: &mut Vec<ModuleImport>| {
            if !own.is_empty() {
                let entries: Arc<[ModuleImport]> = std::mem::take(own).into();
                let len = entries.len();
                result.append_part(entries, 0..len);
            }
        };
        for import in entries {
            if let Some(&i) = index.get(&(import.alias.as_str(), import.module, import.global)) {
                flush(&mut result, &mut own);
                result.extend_shared(common, i..i + 1);
            } else {
                own.push(import);
            }
        }
        flush(&mut result, &mut own);
        result
    }
}

impl From<Vec<ModuleImport>> for ModuleImports {
    fn from(entries: Vec<ModuleImport>) -> Self {
        let len = entries.len();
        let mut result = Self::default();
        result.append_part(entries.into(), 0..len);
        result
    }
}

impl FromIterator<ModuleImport> for ModuleImports {
    fn from_iter<T: IntoIterator<Item = ModuleImport>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl PartialEq for ModuleImports {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}
impl Eq for ModuleImports {}

impl std::ops::Index<usize> for ModuleImports {
    type Output = ModuleImport;

    fn index(&self, mut index: usize) -> &Self::Output {
        for part in &self.parts {
            if index < part.range.len() {
                return &part.entries[part.range.start + index];
            }
            index -= part.range.len();
        }
        panic!("номер импорта вне окружения")
    }
}

type PartsIter<'a> = std::iter::FlatMap<
    std::slice::Iter<'a, Part>,
    std::slice::Iter<'a, ModuleImport>,
    fn(&'a Part) -> std::slice::Iter<'a, ModuleImport>,
>;

/// Итератор по логическому окружению импортов без материализации массива.
pub struct ModuleImportsIter<'a> {
    inner: PartsIter<'a>,
}

impl<'a> Iterator for ModuleImportsIter<'a> {
    type Item = &'a ModuleImport;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl DoubleEndedIterator for ModuleImportsIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a> IntoIterator for &'a ModuleImports {
    type Item = &'a ModuleImport;
    type IntoIter = ModuleImportsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ModuleImportsIter {
            inner: self
                .parts
                .iter()
                .flat_map(|part| part.entries[part.range.clone()].iter()),
        }
    }
}

#[cfg(test)]
#[path = "imports_tests.rs"]
mod tests;
