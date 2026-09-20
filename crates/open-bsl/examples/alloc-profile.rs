//! Счётчики System-аллокаций отдельно для подготовки и цикла benchmark.
//! Маркеры добавляются только в измерительную копию исходника.
//! Запуск: cargo run --release -p open-bsl --example alloc-profile -- benchmarks/xml_parse.bsl
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static REALLOC: AtomicUsize = AtomicUsize::new(0);
static BINS: [AtomicUsize; 7] = [const { AtomicUsize::new(0) }; 7];

struct Counter;

fn record(size: usize) {
    COUNT.fetch_add(1, Relaxed);
    BYTES.fetch_add(size, Relaxed);
    let bin = [16, 32, 64, 128, 256, 512]
        .iter()
        .position(|&limit| size <= limit)
        .unwrap_or(6);
    BINS[bin].fetch_add(1, Relaxed);
    PEAK.fetch_max(LIVE.load(Relaxed), Relaxed);
}

// Обёртка сохраняет layout и все условия System; внутри учёта нет аллокаций.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
            if ACTIVE.load(Relaxed) {
                record(layout.size());
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, size) };
        if !new.is_null() {
            LIVE.fetch_add(size, Relaxed);
            LIVE.fetch_sub(layout.size(), Relaxed);
            if ACTIVE.load(Relaxed) {
                REALLOC.fetch_add(1, Relaxed);
                record(size);
            }
        }
        new
    }
}

#[global_allocator]
static ALLOCATOR: Counter = Counter;

fn start() {
    COUNT.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    REALLOC.store(0, Relaxed);
    for bin in &BINS {
        bin.store(0, Relaxed);
    }
    PEAK.store(LIVE.load(Relaxed), Relaxed);
    ACTIVE.store(true, Relaxed);
}

fn report(phase: &str) {
    ACTIVE.store(false, Relaxed);
    eprint!(
        "{phase},{},{},{},{},{}",
        COUNT.load(Relaxed),
        BYTES.load(Relaxed),
        REALLOC.load(Relaxed),
        LIVE.load(Relaxed),
        PEAK.load(Relaxed)
    );
    for bin in &BINS {
        eprint!(",{}", bin.load(Relaxed));
    }
    eprintln!();
}

struct Output {
    line: Vec<u8>,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for &byte in bytes {
            if byte == b'\n' {
                match self.line.as_slice() {
                    b"ALLOC_BEGIN" => {
                        report("prepare");
                        start();
                    }
                    b"ALLOC_END" => report("run"),
                    _ => {
                        io::stdout().write_all(&self.line)?;
                        io::stdout().write_all(b"\n")?;
                    }
                }
                self.line.clear();
            } else {
                self.line.push(byte);
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn main() {
    eprintln!(
        "layout,BslValue,{},BslObject,{}",
        std::mem::size_of::<bsl_rt::BslValue>(),
        std::mem::size_of::<bsl_rt::BslObject>()
    );
    let path = std::env::args().nth(1).expect("нужен benchmark.bsl");
    let source = std::fs::read_to_string(path).unwrap();
    let begin = "Т = ТекущаяУниверсальнаяДатаВМиллисекундах();";
    let end = "Прошло = ТекущаяУниверсальнаяДатаВМиллисекундах() - Т;";
    assert_eq!(source.matches(begin).count(), 1);
    assert_eq!(source.matches(end).count(), 1);
    let source = source
        .replace(begin, &format!("Сообщить(\"ALLOC_BEGIN\");\n{begin}"))
        .replace(end, &format!("{end}\nСообщить(\"ALLOC_END\");"));
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine
        .state_builder()
        .stdout(Output {
            line: Vec::with_capacity(1024),
        })
        .build();
    eprintln!(
        "phase,requests,requested_bytes,reallocations,live_bytes,peak_live_bytes,le16,le32,le64,le128,le256,le512,gt512"
    );
    start();
    state.run(&module).unwrap();
    ACTIVE.store(false, Relaxed);
}
