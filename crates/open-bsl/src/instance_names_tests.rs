use super::*;
use crate::Engine;

#[test]
fn binding_preserves_unbundled_debug_chunks() {
    let engine = Engine::builder().debug_info(true).build().unwrap();
    let mut program = engine
        .compile("Перем Значение; Функция Читать(Данные) Экспорт Возврат Данные.Right; КонецФункции Значение = 1;")
        .unwrap()
        .program;
    assert!(
        program
            .chunks
            .iter()
            .all(|chunk| chunk.bundle_len().is_empty())
    );
    let mut runtime = RuntimeShapes::seeded(vec!["Other".into()], Vec::new(), None);
    bind(&mut program, &mut runtime).unwrap();
    assert!(
        program
            .chunks
            .iter()
            .all(|chunk| chunk.bundle_len().is_empty())
    );
    bsl_bytecode::image::verify(&program).unwrap();
}
