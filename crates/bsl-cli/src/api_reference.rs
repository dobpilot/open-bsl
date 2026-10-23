//! Генерация справочника BSL API из тех же таблиц, по которым разрешаются
//! вызовы. Порядок намеренно устойчив: таблицы обходятся в порядке объявления,
//! чтобы изменение API давало локальный diff проверяемого снимка.

use std::fmt::Write as _;
use std::io::Write as _;

use bsl_rt::{FunctionKind, LibraryDescriptor, RuntimeRegistry};

use crate::api_reference_sources::{Entry, SourceIndex};

/// Печатает справочник в stdout либо записывает его в указанный файл.
pub fn emit(output: Option<&str>) -> i32 {
    // Конфигурационные библиотеки входят в поставляемый API, но не в
    // обычный движок CLI: без `--config-db` глобал `Метаданные` обязан
    // сохранить прежнюю автономную заглушку.
    let engine = match open_bsl::Engine::builder()
        .register_library(bsl_orm::library())
        .register_library(bsl_session::library())
        .build()
    {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("не удалось собрать каталог API: {error}");
            return 1;
        }
    };
    // Описания и файлы реализаций берутся из исходников того workspace, из
    // которого собран `bsl-cli`; без них справочник был бы другим.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = match SourceIndex::load(&root) {
        Ok(sources) => sources,
        Err(error) => {
            eprintln!("не удалось собрать каталог API: {error}");
            return 1;
        }
    };
    let text = render(engine.registry(), &sources);
    let result = match output {
        Some(path) => std::fs::write(path, text.as_bytes()),
        None => std::io::stdout().lock().write_all(text.as_bytes()),
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            match output {
                Some(path) => eprintln!("не удалось записать справочник в «{path}»: {error}"),
                None => eprintln!("не удалось напечатать справочник: {error}"),
            }
            1
        }
    }
}

/// Детерминированный Markdown-каталог стандартного движка.
fn render(registry: &RuntimeRegistry, sources: &SourceIndex) -> String {
    let mut out = String::new();
    out.push_str(
        "<!-- Сгенерировано bsl-cli --emit-api-reference. Не редактировать вручную. -->\n\n",
    );
    out.push_str("# BSL API open-bsl\n\n");
    out.push_str(
        "Справочник построен из runtime-дескрипторов стандартной сборки `open-bsl`. \
         Имена регистронезависимы; первым указано каноническое написание. \
         Пометка «объявлено без реализации» означает, что имя компилируется, \
         а вызов поднимает ошибку. Описание — первое предложение doc-комментария \
         реализации; «Реализация» — файл в `crates/`, где она находится.\n\n",
    );

    write_builtin_functions(&mut out, sources);
    write_builtin_methods(&mut out, sources);
    out.push_str(
        "Свойства нативного `ОписаниеТипов` описаны в \
         [справочнике квалификаторов](type-description-qualifiers.md).\n\n",
    );

    out.push_str("## Компоненты\n\n");
    for library in registry.libraries() {
        writeln!(out, "### `{}` {}\n", library.package(), library.version()).unwrap();

        if !library.dependencies().is_empty() {
            out.push_str("Зависимости: ");
            for (index, dependency) in library.dependencies().iter().enumerate() {
                if index != 0 {
                    out.push_str(", ");
                }
                write!(out, "`{}={}`", dependency.package, dependency.version).unwrap();
            }
            out.push_str(".\n\n");
        }

        if !library.globals().is_empty() {
            out.push_str(
                "#### Глобальные свойства\n\n| Имя и псевдонимы | Функция получения |\n|---|---|\n",
            );
            for global in library.globals() {
                let function = library
                    .functions()
                    .iter()
                    .find(|f| f.code == global.function)
                    .expect("реестр проверил функцию глобала");
                writeln!(
                    out,
                    "| {} | {} |",
                    names(global.names),
                    names(function.names)
                )
                .unwrap();
            }
            out.push('\n');
        }

        out.push_str("#### Глобальные функции\n\n");
        if library.functions().is_empty() {
            out.push_str("Нет.\n\n");
        } else {
            out.push_str("| Имя и псевдонимы | Вид | Описание | Реализация |\n");
            out.push_str("|---|---|---|---|\n");
            let table: Vec<&[&str]> = library.functions().iter().map(|f| f.names).collect();
            for function in library.functions() {
                writeln!(
                    out,
                    "| {}{} | {} | {} |",
                    names(function.names),
                    declared_only_mark(function.call.is_none()),
                    function_kind(function.kind),
                    cells(&sources.row(library.package(), function.names, &table)),
                )
                .unwrap();
            }
            out.push('\n');
        }

        out.push_str("#### Конструкторы\n\n");
        if library.constructors().is_empty() {
            out.push_str("Нет.\n\n");
        } else {
            out.push_str("| Имя и псевдонимы | Описание | Реализация |\n");
            out.push_str("|---|---|---|\n");
            let table: Vec<&[&str]> = library.constructors().iter().map(|c| c.names).collect();
            for constructor in library.constructors() {
                writeln!(
                    out,
                    "| {}{} | {} |",
                    names(constructor.names),
                    declared_only_mark(constructor.call.is_none()),
                    cells(&sources.row(library.package(), constructor.names, &table)),
                )
                .unwrap();
            }
            out.push('\n');
        }

        out.push_str("#### Типы объектов\n\n");
        if library.types().is_empty() {
            out.push_str("Нет.\n\n");
        } else {
            out.push_str(
                "| Имя | Представление типа | Дополнительные имена | Описание | Реализация |\n",
            );
            out.push_str("|---|---|---|---|---|\n");
            for ty in library.types() {
                let aliases = type_aliases(ty);
                writeln!(
                    out,
                    "| `{}` | `{}` | {} | {} |",
                    ty.name,
                    ty.type_display,
                    if aliases.is_empty() {
                        "—".to_string()
                    } else {
                        names(&aliases)
                    },
                    cells(&sources.ty(ty.package, ty.name)),
                )
                .unwrap();
            }
            out.push('\n');
        }
        write_object_members(&mut out, library, sources);
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// Пометка записи, объявленной без реализации: имя компилируется, вызов
/// поднимает единую ошибку, и справочник не должен выдавать его за поддержку.
fn declared_only_mark(declared_only: bool) -> &'static str {
    if declared_only {
        " — *объявлено без реализации*"
    } else {
        ""
    }
}

/// Колонки «Описание» и «Реализация» строки справочника.
fn cells(entry: &Entry) -> String {
    let description = entry.description.as_deref().unwrap_or("—");
    match &entry.file {
        Some(file) => format!("{description} | [{file}](../../../crates/{file})"),
        None => format!("{description} | —"),
    }
}

fn write_object_members(out: &mut String, library: &LibraryDescriptor, sources: &SourceIndex) {
    let mut any_members = false;
    for ty in library.types() {
        let members = library
            .object_members()
            .find(|members| std::ptr::eq(members.ty(), *ty));
        let Some(members) = members else {
            continue;
        };
        if members.methods().is_empty()
            && members.properties().is_empty()
            && !members.has_dynamic_properties()
        {
            continue;
        }
        if !any_members {
            out.push_str("#### Члены объектов\n\n");
            any_members = true;
        }
        writeln!(out, "##### `{}`\n", ty.name).unwrap();
        if members.has_dynamic_properties() {
            out.push_str(
                "Свойства зависят от экземпляра; ниже перечислена только статическая часть.\n\n",
            );
        }
        if !members.properties().is_empty() {
            out.push_str("| Свойство | Доступ | Описание | Реализация |\n");
            out.push_str("|---|---|---|---|\n");
            let table: Vec<&[&str]> = members.properties().iter().map(|p| p.names).collect();
            for property in members.properties() {
                writeln!(
                    out,
                    "| {} | {} | {} |",
                    names(property.names),
                    if property.set.is_some() {
                        "чтение и запись"
                    } else {
                        "только чтение"
                    },
                    cells(&sources.member(
                        library.package(),
                        Some(ty.name),
                        property.names,
                        &table
                    )),
                )
                .unwrap();
            }
            out.push('\n');
        }
        if !members.methods().is_empty() {
            out.push_str("| Метод | Описание | Реализация |\n");
            out.push_str("|---|---|---|\n");
            let table: Vec<&[&str]> = members.methods().iter().map(|m| m.names()).collect();
            for method in members.methods() {
                writeln!(
                    out,
                    "| {} | {} |",
                    names(method.names()),
                    cells(&sources.member(
                        library.package(),
                        Some(ty.name),
                        method.names(),
                        &table
                    )),
                )
                .unwrap();
            }
            out.push('\n');
        }
    }
}

fn write_builtin_functions(out: &mut String, sources: &SourceIndex) {
    out.push_str("## Встроенные глобальные функции\n\n");
    out.push_str("| Имя и псевдонимы | Вид | Описание | Реализация |\n");
    out.push_str("|---|---|---|---|\n");
    let mut seen = Vec::new();
    for &(_, function) in bsl_rt::BUILTIN_FN_NAMES {
        if seen.contains(&function) {
            continue;
        }
        seen.push(function);
        let aliases = bsl_rt::BUILTIN_FN_NAMES
            .iter()
            .filter_map(|(name, candidate)| (*candidate == function).then_some(*name))
            .collect::<Vec<_>>();
        let kind = if function.is_intrinsic() {
            "встроенная функция"
        } else if function.is_procedure() {
            "процедура"
        } else {
            "функция"
        };
        writeln!(
            out,
            "| {}{} | {kind} | {} |",
            names(&aliases),
            declared_only_mark(function.is_declared_only()),
            cells(&sources.builtin_fn(&format!("{function:?}"))),
        )
        .unwrap();
    }
    out.push('\n');
}

fn write_builtin_methods(out: &mut String, sources: &SourceIndex) {
    out.push_str("## Встроенные методы\n\n");
    out.push_str(
        "Эти методы обслуживает базовый runtime. Применимость зависит от типа \
         получателя, поэтому ссылка ведёт на общую таблицу методов.\n\n",
    );
    out.push_str("| Имя и псевдонимы | Описание | Реализация |\n");
    out.push_str("|---|---|---|\n");
    let mut seen = Vec::new();
    for &(_, method) in bsl_rt::BUILTIN_METHOD_NAMES {
        if seen.contains(&method) {
            continue;
        }
        seen.push(method);
        let aliases = bsl_rt::BUILTIN_METHOD_NAMES
            .iter()
            .filter_map(|(name, candidate)| (*candidate == method).then_some(*name))
            .collect::<Vec<_>>();
        writeln!(
            out,
            "| {} | {} |",
            names(&aliases),
            cells(&sources.builtin_method(&format!("{method:?}")))
        )
        .unwrap();
    }
    out.push('\n');
}

fn names(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn type_aliases(ty: &bsl_rt::TypeDescriptor) -> Vec<&'static str> {
    let mut aliases = Vec::new();
    for &name in ty.type_names {
        if name != ty.name && name != ty.type_display && !aliases.contains(&name) {
            aliases.push(name);
        }
    }
    aliases
}

fn function_kind(kind: FunctionKind) -> &'static str {
    match kind {
        FunctionKind::Function => "функция",
        FunctionKind::Procedure => "процедура",
        FunctionKind::Intrinsic => "встроенная функция",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_only_entries_are_marked_and_implemented_ones_are_not() {
        let engine = open_bsl::Engine::builder()
            .build()
            .expect("движок собирается");
        let sources =
            SourceIndex::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
                .expect("исходники workspace");
        let text = render(engine.registry(), &sources);
        let row = |name: &str| {
            text.lines()
                .find(|line| line.starts_with(&format!("| `{name}`")))
                .unwrap_or_else(|| panic!("нет строки {name}"))
                .to_string()
        };
        for name in ["ЧислоПрописью", "Шрифт", "ДеревоЗначений"] {
            assert!(row(name).contains(declared_only_mark(true)), "{name}");
        }
        for name in ["СтрДлина", "ЗаписьТекста", "Файл"] {
            assert!(!row(name).contains(declared_only_mark(true)), "{name}");
        }
    }

    #[test]
    fn arities_are_not_rendered_and_unknown_sources_show_dashes() {
        use bsl_rt::*;
        fn call(_: &mut CallContext<'_>, _: &[BslValue]) -> RtResult<BslValue> {
            Ok(BslValue::Undefined)
        }
        fn method(
            _: &dyn ObjectProtocol,
            _: &[BslValue],
            _: &mut CallContext<'_>,
        ) -> RtResult<BslValue> {
            Ok(BslValue::Undefined)
        }
        static TYPE: TypeDescriptor = TypeDescriptor {
            package: "arity-test",
            name: "Раздельный",
            type_display: "Раздельный",
            type_names: &[],
        };
        const COUNTS: Arity = Arity::one_of(&[0, 2, 3, 5]);
        static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(&["Метод"], COUNTS, method)];
        static MEMBERS: &[ObjectMembersDescriptor] =
            &[ObjectMembersDescriptor::new(&TYPE).with_methods(METHODS)];
        static LIBRARY: LibraryDescriptor = LibraryDescriptor::new("arity-test", "0.0.0")
            .with_constructors(&[ConstructorDescriptor {
                code: ConstructorCode::new(1),
                names: &["Раздельный"],
                arity: COUNTS,
                call: Some(call),
            }])
            .with_functions(&[FunctionDescriptor {
                code: FunctionCode::new(1),
                names: &["Раздельная"],
                arity: COUNTS,
                kind: FunctionKind::Function,
                call: Some(call),
            }])
            .with_types(&[&TYPE])
            .with_object_member_groups(&[MEMBERS]);
        let mut builder = RuntimeBuilder::new();
        builder.register(core_library()).register(LIBRARY);
        let text = render(&builder.build().unwrap(), &SourceIndex::default());
        assert!(!text.contains("0, 2, 3, 5"));
        assert!(!text.contains("| Аргументы |") && !text.contains("---:"));
        assert!(text.contains("| `Раздельный` | — | — |"));
        assert!(text.contains("| `Раздельная` | функция | — | — |"));
        assert!(text.contains("| `Метод` | — | — |"));
    }

    #[test]
    fn aliases_do_not_repeat_the_primary_type_name() {
        static TYPE: bsl_rt::TypeDescriptor = bsl_rt::TypeDescriptor {
            package: "test",
            name: "Тип",
            type_display: "Тип",
            type_names: &["Type", "Тип"],
        };
        assert_eq!(type_aliases(&TYPE), ["Type"]);
    }
}

#[cfg(test)]
#[path = "api_reference_global_tests.rs"]
mod global_tests;
