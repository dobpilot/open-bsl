use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtResult, Value};
use std::{rc::Rc, sync::Arc};

#[path = "data_separation_frames/conversion.rs"]
mod conversion;

#[path = "data_separation_frames/isolation.rs"]
mod isolation;

struct ModeObserver;

impl DataAccess for ModeObserver {
    fn constant_value(
        &self,
        metadata: &MetadataObject,
        _: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let name = match metadata.id() {
            "observer" => "НаименованиеЯзык1",
            "observer-other" => "ДругоеРазделение",
            _ => panic!("неизвестный наблюдатель"),
        };
        context.data_separation_safe_mode(name).map(Value::Boolean)
    }
}

fn factory() -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "data-separation-frames",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "data-separation-frames",
        vec![
            MetadataObject::new("observer", "НаблюдениеРежима", MetadataKind::Constant),
            MetadataObject::new(
                "observer-other",
                "НаблюдениеДругогоРежима",
                MetadataKind::Constant,
            ),
        ],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn collect(session: &mut Session) -> String {
    let collector = session
        .instantiate(
            r#"Функция Собрать() Экспорт
        ВыводПробы = "";
        Для Каждого СообщениеПробы Из ПолучитьСообщенияПользователю(Истина) Цикл
            ВыводПробы = ВыводПробы + СообщениеПробы.Текст + Символы.ПС;
        КонецЦикла;
        Возврат ВыводПробы;
    КонецФункции"#,
        )
        .unwrap();
    let Value::Str(actual) = session.call(&collector, "Собрать", vec![]).unwrap().0 else {
        panic!("ожидалась строка результатов")
    };
    actual.to_string()
}

#[test]
fn data_separation_frame_counters_match_the_native_control_through_host_observation() {
    let factory = factory();
    // Наблюдаем только счётчик через публичную возможность хоста. Нативный
    // геттер зоны не подменяется в реализации и продолжает явно отказывать.
    let source =
        include_str!("../../../../docs/research/fixtures/data-separation-frames-1017/measure.bsl")
            .replace(
                "БезопасныйРежимРазделенияДанных(\"НаименованиеЯзык1\")",
                "Константы.НаблюдениеРежима.Получить()",
            );
    for number in [1, 2] {
        let mut session = factory
            .open(
                Principal::new("probe", "", vec![]),
                SessionResources::new(Box::new(()), number)
                    .with_data_access(Rc::new(ModeObserver))
                    .with_data_separation_common_attributes(["НаименованиеЯзык1".to_string()]),
            )
            .unwrap();
        session.execute(&source).unwrap();
        let actual = collect(&mut session);
        // Преобразование числового и строкового флага — отдельная проверка.
        // Здесь сравниваются все остальные строки без изменения значений.
        let frame_rows = |text: &str| {
            text.lines()
                .filter(|line| !line.starts_with("NUMBER=") && !line.starts_with("STRING="))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            frame_rows(&actual),
            frame_rows(include_str!(
                "../../../../docs/research/fixtures/data-separation-frames-1017/measure.platform.txt"
            ))
        );
    }
}

#[test]
fn two_names_with_shared_parameters_have_independent_folded_counters() {
    let factory = factory();
    let mut session = factory
        .open(
            Principal::new("probe", "", vec![]),
            SessionResources::new(Box::new(()), 1)
                .with_data_access(Rc::new(ModeObserver))
                .with_data_separation_common_attributes([
                    "НаименованиеЯзык1".to_string(),
                    "ДругоеРазделение".to_string(),
                ]),
        )
        .unwrap();
    let mut source = include_str!(
        "../../../../docs/research/fixtures/data-separation-frames-1017/measure-names.bsl"
    )
    .to_string();
    for (name, observer) in [
        ("НаименованиеЯзык1", "НаблюдениеРежима"),
        ("наименованиеязык1", "НаблюдениеРежима"),
        ("ДругоеРазделение", "НаблюдениеДругогоРежима"),
        ("другоеразделение", "НаблюдениеДругогоРежима"),
    ] {
        source = source.replace(
            &format!("БезопасныйРежимРазделенияДанных(\"{name}\")"),
            &format!("Константы.{observer}.Получить()"),
        );
    }
    session.execute(&source).unwrap();
    assert_eq!(
        collect(&mut session).trim(),
        include_str!(
            "../../../../docs/research/fixtures/data-separation-frames-1017/measure-names.platform.txt"
        )
        .trim()
    );
}
