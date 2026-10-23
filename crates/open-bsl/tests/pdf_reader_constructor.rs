#[cfg(feature = "pdf")]
use open_bsl::{Engine, Value};

#[cfg(feature = "pdf")]
#[test]
fn pdf_reader_declares_documented_arity_and_rejects_unsupported_execution() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ЧтениеPDF", "pDfReAdEr"] {
        for args in ["", "\"input.pdf\""] {
            let expression = format!("Новый {name}({args})");
            engine
                .compile(&format!(
                    "Процедура НеВызывается() Читатель = {expression}; КонецПроцедуры"
                ))
                .unwrap();
            let error = engine
                .new_state()
                .exec(&format!("Возврат {expression};"))
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("ЧтениеPDF") && error.contains("не поддерживается"),
                "{error}"
            );
            assert_eq!(engine.new_state().exec(&format!("Попытка Читатель = {expression}; Исключение Возврат Истина; КонецПопытки; Возврат Ложь;")).unwrap(), Value::Boolean(true));
        }
        assert!(
            engine
                .compile(&format!("Читатель = Новый {name}(1, 2);"))
                .is_err()
        );
    }
}
