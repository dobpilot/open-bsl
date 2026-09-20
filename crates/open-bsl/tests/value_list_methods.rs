use open_bsl::Engine;

#[test]
fn measured_value_list_methods_work_through_direct_and_parameter_calls() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let module = engine
            .compile(
                r#"
Процедура ОчиститьЧерезПараметр(Получатель)
    Получатель.Clear();
КонецПроцедуры

Функция ОшибочноеУдаление(Получатель)
    Возврат Получатель.Delete(Получатель[0]);
КонецФункции

СписокПробы = Новый СписокЗначений;
Первый = СписокПробы.Add("a", "A");
Второй = СписокПробы.Add("b", "B", Истина, Неопределено);
Третий = СписокПробы.Insert("1", "x", "X", Истина, Неопределено);
Если СписокПробы.Count() <> 3 Или СписокПробы.Get(1.9) <> Третий Тогда
    ВызватьИсключение "get";
КонецЕсли;
Если Третий.Value <> "x" Или Третий.Presentation <> "X" Или Не Третий.Check Тогда
    ВызватьИсключение "insert fields";
КонецЕсли;
Если СписокПробы.IndexOf(Первый) <> 0 Или СписокПробы.Индекс(Третий) <> 1 Тогда
    ВызватьИсключение "index";
КонецЕсли;

СписокПробы.Move(Первый, 1);
Если СписокПробы[0] <> Третий Или СписокПробы[1] <> Первый Тогда
    ВызватьИсключение "move";
КонецЕсли;
ОшибкаБыла = Ложь;
Попытка СписокПробы.Move(Первый, 20); Исключение ОшибкаБыла = Истина; КонецПопытки;
Если Не ОшибкаБыла Или СписокПробы[1] <> Первый Тогда ВызватьИсключение "move error"; КонецЕсли;

Копия = СписокПробы.Copy();
Если Копия = СписокПробы Или Копия[0] = СписокПробы[0] Или Копия.Count() <> 3 Тогда
    ВызватьИсключение "copy identity";
КонецЕсли;
Копия.Delete(Копия[0]);
Если Копия.Count() <> 2 Или СписокПробы.Count() <> 3 Тогда ВызватьИсключение "copy independence"; КонецЕсли;

СписокПробы.Удалить(Второй);
Если СписокПробы.Count() <> 2 Или СписокПробы.IndexOf(Второй) <> -1 Тогда
    ВызватьИсключение "delete";
КонецЕсли;

ОшибкаБыла = Ложь;
Попытка Результат = ОшибочноеУдаление(СписокПробы); Исключение ОшибкаБыла = Истина; КонецПопытки;
Если Не ОшибкаБыла Или СписокПробы.Count() <> 2 Тогда ВызватьИсключение "procedure expression"; КонецЕсли;

ОчиститьЧерезПараметр(СписокПробы);
Если СписокПробы.Count() <> 0 Тогда ВызватьИсключение "clear"; КонецЕсли;
"#,
            )
            .unwrap();
        engine.state_builder().build().run(&module).unwrap();
    }
}

#[test]
fn notify_description_accepts_every_measured_value_list_handler_name() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    state
        .exec(
            r#"
СписокПробы = Новый СписокЗначений;
Для Каждого ИмяМетода Из СтрРазделить(
    "Удалить,Delete,Очистить,Clear,Вставить,Insert,Получить,Get,Скопировать,Copy,Сдвинуть,Move,Индекс,IndexOf",
    ",") Цикл
    Описание = Новый ОписаниеОповещения(ИмяМетода, СписокПробы);
КонецЦикла;
"#,
        )
        .unwrap();
}

#[test]
fn begin_delivery_calls_the_same_value_list_method_table_in_every_mode() {
    let source = r#"
СписокПробы = Новый СписокЗначений;
ФайлПробы = Новый Файл("before");
ФайлПробы.BeginInitialization(
    Новый NotifyDescription("Add", СписокПробы, "данные"),
    "after");
Если СписокПробы.Count() <> 0 Тогда ВызватьИсключение "синхронная доставка"; КонецЕсли;
Возврат СписокПробы;
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for bytecode in [false, true] {
            let module = engine.compile(source).unwrap();
            let module = if bytecode {
                engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
            } else {
                module
            };
            let result = engine.new_state().run(&module).unwrap();
            assert_eq!(result.collection_len().unwrap(), 1);
        }
    }
}
