//! Генерация справочника BSL API из тех же таблиц, по которым разрешаются
//! вызовы. Порядок намеренно устойчив: таблицы обходятся в порядке объявления,
//! чтобы изменение API давало локальный diff проверяемого снимка.

use std::fmt::Write as _;
use std::io::Write as _;

use bsl_rt::{FunctionKind, LibraryDescriptor, RuntimeRegistry};

/// Печатает справочник в stdout либо записывает его в указанный файл.
pub fn emit(output: Option<&str>) -> i32 {
    let engine = match crate::engine() {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("не удалось собрать каталог API: {error}");
            return 1;
        }
    };
    let text = render(engine.registry());
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
fn render(registry: &RuntimeRegistry) -> String {
    let mut out = String::new();
    out.push_str(
        "<!-- Сгенерировано bsl-cli --emit-api-reference. Не редактировать вручную. -->\n\n",
    );
    out.push_str("# BSL API open-bsl\n\n");
    out.push_str(
        "Справочник построен из runtime-дескрипторов стандартной сборки `open-bsl`. \
         Имена регистронезависимы; первым указано каноническое написание.\n\n",
    );

    write_builtin_functions(&mut out);
    write_builtin_methods(&mut out);

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

        out.push_str("#### Глобальные функции\n\n");
        if library.functions().is_empty() {
            out.push_str("Нет.\n\n");
        } else {
            out.push_str("| Имя и псевдонимы | Вид | Аргументы |\n");
            out.push_str("|---|---|---:|\n");
            for function in library.functions() {
                writeln!(
                    out,
                    "| {} | {} | {} |",
                    names(function.names),
                    function_kind(function.kind),
                    function.arity,
                )
                .unwrap();
            }
            out.push('\n');
        }

        out.push_str("#### Конструкторы\n\n");
        if library.constructors().is_empty() {
            out.push_str("Нет.\n\n");
        } else {
            out.push_str("| Имя и псевдонимы | Аргументы |\n");
            out.push_str("|---|---:|\n");
            for constructor in library.constructors() {
                writeln!(
                    out,
                    "| {} | {} |",
                    names(constructor.names),
                    constructor.arity,
                )
                .unwrap();
            }
            out.push('\n');
        }

        out.push_str("#### Типы объектов\n\n");
        if library.types().is_empty() {
            out.push_str("Нет.\n\n");
        } else {
            out.push_str("| Имя | Представление типа | Дополнительные имена |\n");
            out.push_str("|---|---|---|\n");
            for ty in library.types() {
                let aliases = type_aliases(ty);
                writeln!(
                    out,
                    "| `{}` | `{}` | {} |",
                    ty.name,
                    ty.type_display,
                    if aliases.is_empty() {
                        "—".to_string()
                    } else {
                        names(&aliases)
                    },
                )
                .unwrap();
            }
            out.push('\n');
        }
        write_object_members(&mut out, library);
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

fn write_object_members(out: &mut String, library: &LibraryDescriptor) {
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
            out.push_str("| Свойство | Доступ |\n");
            out.push_str("|---|---|\n");
            for property in members.properties() {
                writeln!(
                    out,
                    "| {} | {} |",
                    names(property.names),
                    if property.set.is_some() {
                        "чтение и запись"
                    } else {
                        "только чтение"
                    },
                )
                .unwrap();
            }
            out.push('\n');
        }
        if !members.methods().is_empty() {
            out.push_str("| Метод | Аргументы |\n");
            out.push_str("|---|---:|\n");
            for method in members.methods() {
                writeln!(out, "| {} | {} |", names(method.names()), method.arity(),).unwrap();
            }
            out.push('\n');
        }
    }
}

fn write_builtin_functions(out: &mut String) {
    out.push_str("## Встроенные глобальные функции\n\n");
    out.push_str("| Имя и псевдонимы | Вид | Аргументы |\n");
    out.push_str("|---|---|---:|\n");
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
        let (min, max) = function.arity_range();
        let kind = if function.is_intrinsic() {
            "встроенная функция"
        } else if function.is_procedure() {
            "процедура"
        } else {
            "функция"
        };
        writeln!(
            out,
            "| {} | {kind} | {} |",
            names(&aliases),
            arity(min, max),
        )
        .unwrap();
    }
    out.push('\n');
}

fn write_builtin_methods(out: &mut String) {
    out.push_str("## Встроенные методы\n\n");
    out.push_str(
        "Эти методы обслуживает базовый runtime. Применимость и иногда арность \
         зависят от типа получателя.\n\n",
    );
    out.push_str("| Имя и псевдонимы | Аргументы |\n");
    out.push_str("|---|---:|\n");
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
        let arity = method
            .static_arity()
            .map(|count| count.to_string())
            .unwrap_or_else(|| "зависит от получателя".to_string());
        writeln!(out, "| {} | {arity} |", names(&aliases)).unwrap();
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

fn arity(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}…{max}")
    }
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
    fn arity_formats_exact_and_ranged_counts() {
        assert_eq!(arity(2, 2), "2");
        assert_eq!(arity(1, 3), "1…3");
    }

    #[test]
    fn disjoint_arities_are_rendered_for_all_component_members() {
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
                call,
            }])
            .with_functions(&[FunctionDescriptor {
                code: FunctionCode::new(1),
                names: &["Раздельная"],
                arity: COUNTS,
                kind: FunctionKind::Function,
                call,
            }])
            .with_types(&[&TYPE])
            .with_object_member_groups(&[MEMBERS]);
        let mut builder = RuntimeBuilder::new();
        builder.register(core_library()).register(LIBRARY);
        let text = render(&builder.build().unwrap());
        assert!(text.contains("| `Раздельный` | 0, 2, 3, 5 |"));
        assert!(text.contains("| `Раздельная` | функция | 0, 2, 3, 5 |"));
        assert!(text.contains("| `Метод` | 0, 2, 3, 5 |"));
    }

    #[test]
    fn aliases_do_not_repeat_the_primary_type_name() {
        static TYPE: bsl_rt::TypeDescriptor = bsl_rt::TypeDescriptor {
            package: "test",
            name: "Тип",
            type_display: "Тип с пробелом",
            type_names: &["Type", "Тип"],
        };
        assert_eq!(type_aliases(&TYPE), ["Type"]);
    }
}
