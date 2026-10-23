use super::*;

#[test]
fn named_catalogs_share_every_common_import_without_retaining_self() {
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: (0..128)
                .map(|index| ModuleRecipe {
                    name: format!("Модуль{index}"),
                    source: "Функция Ответ() Экспорт Возврат 42; КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Named,
                })
                .collect(),
            eager_init: false,
        })
        .build()
        .unwrap();
    let catalog = engine.catalog().unwrap();
    let mut first = vec![None; catalog.modules.len()];
    for (own, module) in catalog.modules.iter().enumerate() {
        assert_eq!(module.program.imports.len(), catalog.modules.len() - 1);
        for import in &module.program.imports {
            let target = import.module.index();
            assert_ne!(target, own);
            if let Some(previous) = first[target] {
                assert!(std::ptr::eq(previous, import));
            } else {
                first[target] = Some(import);
            }
        }
    }
}
