//! Страница памяти, в которую можно писать байты и потом их исполнять.
//!
//! Без внешних крейтов: `mmap`/`mprotect`/`munmap` вызываются сырыми
//! системными вызовами Linux на x86-64. Взять для этого `libc` было бы
//! проще, но во всём рабочем пространстве ровно две внешние зависимости
//! (`num-bigint` и `rustyline`), каждая с объяснением, зачем она; три
//! системных вызова на такое не тянут.
//!
//! W^X соблюдается: страница живёт сначала как RW (пишем код), потом
//! переводится в RX (исполняем). Одновременно записываемой и исполняемой
//! она не бывает.

use std::arch::asm;

const PROT_READ: usize = 1;
const PROT_WRITE: usize = 2;
const PROT_EXEC: usize = 4;
const MAP_PRIVATE: usize = 0x02;
const MAP_ANONYMOUS: usize = 0x20;

const SYS_MMAP: usize = 9;
const SYS_MPROTECT: usize = 10;
const SYS_MUNMAP: usize = 11;

/// Возвращаемое ядром значение — это указатель ЛИБО отрицательный код
/// ошибки в диапазоне [-4095, -1]. Отдельного errno у сырого вызова нет.
fn failed(ret: isize) -> bool {
    (-4095..0).contains(&ret)
}

unsafe fn sys3(number: usize, a: usize, b: usize, c: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") number => ret,
            in("rdi") a,
            in("rsi") b,
            in("rdx") c,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

unsafe fn sys6(number: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") number => ret,
            in("rdi") a,
            in("rsi") b,
            in("rdx") c,
            in("r10") d,
            in("r8") e,
            in("r9") f,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

/// Готовый к исполнению блок машинного кода. При уничтожении страница
/// возвращается ядру, поэтому держать `JitCode` обязательно всё то время,
/// пока по указателям внутрь него могут прыгнуть.
pub struct ExecutableBuffer {
    ptr: *mut u8,
    len: usize,
}

impl ExecutableBuffer {
    /// Копирует `code` в свежую страницу и делает её исполняемой.
    ///
    /// Возвращает `None`, если ядро отказало (кончилась память, запрещён
    /// `PROT_EXEC` политикой). Отказ — не ошибка: вызывающий просто
    /// останется на интерпретаторе.
    pub fn new(code: &[u8]) -> Option<Self> {
        if code.is_empty() {
            return None;
        }
        // Округление вверх до страницы: mmap всё равно выдаёт целые
        // страницы, а mprotect требует выровненный адрес и длину.
        let page = 4096;
        let len = code.len().div_ceil(page) * page;

        let raw = unsafe {
            sys6(
                SYS_MMAP,
                0,
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                usize::MAX, // fd = -1
                0,
            )
        };
        if failed(raw) {
            return None;
        }
        let ptr = raw as *mut u8;

        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len());
        }

        let protected = unsafe { sys3(SYS_MPROTECT, ptr as usize, len, PROT_READ | PROT_EXEC) };
        if failed(protected) {
            unsafe { sys3(SYS_MUNMAP, ptr as usize, len, 0) };
            return None;
        }

        Some(ExecutableBuffer { ptr, len })
    }

    /// Адрес байта `offset` внутри блока. Точка входа: у нас на один чанк
    /// приходится много входов — по одному на каждую инструкцию, куда
    /// интерпретатор может вернуть управление.
    pub fn entry_at(&self, offset: usize) -> *const u8 {
        debug_assert!(offset < self.len);
        unsafe { self.ptr.add(offset) }
    }
}

impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        unsafe {
            sys3(SYS_MUNMAP, self.ptr as usize, self.len, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_of_machine_code_can_be_written_and_then_called() {
        // mov eax, 42 ; ret — самая маленькая осмысленная проверка того,
        // что страница получена, записана, переведена в исполняемую и
        // действительно исполняется.
        let code = [0xb8, 42, 0x00, 0x00, 0x00, 0xc3];
        let buf = ExecutableBuffer::new(&code).expect("ядро не дало исполняемую страницу");
        let f: extern "C" fn() -> u32 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        assert_eq!(f(), 42);
    }

    #[test]
    fn an_argument_survives_the_call_boundary() {
        // lea eax, [rdi + rdi] ; ret — проверяем, что соглашение о
        // вызовах System V действует: первый аргумент приходит в rdi.
        let code = [0x8d, 0x04, 0x3f, 0xc3];
        let buf = ExecutableBuffer::new(&code).expect("ядро не дало исполняемую страницу");
        let f: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(buf.entry_at(0)) };
        assert_eq!(f(21), 42);
    }
}
