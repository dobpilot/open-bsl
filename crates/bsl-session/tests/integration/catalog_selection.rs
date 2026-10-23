use std::{rc::Rc, sync::Arc};

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, DataAccess,
    MetadataCatalog, MetadataKind, MetadataObject, Principal, QueryRequest, QueryResultData,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session_with_source(hierarchical: Option<bool>, source: &str) -> Session {
    session_with_adapter(hierarchical, source, |_: Arc<MetadataCatalog>| {
        Rc::new(EmptyAdapter)
    })
}

fn session_with_adapter(
    hierarchical: Option<bool>,
    source: &str,
    adapter: impl FnOnce(Arc<MetadataCatalog>) -> Rc<dyn DataAccess>,
) -> Session {
    let module = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let settings = CatalogStandardAttributes {
        code_type: CatalogCodeType::String,
        code_length: 9,
        description_length: 25,
    };
    let mut object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_object_module(module)
        .with_catalog_standard_attributes(settings)
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if let Some(hierarchical) = hierarchical {
        object = object.with_catalog_hierarchical(hierarchical);
    }
    let catalog = MetadataCatalog::new(
        "deletion-mark",
        vec![
            object,
            MetadataObject::new("other", "ЗамерЧужойСсылки", MetadataKind::Catalog)
                .with_catalog_standard_attributes(settings)
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new("deletion-mark", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            source,
            ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("catalog"),
    ]).unwrap();
    let catalog = Arc::new(catalog);
    let adapter = adapter(Arc::clone(&catalog));
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
    .open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1).with_data_access(adapter),
    )
    .unwrap()
}

fn replay(session: &mut Session, source: &str) -> String {
    let source = source
        .rsplit_once("Исключение\n    Сообщить(\"selection.error=")
        .unwrap()
        .0;
    let source = source.strip_prefix("Попытка\n").unwrap();
    replay_source(session, source)
}

fn replay_source(session: &mut Session, source: &str) -> String {
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {source}\n Возврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

struct EmptyAdapter;
impl DataAccess for EmptyAdapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut open_bsl::CallContext<'_>,
    ) -> open_bsl::RtResult<Value> {
        panic!("выборка не читает константу");
    }
    fn execute_query(
        &self,
        _: &QueryRequest,
        principal: &Principal,
        _: &mut open_bsl::CallContext<'_>,
    ) -> open_bsl::RtResult<QueryResultData> {
        assert_eq!(principal.name(), "Тест");
        QueryResultData::new(
            [
                "Ссылка".into(),
                "Код".into(),
                "Наименование".into(),
                "ПометкаУдаления".into(),
            ],
            Vec::<Vec<Value>>::new(),
        )
        .map_err(|error| open_bsl::RtError::DynamicError(error.to_string()))
    }
}

#[test]
fn empty_catalog_selection_matches_native_oracle() {
    assert_eq!(
        replay(
            &mut session_with_source(Some(false), ""),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-catalog-selection-empty.bsl"
            )
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-selection-empty.platform.txt"
        )
        .trim()
    );
}

struct PopulatedAdapter {
    catalog: Arc<MetadataCatalog>,
    fail: bool,
}
impl DataAccess for PopulatedAdapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut open_bsl::CallContext<'_>,
    ) -> open_bsl::RtResult<Value> {
        panic!("выборка не читает константу")
    }
    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut open_bsl::CallContext<'_>,
    ) -> open_bsl::RtResult<QueryResultData> {
        assert_eq!(principal.name(), "Тест");
        if self.fail {
            return Err(open_bsl::RtError::DynamicError(
                "ошибка чтения выборки".into(),
            ));
        }
        assert!(
            request
                .text()
                .contains("ИЗ Справочник.ЗамерВременныхТаблиц КАК Источник")
        );
        let mut rows = vec![
            vec![
                bsl_orm::catalog_reference_value(Arc::clone(&self.catalog), "catalog", [1; 16])?,
                Value::Str("002".into()),
                Value::Str("Alpha".into()),
                Value::Boolean(false),
            ],
            vec![
                bsl_orm::catalog_reference_value(Arc::clone(&self.catalog), "catalog", [2; 16])?,
                Value::Str("001".into()),
                Value::Str("Zulu".into()),
                Value::Boolean(true),
            ],
        ];
        if request.text().contains("ГДЕ Источник.Код = &Отбор") {
            assert!(
                request
                    .parameters()
                    .any(|(name, value)| name == "Отбор" && value == &Value::Str("002".into()))
            );
            rows.truncate(1);
        } else if request.text().ends_with("Источник.Наименование УБЫВ") {
            rows.reverse();
        } else {
            assert!(request.text().ends_with("Источник.Наименование"));
        }
        QueryResultData::new(
            [
                "Ссылка".into(),
                "Код".into(),
                "Наименование".into(),
                "ПометкаУдаления".into(),
            ],
            rows,
        )
        .map_err(|error| open_bsl::RtError::DynamicError(error.to_string()))
    }
}

#[test]
fn populated_catalog_selection_matches_native_oracle() {
    let source = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-selection-populated.bsl"
    )
    .split_once("// Начало проверяемого контракта выборки.\n")
    .unwrap()
    .1;
    let mut session = session_with_adapter(Some(false), "", |catalog| {
        Rc::new(PopulatedAdapter {
            catalog,
            fail: false,
        })
    });
    assert_eq!(
        replay(&mut session, source),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-selection-populated.platform.txt"
        )
        .trim()
    );
}

#[test]
fn catalog_selection_propagates_adapter_failure() {
    let mut session = session_with_adapter(Some(false), "", |catalog| {
        Rc::new(PopulatedAdapter {
            catalog,
            fail: true,
        })
    });
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.Выбрать(); КонецФункции").unwrap();
    assert!(
        session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains("ошибка чтения выборки")
    );
}

fn evaluate(session: &mut Session, body: &str) -> Value {
    let module = session
        .instantiate(&format!("Функция Проверить() Экспорт {body} КонецФункции"))
        .unwrap();
    session.call(&module, "Проверить", vec![]).unwrap().0
}

#[test]
fn selection_cursors_and_session_adapters_are_independent() {
    let mut populated = session_with_adapter(Some(false), "", |catalog| {
        Rc::new(PopulatedAdapter {
            catalog,
            fail: false,
        })
    });
    assert_eq!(
        evaluate(
            &mut populated,
            r#"
            Первая = Справочники.ЗамерВременныхТаблиц.Выбрать();
            Вторая = Справочники.ЗамерВременныхТаблиц.Select();
            Первая.Следующий(); Первая.Следующий(); Вторая.Next();
            Возврат Первая.Наименование = "Zulu" И Вторая.Description = "Alpha";
        "#
        ),
        Value::Boolean(true)
    );
    let mut empty = session_with_source(Some(false), "");
    assert_eq!(
        evaluate(
            &mut empty,
            "Возврат Справочники.ЗамерВременныхТаблиц.Выбрать().Следующий();"
        ),
        Value::Boolean(false)
    );
    assert_eq!(
        evaluate(
            &mut populated,
            "Возврат Справочники.ЗамерВременныхТаблиц.Выбрать().Следующий();"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn applicable_selection_arguments_match_native_oracle() {
    let source = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-selection-arguments.bsl"
    );
    // Подчинённость ещё не приобретена: случаи владельца остаются открытыми.
    let source = source
        .split("\n\n")
        .filter(|block| !block.contains("selection.argument.owner."))
        .collect::<Vec<_>>()
        .join("\n");
    let expected = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-selection-arguments.platform.txt"
    )
    .lines()
    .filter(|line| !line.contains("selection.argument.owner."))
    .collect::<Vec<_>>()
    .join("\n");
    assert_eq!(
        replay_source(&mut session_with_source(Some(false), ""), &source),
        expected
    );
}

struct ParentAdapter {
    catalog: Arc<MetadataCatalog>,
}
impl DataAccess for ParentAdapter {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut open_bsl::CallContext<'_>,
    ) -> open_bsl::RtResult<Value> {
        panic!("выборка не читает константу")
    }
    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut open_bsl::CallContext<'_>,
    ) -> open_bsl::RtResult<QueryResultData> {
        assert_eq!(principal.name(), "Тест");
        let parent = request
            .parameters()
            .find(|(name, _)| *name == "Родитель")
            .map(|(_, value)| bsl_orm::catalog_reference_parts(value).unwrap().1);
        let names: &[&str] = match parent {
            None => &["Child", "Group", "Root"],
            Some(uuid) if uuid == [0; 16] => &["Group", "Root"],
            Some(uuid) => {
                assert_eq!(
                    uuid,
                    [
                        0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x43, 0x21, 0xab, 0xcd, 0x01, 0x23,
                        0x45, 0x67, 0x89, 0xab
                    ]
                );
                &["Child"]
            }
        };
        if parent.is_some() {
            assert!(request.text().contains("ГДЕ Источник.Родитель = &Родитель"));
        }
        let rows = names.iter().map(|name| {
            vec![
                bsl_orm::catalog_reference_value(Arc::clone(&self.catalog), "catalog", [1; 16])
                    .unwrap(),
                Value::Str("001".into()),
                Value::Str((*name).into()),
                Value::Boolean(false),
            ]
        });
        QueryResultData::new(
            [
                "Ссылка".into(),
                "Код".into(),
                "Наименование".into(),
                "ПометкаУдаления".into(),
            ],
            rows,
        )
        .map_err(|error| open_bsl::RtError::DynamicError(error.to_string()))
    }
}

#[test]
fn hierarchical_selection_parent_matches_native_oracle() {
    let source = include_str!(
        "../../../../tests/conformance/measure/1c/measure-catalog-selection-parent.bsl"
    )
    .split_once("// Начало проверяемого контракта выборки.\n")
    .unwrap()
    .1;
    let mut session =
        session_with_adapter(Some(true), "", |catalog| Rc::new(ParentAdapter { catalog }));
    assert_eq!(
        replay_source(&mut session, source),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-catalog-selection-parent.platform.txt"
        )
        .trim()
    );
}
