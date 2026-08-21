//! Feature-тесты фасада: каждый статический компонент виден исходному
//! коду, записывает зависимость в заголовок байт-кода и исполняется, а
//! без своей cargo-фичи его имена — ошибка компиляции.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use open_bsl::{
    Arity, CallContext, Engine, FunctionCode, FunctionDescriptor, FunctionKind, LibraryDependency,
    LibraryDescriptor, RtResult, Value,
};

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

fn host_answer(_context: &mut CallContext<'_>, _arguments: &[Value]) -> RtResult<Value> {
    Ok(Value::Boolean(true))
}

const HOST_FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["ОтветХоста", "HostAnswer"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: host_answer,
}];

fn host_library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: "example-host",
        version: "1.0.0",
        dependencies: &[LibraryDependency {
            package: bsl_rt::PACKAGE_NAME,
            version: bsl_rt::PACKAGE_VERSION,
        }],
        functions: HOST_FUNCTIONS,
        constructors: &[],
        types: &[],
    }
}

#[test]
fn state_exec_and_eval_hide_the_internal_pipeline() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();

    assert_eq!(state.eval("2 + 2").unwrap().to_string(), "4");
    assert!(matches!(
        state.exec("Возврат Истина;").unwrap(),
        Value::Boolean(true)
    ));
}

#[test]
fn module_round_trips_through_the_public_bytecode_api() {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile("Возврат 42;").unwrap();
    let restored = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();

    assert_eq!(engine.new_state().run(&restored).unwrap().to_string(), "42");
}

#[test]
fn states_use_independent_stdout_and_stderr() {
    let engine = Engine::builder().build().unwrap();
    let out_a = SharedWriter::default();
    let err_a = SharedWriter::default();
    let out_b = SharedWriter::default();
    let err_b = SharedWriter::default();
    let mut state_a = engine
        .state_builder()
        .stdout(out_a.clone())
        .stderr(err_a.clone())
        .build();
    let mut state_b = engine
        .state_builder()
        .stdout(out_b.clone())
        .stderr(err_b.clone())
        .jit(true)
        .build();

    state_a.exec("Сообщить(\"a\");").unwrap();
    state_b.exec("Сообщить(\"b\");").unwrap();

    assert_eq!(out_a.text(), "a\n");
    assert_eq!(out_b.text(), "b\n");
    assert_eq!(err_a.text(), "");
    assert_eq!(err_b.text(), "");
}

#[test]
fn engine_builder_exposes_a_registered_component_to_source_code() {
    let engine = Engine::builder()
        .register_library(host_library())
        .build()
        .unwrap();
    let module = engine.compile("Возврат HostAnswer();").unwrap();

    assert_eq!(module.requirements().len(), 2);
    assert_eq!(module.requirements()[1].package, "example-host");
    assert!(matches!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    ));
}

#[cfg(feature = "binbuf")]
#[test]
fn binbuf_feature_records_and_executes_bit_operations_as_component_calls() {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile("Возврат ПобитовоеИ(12, 10);").unwrap();

    assert_eq!(module.requirements().len(), 2);
    assert_eq!(module.requirements()[1].package, "bsl-binbuf");
    assert_eq!(
        module.requirements()[1].version,
        bsl_binbuf::PACKAGE_VERSION
    );
    assert_eq!(engine.new_state().run(&module).unwrap().to_string(), "8");
    assert!(module.bytecode().unwrap().contains("CallComponent"));
}

#[cfg(not(feature = "binbuf"))]
#[test]
fn missing_binbuf_feature_is_a_compile_error_for_bit_operations() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат ПобитовоеИ(12, 10);").is_err());
}

#[cfg(feature = "textdoc")]
#[test]
fn textdoc_feature_registers_constructor_methods_and_parameters() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "макет = Новый ТекстовыйДокумент;\n\
             макет.ДобавитьСтроку(\"#Область Тело\");\n\
             макет.ДобавитьСтроку(\"[Имя]\");\n\
             макет.ДобавитьСтроку(\"#КонецОбласти\");\n\
             область = макет.ПолучитьОбласть(\"Тело\");\n\
             область.Параметры.Имя = \"мир\";\n\
             результат = Новый ТекстовыйДокумент;\n\
             результат.Вывести(область);\n\
             Возврат результат.ПолучитьТекст();",
        )
        .unwrap();

    assert_eq!(module.requirements().len(), 2);
    assert_eq!(module.requirements()[1].package, "bsl-textdoc");
    assert_eq!(
        module.requirements()[1].version,
        bsl_textdoc::library().version
    );
    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "мир  \n"
    );
    assert!(module.bytecode().unwrap().contains("CreateObject"));
}

#[cfg(not(feature = "textdoc"))]
#[test]
fn missing_textdoc_feature_is_a_compile_error() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ТекстовыйДокумент;").is_err());
}

#[cfg(feature = "json")]
#[test]
fn json_feature_registers_functions_objects_and_module_callbacks() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Функция Преобразовать(Свойство, Значение, Доп, Отказ) Экспорт\n\
             Возврат \"<ok>\";\n\
             КонецФункции\n\
             писатель = Новый ЗаписьJSON;\n\
             писатель.УстановитьСтроку();\n\
             ЗаписатьJSON(писатель, Новый ТаблицаЗначений, , \"Преобразовать\", Истина);\n\
             Возврат писатель.Закрыть();",
        )
        .unwrap();

    assert_eq!(module.requirements().len(), 2);
    assert_eq!(module.requirements()[1].package, "bsl-json");
    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "\"<ok>\""
    );
    let bytecode = module.bytecode().unwrap();
    assert!(bytecode.contains("CreateObject"));
    assert!(bytecode.contains("CallComponent"));
}

#[cfg(not(feature = "json"))]
#[test]
fn missing_json_feature_rejects_functions_and_constructors() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ЧтениеJSON;").is_err());
    assert!(
        engine
            .compile("Возврат ПрочитатьЗначениеJSON(\"1\");")
            .is_err()
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_feature_records_constructors_and_supports_the_bare_manager() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Если ФайловыеПотоки = ФайловыеПотоки Тогда\n\
                 Возврат 1;\n\
             КонецЕсли;\n\
             буфер = Новый БуферДвоичныхДанных(4);\n\
             поток = Новый ПотокВПамяти(буфер);\n\
             писатель = Новый ЗаписьДанных(поток);\n\
             писатель.ЗаписатьБайт(65);\n\
             поток.Перейти(0, ПозицияВПотоке.Начало);\n\
             читатель = Новый ЧтениеДанных(поток);\n\
             Возврат читатель.ПрочитатьБайт();",
        )
        .unwrap();

    assert!(
        module
            .requirements()
            .iter()
            .any(|requirement| requirement.package == "bsl-stream"
                && requirement.version == bsl_stream::PACKAGE_VERSION)
    );
    assert_eq!(engine.new_state().run(&module).unwrap().to_string(), "65");
    assert!(module.bytecode().unwrap().contains("CreateObject"));
}

#[cfg(not(feature = "stream"))]
#[test]
fn missing_stream_feature_rejects_constructors_and_the_bare_manager() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ПотокВПамяти;").is_err());
    assert!(engine.compile("Возврат ФайловыеПотоки;").is_err());
}

#[cfg(feature = "zip")]
#[test]
fn zip_feature_records_constructors_and_reads_written_archives() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("писатель = Новый ЗаписьZipФайла(); Возврат ТипЗнч(писатель);")
        .unwrap();
    assert!(
        module
            .requirements()
            .iter()
            .any(|requirement| requirement.package == "bsl-zip"
                && requirement.version == bsl_zip::PACKAGE_VERSION)
    );
    assert!(module.bytecode().unwrap().contains("CreateObject"));

    // Читатель без источника создаётся закрытым — законная форма.
    let module = engine
        .compile("чтение = Новый ЧтениеZipФайла(); Возврат ТипЗнч(чтение);")
        .unwrap();
    // Представление типа — «Чтение ZIP файла», как на платформе.
    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "Чтение ZIP файла"
    );
}

#[cfg(not(feature = "zip"))]
#[test]
fn missing_zip_feature_rejects_archive_constructors() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ЧтениеZipФайла();").is_err());
    assert!(
        engine
            .compile("Возврат Новый ЗаписьФайлаАрхива();")
            .is_err()
    );
}

#[cfg(feature = "pdf")]
#[test]
fn pdf_feature_records_constructors_and_builds_attachments() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("док = Новый ДокументPDF(); Возврат ТипЗнч(док.Вложения);")
        .unwrap();
    assert!(
        module
            .requirements()
            .iter()
            .any(|requirement| requirement.package == "bsl-pdf"
                && requirement.version == bsl_pdf::PACKAGE_VERSION)
    );
    assert!(module.bytecode().unwrap().contains("CreateObject"));
    // Коллекция вложений есть и до чтения (измерено).
    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "КоллекцияВложенийPDF"
    );
}

#[cfg(not(feature = "pdf"))]
#[test]
fn missing_pdf_feature_rejects_pdf_constructors() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ДокументPDF();").is_err());
    assert!(
        engine
            .compile("Возврат Новый КоллекцияВложенийPDF();")
            .is_err()
    );
}

#[cfg(feature = "xml")]
#[test]
fn xml_feature_records_xdto_constructors_and_functions() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "фаб = Новый ФабрикаXDTO(); сер = Новый СериализаторXDTO(фаб); \
             Возврат ТипЗнч(сер);",
        )
        .unwrap();
    assert!(
        module
            .requirements()
            .iter()
            .any(|requirement| requirement.package == "bsl-xml"
                && requirement.version == bsl_xml::PACKAGE_VERSION)
    );
    assert!(module.bytecode().unwrap().contains("CreateObject"));
    // Представление типа — «Сериализатор XDTO», как на платформе.
    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "Сериализатор XDTO"
    );

    // Глобальная `ФабрикаXDTO` конфигурации — измеренно ловимый отказ.
    let module = engine
        .compile(
            "Попытка\n Ф = ФабрикаXDTO();\n Возврат \"есть\";\n\
             Исключение\n Возврат \"нет\";\nКонецПопытки;",
        )
        .unwrap();
    assert_eq!(engine.new_state().run(&module).unwrap().to_string(), "нет");
}

#[cfg(not(feature = "xml"))]
#[test]
fn missing_xml_feature_rejects_xdto_constructors() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ФабрикаXDTO();").is_err());
    assert!(
        engine
            .compile("сер = Новый СериализаторXDTO(Неопределено);")
            .is_err()
    );
}

#[cfg(feature = "spreadsheet")]
#[test]
fn spreadsheet_feature_records_the_constructor_and_builds_areas() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "Док = Новый ТабличныйДокумент; Обл = Док.Область(1, 1, 1, 1); \
             Обл.Текст = \"привет\"; Возврат Док.Область(1, 1, 1, 1).Текст;",
        )
        .unwrap();
    assert!(
        module
            .requirements()
            .iter()
            .any(|requirement| requirement.package == "bsl-spreadsheet"
                && requirement.version == bsl_spreadsheet::PACKAGE_VERSION)
    );
    assert!(module.bytecode().unwrap().contains("CreateObject"));
    assert_eq!(
        engine.new_state().run(&module).unwrap().to_string(),
        "привет"
    );
}

#[cfg(not(feature = "spreadsheet"))]
#[test]
fn missing_spreadsheet_feature_rejects_the_constructor() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат Новый ТабличныйДокумент;").is_err());
}

#[cfg(feature = "regexp")]
#[test]
fn regexp_feature_registers_functions_and_external_result_objects() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            "р = СтрНайтиПоРегулярномуВыражению(\"абв\", \"(б)\");\n\
             г = р.ПолучитьГруппы();\n\
             Возврат г[0].Значение;",
        )
        .unwrap();

    assert_eq!(module.requirements().len(), 2);
    assert_eq!(module.requirements()[1].package, "bsl-regexp");
    assert_eq!(engine.new_state().run(&module).unwrap().to_string(), "б");
}

#[cfg(not(feature = "regexp"))]
#[test]
fn missing_regexp_feature_is_a_compile_error() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Возврат СтрПодобнаПоРегулярномуВыражению(\"а\", \"а\");")
            .is_err()
    );
}

/// Набор символов условной компиляции по умолчанию — внешнее соединение:
/// open-bsl исполняет BSL из внешней программы и без интерфейса.
#[test]
fn the_default_preprocessor_context_is_an_external_connection() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    let src = "\
#Если Сервер И ВнешнееСоединение И НЕ Клиент Тогда
Возврат \"внешнее соединение\";
#Иначе
Возврат \"что-то другое\";
#КонецЕсли";
    assert_eq!(state.exec(src).unwrap().to_string(), "внешнее соединение");
}

/// Значение символа — свойство контекста развёртывания, а не языка,
/// поэтому хост вправе объявить свой. Оба написания — один символ.
#[test]
fn the_host_may_declare_its_own_preprocessor_context() {
    let engine = Engine::builder()
        .preproc_symbol("Клиент", true)
        .preproc_symbol("Server", false)
        .build()
        .unwrap();
    let mut state = engine.new_state();
    let src = "\
#Если Клиент И НЕ Сервер Тогда
Возврат \"клиент\";
#Иначе
Возврат \"не клиент\";
#КонецЕсли";
    assert_eq!(state.exec(src).unwrap().to_string(), "клиент");
}

/// СОЗНАТЕЛЬНОЕ ОТСТУПЛЕНИЕ от платформы. На 8.3.27 у кода из
/// `Выполнить`/`Вычислить` ложны ВСЕ символы: `#Если Сервер Тогда` там не
/// срабатывает, выбирается `#Иначе` (измерено, см. `docs/bsl-preproc.md`).
/// Здесь фрагмент видит тот же контекст, что и модуль вокруг него, —
/// иначе набор, заданный хостом, до динамического кода бы не дошёл, а
/// платформенное поведение молча выбрасывало бы код без диагностики.
#[test]
fn dynamic_code_sees_the_same_preprocessor_context_as_the_module() {
    let engine = Engine::builder()
        .preproc_symbol("Клиент", true)
        .build()
        .unwrap();
    let mut state = engine.new_state();
    let src = "\
Рез = \"ветка не сработала\";
Выполнить(\"#Если Клиент Тогда
|Рез = \"\"клиент\"\";
|#КонецЕсли\");
Возврат Рез;";
    assert_eq!(state.exec(src).unwrap().to_string(), "клиент");
}
