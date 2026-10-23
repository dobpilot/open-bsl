//! Диагностика хранения общей таблицы имён каталога без исполнения BSL.

use bsl_compiler::{BuildOptions, compile_configuration};
use std::collections::HashSet;

fn main() {
    let mut resolved = Vec::new();
    for module in 0..64 {
        let expression = (0..32)
            .map(|field| format!("Объект.Поле{module}_{field}"))
            .collect::<Vec<_>>()
            .join(" + ");
        let parsed = bsl_syntax::parse(&format!(
            "Функция Прочитать(Объект) Экспорт Возврат {expression}; КонецФункции"
        ))
        .expect("разбор диагностической программы");
        resolved.push(bsl_sema::resolve_program(&parsed.items).expect("разрешение имён"));
    }
    let modules = resolved
        .iter()
        .enumerate()
        .map(|(index, program)| (format!("Модуль{index}"), program))
        .collect::<Vec<_>>();
    let (catalog, _) = compile_configuration(&modules, None, BuildOptions::default())
        .expect("сборка диагностического каталога");
    let mut pointers = HashSet::new();
    let mut retained_bytes = 0;
    let mut unique_bytes = 0;
    for module in &catalog.modules {
        let names = &module.program.names;
        let bytes = names.len() * size_of::<String>()
            + names.iter().map(|name| name.capacity()).sum::<usize>();
        retained_bytes += bytes;
        if pointers.insert(names.as_ptr()) {
            unique_bytes += bytes;
        }
        assert_eq!(names, &catalog.modules[0].program.names);
    }
    println!("modules={}", catalog.modules.len());
    println!("names={}", catalog.modules[0].program.names.len());
    println!("table_allocations={}", pointers.len());
    println!("name_storage_bytes={retained_bytes}");
    println!("unique_name_storage_bytes={unique_bytes}");
}
