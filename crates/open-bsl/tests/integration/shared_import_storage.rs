use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, Value};

#[test]
fn shared_imports_preserve_static_nested_dynamic_and_private_visibility() {
    let modules = vec![
        ModuleRecipe {
            name: "Данные".into(),
            visibility: ModuleVisibility::Named,
            source: "Функция Число() Экспорт Возврат 40; КонецФункции".into(),
            imports: vec![],
        },
        ModuleRecipe {
            name: "Общий".into(),
            visibility: ModuleVisibility::Global,
            source: "Функция Добавка() Экспорт Возврат 2; КонецФункции".into(),
            imports: vec![],
        },
        ModuleRecipe {
            name: "Скрытый".into(),
            visibility: ModuleVisibility::Private,
            source: "Функция Число() Экспорт Возврат 100; КонецФункции".into(),
            imports: vec![],
        },
        ModuleRecipe {
            name: "Работа".into(),
            visibility: ModuleVisibility::Named,
            source: r#"
Функция Проверить() Экспорт
    Если Данные.Число() + Добавка() <> 42 Тогда ВызватьИсключение "static"; КонецЕсли;
    Попытка
        НеожиданноеЗначение = Вычислить("Скрытый.Число()");
    Исключение
        Возврат Вычислить("Вычислить(""Псевдоним.Число() + Добавка()"")");
    КонецПопытки;
    ВызватьИсключение "private module became visible";
КонецФункции
"#
            .into(),
            imports: vec![("Псевдоним".into(), "Данные".into())],
        },
    ];
    for optimized in [false, true] {
        let engine = Engine::builder()
            .optimizations(if optimized {
                bsl_compiler::Optimizations::all()
            } else {
                bsl_compiler::Optimizations::default()
            })
            .configuration(ModuleGraphRecipe {
                modules: modules.clone(),
                eager_init: false,
            })
            .build()
            .unwrap();
        let entry = engine.compile_entry("Возврат Работа.Проверить();").unwrap();
        assert_eq!(
            engine.new_state().run(&entry).unwrap(),
            Value::number_from_i64(42)
        );
        let text = engine.image_bytecode(&entry, None).unwrap();
        let image = bsl_bytecode::parse_image(&text).unwrap();
        assert_eq!(bsl_bytecode::write_image(&image, None).unwrap(), text);
        let bsl_bytecode::BytecodeImage::Configuration { catalog, .. } = image else {
            panic!("каталог")
        };
        let engine = Engine::builder()
            .configuration_image(catalog, false)
            .build()
            .unwrap();
        let entry = engine.compile_entry("Возврат Работа.Проверить();").unwrap();
        assert_eq!(
            engine.new_state().run(&entry).unwrap(),
            Value::number_from_i64(42)
        );
    }
}
