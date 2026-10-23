use super::*;
use open_bsl::{ColumnType, Engine, HostError, TypeId, UserMessageDto, UserMessageSink};

#[derive(Debug, Default)]
struct Messages(RefCell<Vec<UserMessageDto>>);

impl UserMessageSink for Messages {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        self.0.borrow_mut().push(message.clone());
        Ok(())
    }
}

#[test]
fn business_check_message_matches_the_native_text_and_attribute() {
    let catalog = MetadataCatalog::new(
        "business-message",
        vec![
            MetadataObject::new(
                "process",
                "ЗамерБизнесПроцесса",
                MetadataKind::BusinessProcess,
            ),
            MetadataObject::new("ru", "Русский", MetadataKind::Language)
                .with_string_property("КодЯзыка", "ru"),
        ],
    )
    .unwrap()
    .with_business_process_metadata(
        "process",
        Some(BusinessProcessNumberSettings {
            number_type: DocumentNumberType::String,
            length: 11,
            variable_length: true,
        }),
        Some(vec![
            DocumentAttribute::new(
                "text",
                "ТекстПробы",
                vec![ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                }],
            )
            .with_fill_from_filling_value(false)
            .with_fill_checking(FillChecking::ShowError)
            .with_synonym("ru", "Текст пробы"),
        ]),
        Some(vec![]),
    )
    .unwrap()
    .with_business_process_standard_fill_checking("process", vec![])
    .unwrap()
    .with_configuration(
        "Проба",
        "1.0",
        ScriptVariant::Russian,
        CompatibilityMode::DontUse,
        Vec::<String>::new(),
    )
    .unwrap()
    .with_primary_language("ru")
    .unwrap();
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let messages = Rc::new(Messages::default());
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(Arc::new(catalog), Principal::new("user", "Тест", vec![]))
                .with_system_language("en"),
        ))
        .message_sink(messages.clone())
        .build();
    let result = state
        .exec("Возврат БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс().ПроверитьЗаполнение();")
        .unwrap();
    assert_eq!(result, Value::Boolean(false));
    let messages = messages.0.borrow();
    assert_eq!(messages.len(), 1);
    let actual = format!(
        "business.check.message.result\t0\nbusiness.check.message.count\t{}\nbusiness.check.message.text\t{}\nbusiness.check.message.field\t{}\n",
        messages.len(),
        messages[0].text,
        messages[0].field
    );
    assert_eq!(
        actual,
        include_str!(
            "../../../docs/research/fixtures/business-process-object/check-filling-message.platform.txt"
        )
    );
}
