use open_bsl::Engine;

#[test]
fn data_composition_settings_item_view_mode_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Авто) <> "Авто"
    Или Строка(РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Обычный) <> "Обычный"
    Или Строка(РежимОтображенияЭлементаНастройкиКомпоновкиДанных.БыстрыйДоступ) <> "Быстрый доступ"
    Или Строка(РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Недоступный) <> "Недоступный"
    Или Строка(ТипЗнч(РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Авто)) <> "РежимОтображенияЭлементаНастройкиКомпоновкиДанных"
    Или DataCompositionSettingsItemViewMode.Auto <> РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Авто
    Или DataCompositionSettingsItemViewMode.Normal <> РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Обычный
    Или DataCompositionSettingsItemViewMode.QuickAccess <> РежимОтображенияЭлементаНастройкиКомпоновкиДанных.БыстрыйДоступ
    Или DataCompositionSettingsItemViewMode.Inaccessible <> РежимОтображенияЭлементаНастройкиКомпоновкиДанных.Недоступный Тогда
    ВызватьИсключение "data-composition-settings-item-view-mode";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
