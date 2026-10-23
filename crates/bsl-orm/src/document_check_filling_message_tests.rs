use super::*;
use open_bsl::{Engine, HostError, UserMessageDto, UserMessageSink};

#[derive(Debug, Default)]
struct Messages(RefCell<Vec<UserMessageDto>>);
impl UserMessageSink for Messages {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        self.0.borrow_mut().push(message.clone());
        Ok(())
    }
}

#[test]
fn document_message_fields_and_text_match_both_native_language_controls() {
    for (language, native) in [
        (
            "en",
            include_str!(
                "../../../docs/research/fixtures/document-check-filling/measure-messages.platform.txt"
            ),
        ),
        (
            "ru",
            include_str!(
                "../../../docs/research/fixtures/document-check-filling/measure-messages-ru.platform.txt"
            ),
        ),
    ] {
        let document = MetadataObject::new("doc", "Проба", MetadataKind::Document)
            .with_document_number_settings(DocumentNumberSettings {
                number_type: DocumentNumberType::String,
                number_length: 9,
            })
            .with_document_standard_fill_checking(vec![CatalogStandardAttributeFillChecking {
                name: "Date".into(),
                fill_checking: FillChecking::ShowError,
            }])
            .with_document_attributes(vec![])
            .with_document_tabular_sections(vec![
                DocumentTabularSection::new("section", "Запасы")
                    .with_fill_checking(FillChecking::ShowError)
                    .with_attributes(vec![
                        DocumentAttribute::new(
                            "column",
                            "НомерВариантаКП",
                            vec![open_bsl::ColumnType::plain(TypeRef::Native(
                                open_bsl::TypeId::Number,
                            ))],
                        )
                        .with_fill_checking(FillChecking::ShowError),
                    ]),
            ]);
        let catalog = Arc::new(MetadataCatalog::new("messages", vec![document]).unwrap());
        let mut builder = Engine::builder().register_library(library());
        for ty in catalog.transferable_value_types() {
            builder = builder.register_value_type(ty);
        }
        let engine = builder.build().unwrap();
        let messages = Rc::new(Messages::default());
        let mut state = engine
            .state_builder()
            .host_data(Rc::new(
                SessionData::new(catalog, Principal::new("user", "Тест", vec![]))
                    .with_system_language(language),
            ))
            .message_sink(messages.clone())
            .build();
        assert_eq!(
            state
                .exec("Возврат Документы.Проба.СоздатьДокумент().ПроверитьЗаполнение();")
                .unwrap(),
            Value::Boolean(false)
        );
        assert_eq!(state.exec("ОбъектПробы = Документы.Проба.СоздатьДокумент(); ОбъектПробы.Дата = Дата(2026, 10, 11); ОбъектПробы.Запасы.Добавить(); Возврат ОбъектПробы.ПроверитьЗаполнение();").unwrap(), Value::Boolean(false));
        let actual = messages
            .0
            .borrow()
            .iter()
            .map(|m| format!("{}|{}", m.field, m.text))
            .collect::<Vec<_>>();
        let expected = native
            .lines()
            .filter(|line| line.contains(".item\t"))
            .map(|line| line.split_once('\t').unwrap().1.to_owned())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{language}");
    }
}
