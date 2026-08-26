//! Кодировщик подмножества x86-64, которого хватает шаблонному JIT-у.
//!
//! Не ассемблер общего назначения: здесь ровно те формы инструкций,
//! которые встречаются в порождаемом коде. Каждая — с байтами в
//! комментарии, потому что проверять их можно только чтением справочника
//! и тестом.
//!
//! Соглашение о вызовах — System V AMD64: целочисленные аргументы идут в
//! rdi, rsi, rdx, rcx, r8, r9; результат в rax; rbx и rbp сохраняются
//! вызываемым; rsp перед `call` обязан быть кратен 16.

/// Регистры, которые нам нужны по именам. Номера — те, что кодируются в
/// ModRM/REX.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
}

/// Ссылка на ещё не известный адрес перехода: место в буфере, куда надо
/// вписать смещение, когда цель станет известна.
pub struct Patch {
    /// Смещение поля rel32 в буфере.
    at: usize,
}

#[derive(Default)]
pub struct Assembler {
    code: Vec<u8>,
}

impl Assembler {
    pub fn new() -> Self {
        Assembler { code: Vec::new() }
    }

    pub fn here(&self) -> usize {
        self.code.len()
    }

    pub fn finish(self) -> Vec<u8> {
        self.code
    }

    /// `push r64` — 0x50+rd, с REX.B для r8..r15 (нам не нужны).
    pub fn push(&mut self, r: Reg) {
        self.code.push(0x50 + r as u8);
    }

    /// `pop r64` — 0x58+rd.
    pub fn pop(&mut self, r: Reg) {
        self.code.push(0x58 + r as u8);
    }

    /// `mov r64, r64` — REX.W 0x89 /r, направление src -> dst.
    pub fn mov_rr(&mut self, dst: Reg, src: Reg) {
        self.code
            .extend_from_slice(&[0x48, 0x89, modrm_reg(src, dst)]);
    }

    /// `mov r64, imm64` — REX.W 0xB8+rd. Полные 8 байт: адреса шимов не
    /// умещаются в 32 бита и знаково не расширяются.
    pub fn mov_r_imm64(&mut self, dst: Reg, value: u64) {
        self.code.extend_from_slice(&[0x48, 0xb8 + dst as u8]);
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    /// `mov r32, imm32` — 0xB8+rd. Верхняя половина регистра обнуляется
    /// самим процессором, что нам и нужно для беззнаковых номеров.
    pub fn mov_r_imm32(&mut self, dst: Reg, value: u32) {
        self.code.push(0xb8 + dst as u8);
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    /// `mov r8d, imm32` — 0x41 0xB8. `r8` в `ModRM` не кодируется без `REX.B`,
    /// поэтому форма отдельная, а не через `mov_r_imm32`.
    pub fn mov_r8d_imm32(&mut self, value: u32) {
        self.code.extend_from_slice(&[0x41, 0xb8]);
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    /// `mov r64, [base + disp32]` — REX.W 0x8B /r, `mod = 10`.
    pub fn mov_r_membase_disp32(&mut self, dst: Reg, base: Reg, displacement: i32) {
        self.code
            .extend_from_slice(&[0x48, 0x8b, 0x80 | ((dst as u8) << 3) | base as u8]);
        self.code.extend_from_slice(&displacement.to_le_bytes());
    }

    /// `sub qword ptr [base], imm8` — REX.W 0x83 /5, `mod = 00`.
    pub fn sub_mem_imm8(&mut self, base: Reg, value: i8) {
        debug_assert!(base != Reg::Rbp, "[rbp] требует отдельного disp8=0");
        self.code
            .extend_from_slice(&[0x48, 0x83, (5 << 3) | base as u8, value as u8]);
    }

    /// `add rsp, imm8` / `sub rsp, imm8` — REX.W 0x83 /0 и /5.
    pub fn add_rsp(&mut self, value: i8) {
        self.code
            .extend_from_slice(&[0x48, 0x83, 0xc4, value as u8]);
    }

    pub fn sub_rsp(&mut self, value: i8) {
        self.code
            .extend_from_slice(&[0x48, 0x83, 0xec, value as u8]);
    }

    /// `call r64` — 0xFF /2.
    pub fn call_r(&mut self, r: Reg) {
        self.code.extend_from_slice(&[0xff, modrm_digit(2, r)]);
    }

    /// `test rax, rax` — REX.W 0x85 /r.
    pub fn test_rax_rax(&mut self) {
        self.code.extend_from_slice(&[0x48, 0x85, 0xc0]);
    }

    /// `cmp rax, imm8` — REX.W 0x83 /7.
    pub fn cmp_rax_imm8(&mut self, value: i8) {
        self.code
            .extend_from_slice(&[0x48, 0x83, 0xf8, value as u8]);
    }

    pub fn ret(&mut self) {
        self.code.push(0xc3);
    }

    /// `jmp rel32` — 0xE9. Цель дописывается через [`Assembler::patch`].
    pub fn jmp(&mut self) -> Patch {
        self.code.push(0xe9);
        self.reserve_rel32()
    }

    /// `jcc rel32` — 0x0F 0x8x. Условие задаётся кодом `cc`.
    pub fn jcc(&mut self, cc: Cond) -> Patch {
        self.code.extend_from_slice(&[0x0f, 0x80 + cc as u8]);
        self.reserve_rel32()
    }

    fn reserve_rel32(&mut self) -> Patch {
        let at = self.code.len();
        self.code.extend_from_slice(&[0, 0, 0, 0]);
        Patch { at }
    }

    /// Вписывает в зарезервированное поле смещение до `target`.
    /// rel32 отсчитывается от КОНЦА инструкции перехода, то есть от
    /// `at + 4`.
    pub fn patch(&mut self, patch: Patch, target: usize) {
        let from = (patch.at + 4) as i64;
        let rel = target as i64 - from;
        debug_assert!(i32::try_from(rel).is_ok(), "переход не влезает в rel32");
        let bytes = (rel as i32).to_le_bytes();
        self.code[patch.at..patch.at + 4].copy_from_slice(&bytes);
    }
}

/// Коды условий, которые нам нужны.
#[derive(Clone, Copy)]
pub enum Cond {
    /// ZF = 0 — «результат не ноль».
    NotZero = 0x5,
    /// ZF = 1 — «равно».
    Zero = 0x4,
}

/// `ModRM` для формы «регистр-регистр»: `mod = 11`, `reg` = поле `/r`, `rm` = второй.
fn modrm_reg(reg: Reg, rm: Reg) -> u8 {
    0xc0 | ((reg as u8) << 3) | rm as u8
}

/// `ModRM`, где поле `/r` — не регистр, а цифра расширения кода операции.
fn modrm_digit(digit: u8, rm: Reg) -> u8 {
    0xc0 | (digit << 3) | rm as u8
}

#[cfg(test)]
mod tests {
    use super::super::mem::ExecutableBuffer;
    use super::*;

    #[test]
    fn a_generated_function_returns_a_constant() {
        let mut a = Assembler::new();
        a.mov_r_imm32(Reg::Rax, 42);
        a.ret();
        let buf = ExecutableBuffer::new(&a.finish()).unwrap();
        let f: extern "C" fn() -> u64 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        assert_eq!(f(), 42);
    }

    #[test]
    fn a_generated_function_calls_through_a_register_and_keeps_the_argument() {
        // Пролог, сохранение аргумента в rbx, вызов чужой функции через
        // rax, возврат её результата — ровно та последовательность, из
        // которой состоит порождаемый код.
        extern "C" fn triple(x: u64) -> u64 {
            x * 3
        }
        let mut a = Assembler::new();
        a.push(Reg::Rbp);
        a.push(Reg::Rbx);
        a.sub_rsp(8);
        a.mov_rr(Reg::Rbx, Reg::Rdi);
        a.mov_rr(Reg::Rdi, Reg::Rbx);
        a.mov_r_imm64(Reg::Rax, triple as *const () as u64);
        a.call_r(Reg::Rax);
        a.add_rsp(8);
        a.pop(Reg::Rbx);
        a.pop(Reg::Rbp);
        a.ret();
        let buf = ExecutableBuffer::new(&a.finish()).unwrap();
        let f: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        assert_eq!(f(14), 42);
    }

    #[test]
    fn a_forward_jump_skips_the_code_between() {
        let mut a = Assembler::new();
        a.mov_r_imm32(Reg::Rax, 42);
        let over = a.jmp();
        a.mov_r_imm32(Reg::Rax, 7); // должно быть перепрыгнуто
        let target = a.here();
        a.patch(over, target);
        a.ret();
        let buf = ExecutableBuffer::new(&a.finish()).unwrap();
        let f: extern "C" fn() -> u64 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        assert_eq!(f(), 42);
    }

    #[test]
    fn a_conditional_jump_reads_the_flags_of_the_previous_test() {
        // rax = аргумент; если он ноль — вернуть 1, иначе 2. Проверяет и
        // test, и jcc, и что патч назад (в уже пройденный код) не нужен.
        let mut a = Assembler::new();
        a.mov_rr(Reg::Rax, Reg::Rdi);
        a.test_rax_rax();
        let if_nonzero = a.jcc(Cond::NotZero);
        a.mov_r_imm32(Reg::Rax, 1);
        a.ret();
        let target = a.here();
        a.patch(if_nonzero, target);
        a.mov_r_imm32(Reg::Rax, 2);
        a.ret();
        let buf = ExecutableBuffer::new(&a.finish()).unwrap();
        let f: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        assert_eq!(f(0), 1);
        assert_eq!(f(5), 2);
    }

    #[test]
    fn generated_code_decrements_a_counter_through_context() {
        #[repr(C)]
        struct Context {
            padding: u64,
            counter: *mut usize,
        }

        let mut a = Assembler::new();
        a.mov_rr(Reg::Rbx, Reg::Rdi);
        a.mov_r_membase_disp32(Reg::Rax, Reg::Rbx, 8);
        a.sub_mem_imm8(Reg::Rax, 1);
        a.mov_r_imm32(Reg::Rax, 0);
        a.ret();
        let buf = ExecutableBuffer::new(&a.finish()).unwrap();
        let f: extern "C" fn(*mut Context) -> u64 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        let mut counter = 3usize;
        let mut context = Context {
            padding: 0,
            counter: &mut counter,
        };
        assert_eq!(f(&mut context), 0);
        assert_eq!(counter, 2);
    }
}
