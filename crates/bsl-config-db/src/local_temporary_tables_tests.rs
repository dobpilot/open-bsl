//! Материализация временных таблиц на собственном PostgreSQL.

use super::*;

#[test]
fn temporary_tables_and_parameter_snapshots_execute_on_postgres() {
    let Some((_, mut state, _)) = super::session_transactions_tests::control() else {
        // Без собственного стенда полный workspace-прогон SQL не подтверждает.
        return;
    };
    let result = state.exec(r#"
        Функция СкалярПробы(ЗапросПробы)
            ВыборкаПробы = ЗапросПробы.Выполнить().Выбрать();
            Если Не ВыборкаПробы.Следующий() Тогда ВызватьИсключение "нет строки"; КонецЕсли;
            Возврат ВыборкаПробы.Получить(0);
        КонецФункции
        МенеджерПробы = Новый МенеджерВременныхТаблиц;
        ЗапросПробы = Новый Запрос("ВЫБРАТЬ 1 КАК Значение ПОМЕСТИТЬ ВТ");
        ЗапросПробы.МенеджерВременныхТаблиц = МенеджерПробы;
        РезультатПробы = ЗапросПробы.Выполнить();
        Если РезультатПробы.Выгрузить().Количество() <> 1 Тогда ВызватьИсключение "одна строка счётчика"; КонецЕсли;
        ИмяКолонкиПробы = РезультатПробы.Выгрузить().Колонки[0].Имя;
        ВыборкаПробы = РезультатПробы.Выбрать();
        Если Не ВыборкаПробы.Следующий() Или ВыборкаПробы.Получить(0) <> 1 Тогда ВызватьИсключение "счётчик помещения"; КонецЕсли;
        ЗапросПробы.Текст = "ВЫБРАТЬ Значение ИЗ ВТ";
        Если СкалярПробы(ЗапросПробы) <> 1 Тогда ВызватьИсключение "чтение ВТ"; КонецЕсли;
        ЗапросПробы.Текст = "ВЫБРАТЬ 2 КАК Значение ДОБАВИТЬ ВТ";
        Если СкалярПробы(ЗапросПробы) <> 1 Тогда ВызватьИсключение "счётчик добавления"; КонецЕсли;
        ЗапросПробы.Текст = "ВЫБРАТЬ КОЛИЧЕСТВО(*) ИЗ ВТ";
        Если СкалярПробы(ЗапросПробы) <> 2 Тогда ВызватьИсключение "два значения ВТ"; КонецЕсли;
        ЗапросПробы.Текст = "УНИЧТОЖИТЬ ВТ";
        Если ЗапросПробы.Выполнить() <> Неопределено Тогда ВызватьИсключение "результат удаления"; КонецЕсли;

        ТаблицаПробы = Новый ТаблицаЗначений;
        ТаблицаПробы.Колонки.Добавить("Значение", Новый ОписаниеТипов("Число"));
        СтрокаПробы = ТаблицаПробы.Добавить();
        СтрокаПробы.Значение = 1;
        ЗапросПробы.Текст = "ВЫБРАТЬ Источник.Значение КАК Значение ПОМЕСТИТЬ Снимок ИЗ &Таблица КАК Источник";
        ЗапросПробы.УстановитьПараметр("Таблица", ТаблицаПробы);
        ЗапросПробы.Выполнить();
        СтрокаПробы.Значение = 2;
        ЗапросПробы.Текст = "ВЫБРАТЬ Значение ИЗ Снимок";
        Если СкалярПробы(ЗапросПробы) <> 1 Тогда ВызватьИсключение "параметр не стал снимком"; КонецЕсли;

        НетипизированнаяПробы = Новый ТаблицаЗначений;
        НетипизированнаяПробы.Колонки.Добавить("Значение");
        НетипизированнаяПробы.Добавить().Значение = 1;
        ЗапросПробы.УстановитьПараметр("Таблица", НетипизированнаяПробы);
        ЗапросПробы.Текст = "ВЫБРАТЬ Источник.Значение ПОМЕСТИТЬ НетТипа ИЗ &Таблица КАК Источник";
        ОтказПробы = Ложь;
        Попытка ЗапросПробы.Выполнить(); Исключение ОтказПробы = Истина; КонецПопытки;
        Если Не ОтказПробы Тогда ВызватьИсключение "угадан тип колонки"; КонецЕсли;

        НеподдержаннаяПробы = Новый ТаблицаЗначений;
        НеподдержаннаяПробы.Колонки.Добавить("Значение", Новый ОписаниеТипов("Структура"));
        НеподдержаннаяПробы.Добавить().Значение = Новый Структура("Ключ", 1);
        ЗапросПробы.УстановитьПараметр("Таблица", НеподдержаннаяПробы);
        ОтказПробы = Ложь;
        Попытка ЗапросПробы.Выполнить(); Исключение ОтказПробы = Истина; КонецПопытки;
        Если Не ОтказПробы Тогда ВызватьИсключение "угадан SQL-тип структуры"; КонецЕсли;

        МенеджерПробы.Закрыть();
        МенеджерПробы.Close();
        ЗапросПробы = Новый Запрос("ВЫБРАТЬ 7 КАК Значение");
        ЗапросПробы.TempTablesManager = МенеджерПробы;
        Если СкалярПробы(ЗапросПробы) <> 7 Тогда ВызватьИсключение "чтение после закрытия"; КонецЕсли;
        ЗапросПробы.Текст = "ВЫБРАТЬ 1 КАК Значение ПОМЕСТИТЬ Закрытая";
        ОтказПробы = Ложь;
        Попытка ЗапросПробы.Выполнить(); Исключение ОтказПробы = Истина; КонецПопытки;
        Если Не ОтказПробы Тогда ВызватьИсключение "помещение после закрытия"; КонецЕсли;
        Возврат "PASS|" + ИмяКолонкиПробы;
    "#).unwrap();
    let Value::Str(result) = result else {
        panic!("ожидалась строка результата");
    };
    assert_eq!(result.to_string(), "PASS|Count");
    // Специальная колонка и её единственная строка проверены по нативной
    // квитанции `measure-temp-tables-results.platform.txt`.
    eprintln!("temporary.place.column={result}");
}

#[test]
fn a_database_source_is_materialized_before_its_next_committed_change() {
    use super::local_option_register_reads_tests::fixture_sql;
    use open_sdbl::metadata::{
        DbNameEntry, DbNames, InfoBaseUser, LiveColumn, LiveTable, SchemaColumn, SchemaStorage,
        SchemaTable, UserRow, parse_role_rights, resolve_metadata,
    };
    use std::sync::Arc;

    let Some((control, _, _)) = super::session_transactions_tests::control() else {
        return;
    };
    let port = std::env::var("OPEN_SDBL_TRANSACTION_CONTROL_PORT").unwrap();
    let (catalog, native) = crate::functional_options::tests::native("catalog-native", |_| {});
    let owner = native
        .descriptors()
        .iter()
        .find(|d| d.name == "ЗамерВременныхТаблиц")
        .unwrap();
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![DbNameEntry {
            guid: owner.object_guid.clone(),
            alias: "Reference".into(),
            number: 41,
        }]),
        native.descriptors().to_vec(),
        SchemaStorage {
            tables: vec![SchemaTable {
                name: "Reference41".into(),
                number: 41,
                owner: None,
                inline_name: None,
                columns: vec![SchemaColumn {
                    name: "Description".into(),
                    types: vec![open_sdbl::metadata::ColumnType {
                        tag: "S".into(),
                        reference_target: None,
                    }],
                }],
                indexes: vec![],
            }],
            anomalies: vec![],
        },
        vec![LiveTable {
            name: "_reference41".into(),
            columns: vec![LiveColumn {
                name: "_description".into(),
                data_type: "character varying".into(),
            }],
            indexes: vec![],
        }],
    )
    .snapshot;
    let catalog = crate::functional_options::project(catalog, &metadata).unwrap();
    // Метаданные и разрешающая роль — исходные ресурсы CF. Физический номер
    // таблицы и пользователь — собственный стенд, не измерение хранения UUID.
    let role = "3cfbfd8c-c47e-4185-8f69-d581cf8cedec";
    let user = "11111111-1111-1111-1111-111111111111";
    let record = format!("{{{user},\"FunctionalAdmin\",\"\",{{1,{role}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|b| b ^ 0x5a));
    let mut access = AccessStore::new(&metadata);
    access.set_users(vec![
        InfoBaseUser::new(
            UserRow {
                name: "FunctionalAdmin",
                description: "",
                os_name: "",
                email: "",
                show_in_list: false,
                standard_authentication: true,
                administrative: false,
            },
            &encoded,
        )
        .unwrap(),
    ]);
    access.insert_rights(role.parse().unwrap(), parse_role_rights(include_bytes!(
        "../tests/fixtures/functional-options/source-rights-native/3cfbfd8c-c47e-4185-8f69-d581cf8cedec.rights.deflate"
    )).unwrap());
    let principal = Principal::new(user, "FunctionalAdmin", vec![role.into()]);
    let adapter = Rc::new(DatabaseDataAccess::new(
        Rc::clone(&control.connection),
        metadata,
        access,
        &principal,
    ));
    let engine = open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(Arc::new(catalog), principal).with_data_access(adapter),
        ))
        .build();
    fixture_sql(
        &port,
        "CREATE TABLE _reference41 (_description varchar(25)); INSERT INTO _reference41 VALUES ('Before');",
    );
    let module = engine.compile(r#"
        Перем МенеджерПробы;
        Процедура ПоместитьПробы() Экспорт
            ЗапросПробы = Новый Запрос("ВЫБРАТЬ Источник.Наименование КАК Значение ПОМЕСТИТЬ СнимокБазы ИЗ Справочник.ЗамерВременныхТаблиц КАК Источник");
            ЗапросПробы.МенеджерВременныхТаблиц = МенеджерПробы;
            ЗапросПробы.Выполнить();
        КонецПроцедуры
        Функция ПрочитатьПробы(ВременнаяПробы) Экспорт
            Если ВременнаяПробы Тогда
                ЗапросПробы = Новый Запрос("ВЫБРАТЬ Значение ИЗ СнимокБазы");
                ЗапросПробы.МенеджерВременныхТаблиц = МенеджерПробы;
            Иначе
                ЗапросПробы = Новый Запрос("ВЫБРАТЬ Наименование КАК Значение ИЗ Справочник.ЗамерВременныхТаблиц");
            КонецЕсли;
            ВыборкаПробы = ЗапросПробы.Выполнить().Выбрать();
            Если Не ВыборкаПробы.Следующий() Тогда ВызватьИсключение "нет строки"; КонецЕсли;
            Возврат ВыборкаПробы.Значение;
        КонецФункции
        МенеджерПробы = Новый МенеджерВременныхТаблиц;
    "#).unwrap();
    let instance = state.instantiate(&module).unwrap();
    instance.call(&mut state, "ПоместитьПробы", vec![]).unwrap();
    // Только внешний писатель стенда изменяет свою строку. Поддержка записи
    // объектов через ORM этим контролем не заявляется.
    fixture_sql(&port, "UPDATE _reference41 SET _description = 'After';");
    let oracle = include_str!(
        "../../../tests/conformance/measure/1c/measure-temp-tables-results.platform.txt"
    );
    for (temporary, id) in [
        (false, "TEMP.RESULT.DB.SOURCE.AFTER.CHANGE"),
        (true, "TEMP.RESULT.DB.PLACED.AFTER.CHANGE"),
    ] {
        let expected = oracle
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .find_map(|(key, value)| (key == id).then_some(value))
            .unwrap();
        let actual = instance
            .call(
                &mut state,
                "ПрочитатьПробы",
                vec![Value::Boolean(temporary)],
            )
            .unwrap()
            .0;
        assert_eq!(actual, Value::Str(expected.into()));
    }
}
