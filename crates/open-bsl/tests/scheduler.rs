use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use open_bsl::Engine;

#[derive(Clone, Default)]
struct SharedWriter(Rc<RefCell<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SharedWriter {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).unwrap()
    }
}

#[test]
fn backward_edges_switch_tasks_in_fifo_order_in_interpreter_and_jit() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Перем Порядок;\n\
             Порядок = Новый Массив;\n\
             Асинх Функция А()\n\
               Для Номер = 1 По 3 Цикл Порядок.Добавить(\"A\"); КонецЦикла;\n\
               Возврат 1;\n\
             КонецФункции\n\
             Асинх Функция Б()\n\
               Для Номер = 1 По 3 Цикл Порядок.Добавить(\"B\"); КонецЦикла;\n\
               Возврат 2;\n\
             КонецФункции\n\
             Асинх Процедура Проверить()\n\
               ОбещаниеА = А(); ОбещаниеБ = Б();\n\
               Ждать ОбещаниеА; Ждать ОбещаниеБ;\n\
               Для Номер = 0 По 5 Цикл Сообщить(Порядок[Номер]); КонецЦикла;\n\
             КонецПроцедуры\n\
             Проверить();",
        )
        .unwrap();

    for jit in [false, true] {
        let output = SharedWriter::default();
        engine
            .state_builder()
            .jit(jit)
            .safe_points_per_quantum(1)
            .stdout(output.clone())
            .build()
            .run(&module)
            .unwrap();
        assert_eq!(output.text(), "A\nB\nA\nB\nA\nB\n", "jit={jit}");
    }
}

#[test]
fn fast_numeric_for_yields_only_when_another_task_is_live() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Асинх Процедура Длинная()\n\
               Сообщить(\"long-start\");\n\
               Для Номер = 1 По 100 Цикл КонецЦикла;\n\
               Сообщить(\"long-end\");\n\
             КонецПроцедуры\n\
             Асинх Процедура Короткая() Сообщить(\"short\"); КонецПроцедуры\n\
             Длинная(); Короткая();",
        )
        .unwrap();

    for jit in [false, true] {
        let output = SharedWriter::default();
        engine
            .state_builder()
            .jit(jit)
            .safe_points_per_quantum(1)
            .stdout(output.clone())
            .build()
            .run(&module)
            .unwrap();
        assert_eq!(output.text(), "long-start\nshort\nlong-end\n");
    }
}

#[test]
fn backward_goto_and_frame_changes_are_scheduler_safe_points() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Функция Рекурсия(Глубина)\n\
               Если Глубина = 0 Тогда Возврат 0; КонецЕсли;\n\
               Возврат Рекурсия(Глубина - 1);\n\
             КонецФункции\n\
             Асинх Процедура ЦиклИРекурсия()\n\
               Сообщить(\"long-start\");\n\
               Номер = 0; Пока Номер < 3 Цикл Номер = Номер + 1; КонецЦикла;\n\
               Рекурсия(3);\n\
               Сообщить(\"long-end\");\n\
             КонецПроцедуры\n\
             Асинх Процедура Короткая() Сообщить(\"short\"); КонецПроцедуры\n\
             ЦиклИРекурсия(); Короткая();",
        )
        .unwrap();

    for jit in [false, true] {
        let output = SharedWriter::default();
        engine
            .state_builder()
            .jit(jit)
            .safe_points_per_quantum(2)
            .stdout(output.clone())
            .build()
            .run(&module)
            .unwrap();
        assert_eq!(output.text(), "long-start\nshort\nlong-end\n", "jit={jit}");
    }
}

#[test]
fn zero_scheduler_quantum_is_rejected_before_execution() {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile("Возврат 1;").unwrap();
    let error = engine
        .state_builder()
        .safe_points_per_quantum(0)
        .build()
        .run(&module)
        .unwrap_err();
    assert!(error.to_string().contains("квант планировщика"));
}
