use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, configuration_reference_value};
use std::sync::Arc;

#[test]
fn chart_reference_preserves_acquired_owner_type_and_record_identity() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "chart-control",
            vec![
                MetadataObject::new("chart", "ЗамерСчетов", MetadataKind::ChartOfAccounts),
                MetadataObject::new("other", "ДругойПлан", MetadataKind::ChartOfAccounts),
                MetadataObject::new("constant", "Константа", MetadataKind::Constant),
            ],
        )
        .unwrap(),
    );
    let ty = catalog.value_types().next().unwrap();
    assert_eq!(
        ty.names(),
        [
            "ПланСчетовСсылка.ЗамерСчетов",
            "ChartOfAccountsRef.ЗамерСчетов"
        ]
    );
    let uuid = [1; 16];
    let first = configuration_reference_value(Arc::clone(&catalog), "chart", uuid).unwrap();
    let same = configuration_reference_value(Arc::clone(&catalog), "chart", uuid).unwrap();
    let different_owner =
        configuration_reference_value(Arc::clone(&catalog), "other", uuid).unwrap();
    let different_record =
        configuration_reference_value(Arc::clone(&catalog), "chart", [2; 16]).unwrap();
    assert_eq!(first, same);
    assert_ne!(first, different_owner);
    assert_ne!(first, different_record);
    assert!(configuration_reference_value(Arc::clone(&catalog), "constant", uuid).is_err());
    assert!(configuration_reference_value(catalog, "missing", uuid).is_err());
}

use bsl_orm::{DataAccess, Principal, QueryRequest, QueryResultData, SessionData, library};
use open_bsl::{CallContext, Engine, RtError, RtResult, Value};
use std::{cell::Cell, rc::Rc};

#[derive(Debug)]
struct ChartAccess {
    catalog: Arc<MetadataCatalog>,
    calls: Cell<usize>,
}

impl DataAccess for ChartAccess {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "константа не используется в контроле".into(),
        ))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "chart-user");
        assert_eq!(
            request.text(),
            "ВЫБРАТЬ ЗНАЧЕНИЕ(ПланСчетов.ЗамерСчетов.Основной) КАК Ссылка"
        );
        self.calls.set(self.calls.get() + 1);
        QueryResultData::new(
            ["Ссылка".into()],
            [vec![configuration_reference_value(
                Arc::clone(&self.catalog),
                "chart",
                [3; 16],
            )?]],
        )
        .map_err(|error| RtError::DynamicError(error.to_string()))
    }
}

#[test]
fn predefined_chart_property_uses_current_session_adapter_and_is_read_only() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "chart-control",
            vec![
                MetadataObject::new("chart", "ЗамерСчетов", MetadataKind::ChartOfAccounts)
                    .with_catalog_predefined_names(["Основной"])
                    .with_manager_module("МенеджерСчетов"),
                MetadataObject::new(
                    "shadow-chart",
                    "ПеременныеМенеджера",
                    MetadataKind::ChartOfAccounts,
                )
                .with_catalog_predefined_names(["Основной"])
                .with_manager_module("ПеременныеМенеджера"),
            ],
        )
        .unwrap(),
    );
    let access = Rc::new(ChartAccess {
        catalog: Arc::clone(&catalog),
        calls: Cell::new(0),
    });
    let mut builder = Engine::builder().register_library(library())
        .configuration(open_bsl::ModuleGraphRecipe {
            eager_init: false,
            modules: vec![open_bsl::ModuleRecipe {
                name: "МенеджерСчетов".into(),
                visibility: open_bsl::ModuleVisibility::Private,
                imports: vec![],
                source: "Функция Читать() Экспорт Возврат Основной; КонецФункции Функция МакетБезИмени() Экспорт Возврат ПолучитьМакет(); КонецФункции Функция Пустой() Экспорт Возврат ПустаяСсылка(); КонецФункции Функция Локальный() Экспорт Перем Основной; Основной = 7; Возврат Основной; КонецФункции".into(),
            }, open_bsl::ModuleRecipe {
                name: "ПеременныеМенеджера".into(),
                visibility: open_bsl::ModuleVisibility::Named,
                imports: vec![],
                source: "Перем Основной; Функция Читать() Экспорт Основной = 9; Возврат Основной; КонецФункции Функция GetTemplate(Первый, Второй) Экспорт Возврат Первый + Второй; КонецФункции".into(),
            }],
        })
        .configuration_component_module("ПланыСчетов", "ЗамерСчетов", "МенеджерСчетов")
        .configuration_method_context("МенеджерСчетов", MetadataKind::ChartOfAccounts.receiver_methods())
        .configuration_property_names("МенеджерСчетов", ["Основной"])
        .configuration_property_names("ПеременныеМенеджера", ["Основной"])
        .configuration_component_module("ПланыСчетов", "ПеременныеМенеджера", "ПеременныеМенеджера");
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog, Principal::new("chart-user", "Тест", vec![]))
                .with_data_access(access.clone()),
        ))
        .build();
    assert_eq!(
        state
            .exec(
                "Возврат ПланыСчетов.ЗамерСчетов.Основной = ChartsOfAccounts.ЗамерСчетов.оСнОвНоЙ;"
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(access.calls.get(), 2);
    assert_eq!(
        state
            .exec("Возврат ПланыСчетов.ЗамерСчетов.Читать() = ПланыСчетов.ЗамерСчетов.Основной;")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат ПланыСчетов.ЗамерСчетов.Локальный();")
            .unwrap(),
        Value::number_from_i64(7)
    );
    assert_eq!(
        state
            .exec("Возврат ПланыСчетов.ПеременныеМенеджера.Читать();")
            .unwrap(),
        Value::number_from_i64(9)
    );
    assert_eq!(
        state
            .exec("Возврат ПланыСчетов.ПеременныеМенеджера.GetTemplate(4, 5);")
            .unwrap(),
        Value::number_from_i64(9)
    );
    assert!(engine.compile("Возврат Основной;").is_err());
    assert_eq!(access.calls.get(), 4);
    assert_eq!(state.exec("Возврат ПланыСчетов.ЗамерСчетов.Пустой() = ChartsOfAccounts.ЗамерСчетов.EmptyRef();").unwrap(), Value::Boolean(true));
    assert_eq!(
        state
            .exec("Возврат ПланыСчетов.ЗамерСчетов.ПустаяСсылка().IsEmpty();")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Строка(ПланыСчетов.ЗамерСчетов.EmptyRef().UUID());")
            .unwrap(),
        Value::Str("00000000-0000-0000-0000-000000000000".into())
    );
    assert_eq!(state.exec(r#"Возврат ТипЗнч(ПланыСчетов.ЗамерСчетов.EmptyRef()) = Тип("ПланСчетовСсылка.ЗамерСчетов") И TypeOf(ПланыСчетов.ЗамерСчетов.ПустаяСсылка()) = Тип("ChartOfAccountsRef.ЗамерСчетов");"#).unwrap(), Value::Boolean(true));
    assert_eq!(access.calls.get(), 4);
    assert_eq!(
        state
            .exec("Возврат ПланыСчетов.ЗамерСчетов.МакетБезИмени();")
            .unwrap(),
        Value::Undefined
    );
    assert_eq!(
        state
            .exec("Возврат ChartsOfAccounts.ЗамерСчетов.gEtTeMpLaTe();")
            .unwrap(),
        Value::Undefined
    );
    let template_error = state
        .exec(r#"Возврат ПланыСчетов.ЗамерСчетов.ПолучитьМакет("Контроль");"#)
        .unwrap_err();
    assert!(
        template_error
            .to_string()
            .contains("ПланСчетов.ПолучитьМакет: получение макета пока не поддерживается")
    );
    assert!(
        state
            .exec(r#"Возврат ПланыСчетов.ЗамерСчетов.GetTemplate("Первый", "Второй");"#)
            .is_err()
    );
    let error = state
        .exec("ПланыСчетов.ЗамерСчетов.Основной = Неопределено;")
        .unwrap_err();
    assert!(matches!(
        error,
        open_bsl::Error::Runtime(RtError::PropertyReadOnly { .. })
    ));
    assert_eq!(access.calls.get(), 4);
}
