//! Непрозрачное BSL-значение `Обещание`.
//!
//! Само состояние ожидания принадлежит одному запуску VM. Значение несёт
//! только пару идентификаторов и не переносит в базовый runtime будущие,
//! каналы или типы конкретного исполнителя host-операций.

use crate::{ObjectProtocol, TypeDescriptor};

/// Уникальный идентификатор одного запуска VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionToken(u64);

impl ExecutionToken {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Номер обещания внутри одного запуска.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PromiseId(u64);

impl PromiseId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Непрозрачная ссылка на состояние обещания в таблице `Execution`.
#[derive(Debug)]
pub struct PromiseValue {
    execution_token: ExecutionToken,
    promise_id: PromiseId,
}

impl PromiseValue {
    #[must_use]
    pub const fn new(execution_token: ExecutionToken, promise_id: PromiseId) -> Self {
        Self {
            execution_token,
            promise_id,
        }
    }

    #[must_use]
    pub const fn execution_token(&self) -> ExecutionToken {
        self.execution_token
    }

    #[must_use]
    pub const fn promise_id(&self) -> PromiseId {
        self.promise_id
    }
}

/// Тип намеренно не имеет конструктора, методов и свойств BSL.
pub static PROMISE_TYPE: TypeDescriptor = TypeDescriptor::new(crate::PACKAGE_NAME, "Обещание");

impl ObjectProtocol for PromiseValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PROMISE_TYPE
    }
}
