/// Байт-код — `enum` в стиле Lua-VM, но без битовой упаковки: 2003-й год уже
/// прошёл, а исчерпывающий `match` при добавлении опкода того стоит.
///
/// `target` у `Jump`/`JumpIfFalse` — АБСОЛЮТНЫЙ индекс инструкции в чанке, не
/// относительное смещение: в M3 чанк не переиспользуется и не релоцируется,
/// поэтому усложнять патчинг под относительные прыжки незачем.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instr {
    Move { dst: u8, src: u8 },
    LoadConst { dst: u8, k: u16 },
    LoadBool { dst: u8, val: bool },
    LoadUndefined { dst: u8 },
    LoadNull { dst: u8 },

    Add { dst: u8, a: u8, b: u8 },
    Sub { dst: u8, a: u8, b: u8 },
    Mul { dst: u8, a: u8, b: u8 },
    Div { dst: u8, a: u8, b: u8 },
    Neg { dst: u8, src: u8 },

    Not { dst: u8, src: u8 },
    /// `И`/`ИЛИ` в BSL не короткозамкнутые — оба операнда всегда уже
    /// вычислены в `a`/`b` до этой инструкции, здесь только комбинирование.
    And { dst: u8, a: u8, b: u8 },
    Or { dst: u8, a: u8, b: u8 },

    Eq { dst: u8, a: u8, b: u8 },
    NotEq { dst: u8, a: u8, b: u8 },
    Lt { dst: u8, a: u8, b: u8 },
    Gt { dst: u8, a: u8, b: u8 },
    Le { dst: u8, a: u8, b: u8 },
    Ge { dst: u8, a: u8, b: u8 },

    Jump { target: i16 },
    /// Условие обязано быть `Булево` — VM бросает ошибку типа, если нет
    /// (строгая булевость: `Если 1 Тогда` не приводится, а падает).
    JumpIfFalse { cond: u8, target: i16 },
}
