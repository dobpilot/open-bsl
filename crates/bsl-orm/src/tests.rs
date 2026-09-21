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
            ],
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
