use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DataAccess, DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind,
    MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslDate, BslObject, CallContext, RtResult, Value, format_value};
use std::sync::Arc;
use std::{cell::Cell, rc::Rc};

fn factory(source: &str) -> SessionFactory {
    typed_factory(source, DocumentNumberType::String)
}

fn typed_factory(source: &str, number_type: DocumentNumberType) -> SessionFactory {
    let module = "Документ.ЗамерДокумента.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("document-new-number", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            source,
            ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("doc"),
    ]).unwrap();
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_object_module(module)
        .with_document_number_settings(DocumentNumberSettings {
            number_type,
            number_length: 9,
        })
        .with_document_attributes(vec![]);
    let catalog = MetadataCatalog::new("document-new-number", vec![document]).unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

struct AllocationSpy {
    calls: Cell<usize>,
}

impl DataAccess for AllocationSpy {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("нумерация не читает константы");
    }
    fn allocate_document_number(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        date: BslDate,
        prefix: &str,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        assert_eq!(metadata.id(), "doc");
        assert_eq!(principal.id(), "user");
        assert!(date.is_empty());
        assert_eq!(prefix, "H-");
        self.calls.set(self.calls.get() + 1);
        Ok(Value::Str("H-0000001".into()))
    }
}

#[test]
fn native_number_handler_cancels_or_dispatches_the_final_prefix_in_each_session() {
    let factory = factory(include_str!(
        "../../../tests/conformance/measure/1c/document-new-number-handler-module.bsl"
    ));
    for _ in 0..2 {
        let adapter = Rc::new(AllocationSpy {
            calls: Cell::new(0),
        });
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_data_access(adapter.clone()),
            )
            .unwrap();
        let script = include_str!(
            "../../../tests/conformance/measure/1c/measure-document-new-number-handler.bsl"
        )
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
        let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{script}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
        let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
        else {
            panic!("массив результатов");
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов");
        };
        let output = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(
            output,
            include_str!(
                "../../../tests/conformance/measure/1c/measure-document-new-number-handler.platform.txt"
            )
        );
        assert_eq!(adapter.calls.get(), 1, "отмена не должна вызвать адаптер");
    }
}

#[test]
fn native_number_body_rejection_precedes_missing_session_adapter() {
    let factory = factory(include_str!(
        "../../../tests/conformance/measure/1c/document-new-number-body-module.bsl"
    ));
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); Возврат ОбъектПробы.ПрочитатьНомерТела(); КонецФункции Функция Выдать() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); ОбъектПробы.SetNewNumber(); КонецФункции").unwrap();
    assert_eq!(
        format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap(),
        "rejected"
    );
    assert!(
        session
            .call(&module, "Выдать", vec![])
            .unwrap_err()
            .to_string()
            .contains("адаптер данных текущего сеанса не подключён")
    );
}

struct UnsupportedAllocation;

impl DataAccess for UnsupportedAllocation {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("выдача номера не читает константы");
    }
}

#[test]
fn an_existing_adapter_rejects_allocation_without_assigning_a_fabricated_number() {
    let factory = factory("");
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(Rc::new(UnsupportedAllocation)),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); ОбъектПробы.Номер = \"old\"; Попытка ОбъектПробы.УстановитьНовыйНомер(); Исключение Возврат ОбъектПробы.Номер; КонецПопытки; Возврат \"unexpected\"; КонецФункции Функция Выдать() Экспорт ОбъектПробы = Документы.ЗамерДокумента.СоздатьДокумент(); ОбъектПробы.УстановитьНовыйНомер(); КонецФункции").unwrap();
    assert_eq!(
        format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap(),
        "old"
    );
    assert!(
        session
            .call(&module, "Выдать", vec![])
            .unwrap_err()
            .to_string()
            .contains("выдача номера документа не поддерживается адаптером данных")
    );
}

#[test]
fn native_numeric_number_handler_uses_its_single_parameter_without_an_adapter() {
    let factory = typed_factory(
        include_str!(
            "../../../tests/conformance/measure/1c/document-new-number-numeric-handler-module.bsl"
        ),
        DocumentNumberType::Number,
    );
    for _ in 0..2 {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let script = include_str!(
            "../../../tests/conformance/measure/1c/measure-document-new-number-numeric-handler.bsl"
        )
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
        let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{script}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
        let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
        else {
            panic!("массив результатов");
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов");
        };
        let output = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(
            output,
            include_str!(
                "../../../tests/conformance/measure/1c/measure-document-new-number-numeric-handler.platform.txt"
            )
        );
    }
}

#[test]
fn native_string_number_handler_uses_its_single_parameter_without_an_adapter() {
    let factory = typed_factory(
        include_str!(
            "../../../tests/conformance/measure/1c/document-new-number-numeric-handler-module.bsl"
        ),
        DocumentNumberType::String,
    );
    for _ in 0..2 {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let script = include_str!(
            "../../../tests/conformance/measure/1c/measure-document-new-number-string-handler.bsl"
        )
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
        let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{script}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
        let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
        else {
            panic!("массив результатов");
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов");
        };
        let output = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(
            output,
            include_str!(
                "../../../tests/conformance/measure/1c/measure-document-new-number-string-handler.platform.txt"
            )
        );
    }
}
