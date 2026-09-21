use super::*;
use std::rc::Rc;

fn catalog() -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("role-user", "Пользователь", MetadataKind::Role),
                MetadataObject::new("role-admin", "Администратор", MetadataKind::Role),
                MetadataObject::new("catalog-versions", "Версии", MetadataKind::Catalog)
                    .with_manager_module("ВерсииМенеджер"),
                MetadataObject::new(
                    "scheduled-job-cleanup",
                    "ОчисткаДанных",
                    MetadataKind::ScheduledJob,
                )
                .with_string_property("ИмяМетода", "Обслуживание.ОчиститьДанные"),
            ],
        )
        .unwrap()
        .with_configuration(
            "Демонстрационная",
            "1.2.3",
            ScriptVariant::Russian,
            ["role-user".into()],
        )
        .unwrap(),
    )
}

fn engine() -> open_bsl::Engine {
    open_bsl::Engine::builder()
        .register_library(library())
        .common_module(
            "ВерсииМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Версии" Тогда
                    ВызватьИсключение "неверный менеджер";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .build()
        .unwrap()
}

fn state(engine: &open_bsl::Engine) -> open_bsl::State {
    engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog(),
            Principal::new("user", "Тест", vec!["role-user".into()]),
        )))
        .build()
}

#[test]
fn role_available_accepts_a_name_and_a_metadata_role() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec("Возврат РольДоступна(\"пОлЬзОвАтЕлЬ\");")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат IsInRole(Метаданные.Roles.Администратор);")
            .unwrap(),
        Value::Boolean(false)
    );
    assert!(
        state
            .exec("Возврат РольДоступна(\"НетТакойРоли\");")
            .unwrap_err()
            .to_string()
            .contains("роль конфигурации не найдена")
    );
}

#[test]
fn configuration_metadata_exposes_name_version_language_and_main_roles() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec("Возврат Метаданные.Имя + \" \" + Метаданные.Версия;")
            .unwrap(),
        Value::Str("Демонстрационная 1.2.3".into())
    );
    assert_eq!(
        state
            .exec(
                "Возврат Метаданные.ВариантВстроенногоЯзыка = ВариантВстроенногоЯзыка.Русский И Метаданные.ВариантВстроенногоЯзыка = Метаданные.СвойстваОбъектов.ВариантВстроенногоЯзыка[\"Russian\"] ;"
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.ОсновныеРоли.Содержит(Метаданные.Роли.Пользователь);")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.DefaultRoles.Contains(Метаданные.Roles.Администратор);")
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state
            .exec(
                r#"
                Имена = "";
                Для Каждого Задание Из Метаданные.РегламентныеЗадания Цикл
                    Имена = Имена + Задание.Имя + ":" + Задание.ИмяМетода;
                КонецЦикла;
                Возврат Имена;
                "#,
            )
            .unwrap(),
        Value::Str("ОчисткаДанных:Обслуживание.ОчиститьДанные".into())
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.Роли[0] = Метаданные.Роли.Пользователь;")
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn missing_configuration_properties_and_invalid_main_roles_fail_explicitly() {
    let missing = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![MetadataObject::new(
                "role-user",
                "Пользователь",
                MetadataKind::Role,
            )],
        )
        .unwrap(),
    );
    let engine = engine();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            missing,
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build();
    assert!(
        state
            .exec("Возврат Метаданные.Имя;")
            .unwrap_err()
            .to_string()
            .contains("свойства конфигурации отсутствуют")
    );

    let error = MetadataCatalog::new(
        "configuration",
        vec![MetadataObject::new(
            "role-user",
            "Пользователь",
            MetadataKind::Role,
        )],
    )
    .unwrap()
    .with_configuration(
        "Конфигурация",
        "1",
        ScriptVariant::Russian,
        ["missing".into()],
    )
    .unwrap_err();
    assert_eq!(error, CatalogError::UnknownMainRole("missing".into()));
}

#[test]
fn manager_export_runs_in_the_session_module_instance() {
    let engine = engine();
    let entry = engine
        .compile(
            r#"
            Функция Проверка() Экспорт
                Возврат Справочники.Версии.Проверить(10);
            КонецФункции
            "#,
        )
        .unwrap();
    let mut state = state(&engine);
    let instance = state.instantiate(&entry).unwrap();
    assert_eq!(
        instance.call(&mut state, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(11)
    );
    assert_eq!(
        instance.call(&mut state, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(12)
    );
}

#[test]
fn unknown_manager_member_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);
    assert!(
        state
            .exec("Возврат Константы.НетТакойКонстанты;")
            .unwrap_err()
            .to_string()
            .contains("НетТакойКонстанты")
    );
}
