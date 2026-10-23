use open_bsl::Engine;

#[test]
fn background_job_execute_preserves_explicit_omitted_positions() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile(
            "Менеджер = Новый ФоновыеЗадания; Менеджер.Выполнить(\"Модуль.Метод\", , \"ключ\");",
        )
        .unwrap();
}
