use open_bsl::Engine;

#[test]
fn platform_type_exposes_the_complete_measured_enumeration() {
    let engine = Engine::builder().build().unwrap();
    let source = r#"
Если Строка(ТипПлатформы.Android_ARM) <> "Android ARM"
    Или Строка(ТипПлатформы.Android_ARM_64) <> "Android ARM_64"
    Или Строка(ТипПлатформы.Android_x86) <> "Android x86"
    Или Строка(ТипПлатформы.Android_x86_64) <> "Android x86_64"
    Или Строка(ТипПлатформы.iOS_ARM) <> "iOS ARM"
    Или Строка(ТипПлатформы.iOS_ARM_64) <> "iOS ARM_64"
    Или Строка(ТипПлатформы.Linux_ARM64) <> "Linux ARM64"
    Или Строка(ТипПлатформы.Linux_E2K) <> "Linux E2K"
    Или Строка(ТипПлатформы.Linux_x86) <> "Linux x86"
    Или Строка(ТипПлатформы.Linux_x86_64) <> "Linux x86-64"
    Или Строка(ТипПлатформы.MacOS_x86) <> "MacOS x86"
    Или Строка(ТипПлатформы.MacOS_x86_64) <> "MacOS x86-64"
    Или Строка(ТипПлатформы.Windows_x86) <> "Windows x86"
    Или Строка(ТипПлатформы.Windows_x86_64) <> "Windows x86-64"
    Или Строка(ТипПлатформы.WinRT_ARM) <> "WinRT ARM"
    Или Строка(ТипПлатформы.WinRT_x86) <> "WinRT x86"
    Или Строка(ТипПлатформы.WinRT_x86_64) <> "WinRT x86_64"
    Или Строка(ТипЗнч(ТипПлатформы.Linux_x86_64)) <> "PlatformType"
    Или PlatformType.Linux_x86_64 <> ТипПлатформы.Linux_x86_64 Тогда
    ВызватьИсключение "platform-type";
КонецЕсли;
"#;
    let module = engine.compile(source).unwrap();
    engine.new_state().run(&module).unwrap();
}
