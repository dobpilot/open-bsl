//! Типизированные ошибки host-границы фоновых заданий.
//!
//! В BSL все ошибки возможности представлены одним классом ловимого
//! исключения с обычной `ИнформацияОбОшибке`; Rust-встраивание различает
//! причины закрытым [`HostErrorCode`] (план фоновых заданий, «Переносимый
//! host-контракт»). Новый платформенно-несовместимый enum в BSL не
//! публикуется — наружу уходит только текст.

/// Закрытый код ошибки host-границы. Список исчерпывающий для текущей
/// поверхности; расширение — обычное добавление варианта, BSL-контракт
/// от этого не меняется.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostErrorCode {
    /// Задание неизвестно runtime: вытеснено из истории либо никогда не
    /// существовало. Live-методы старого снимка не притворяются
    /// успешными и не скрывают потерю истории пустым результатом.
    JobExpired,
    /// Runtime фоновых заданий закрыт (`shutdown` или Drop владельца).
    RuntimeClosed,
    /// Runtime сломан: последовательные паники запуска worker или ошибка
    /// каталога; автоматического recovery нет.
    RuntimeBroken,
    /// Явный ресурсный лимит: admission, бюджет сообщений, staging
    /// временного хранилища, размер диагностики.
    ResourceLimit,
    /// Неблокирующий sink сообщений отказал (backpressure). Скрытых
    /// retry нет; история сообщения при этом не теряется.
    HostBackpressure,
    /// Адрес временного хранилища чужой, закрытый или удалённый.
    InvalidTemporaryStorageAddress,
    /// Host-профиль сеанса заданий недоступен: фабрика вернула ошибку
    /// либо профиль не зарегистрирован в этом движке.
    HostProfileUnavailable,
    /// Неверный вызов: цель задания не найдена, не экспортирована или
    /// не годится по контракту.
    InvalidCall,
}

/// Ошибка host-границы: код для Rust-встраивания и готовый текст для
/// BSL-исключения.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    pub code: HostErrorCode,
    pub text: String,
}

impl HostError {
    #[must_use]
    pub fn new(code: HostErrorCode, text: impl Into<String>) -> Self {
        Self {
            code,
            text: text.into(),
        }
    }

    /// Ошибка как ловимое BSL-исключение.
    #[must_use]
    pub fn raise(self) -> crate::RtError {
        crate::RtError::Host(Box::new(self))
    }
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}
