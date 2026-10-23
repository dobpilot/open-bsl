//! Разбор таблиц нативного образа для проверки артефактов декомпиляции.
//!
//! Форма таблиц сверена с образом buh и читателями `v8unpack`/`v8asm`.
//! Флаги и операнды сохраняются без толкования их исполняемой семантики.
//! Восстанавливаются только целые формы с нативными эталонами исполнения;
//! неподдерживаемые тела отвергаются без выдачи частичного исходника.

use open_sdbl::metadata::{Value, parse_serialized};

#[path = "native_scoped_functions.rs"]
mod scoped_functions;

#[path = "native_source_cleanup.rs"]
mod source_cleanup;

/// Восстанавливает целый поддерживаемый образ в исходный текст BSL.
///
/// # Errors
/// Возвращает место ошибки таблиц или неподдерживаемой формы тела.
pub(crate) fn decompile(bytes: &[u8]) -> Result<String, String> {
    let mut image = parse(bytes)?;
    if let Some(result) = scoped_functions::recover(&image) {
        return result.and_then(|source| source_cleanup::simplify(&source));
    }
    let has_scoped_labels = image.instructions.iter().any(|command| command.code == 42);
    normalize_frame_scoped_shared_labels(&mut image);
    let has_label_jumps = resolve_local_label_jumps(&mut image);
    normalize_measured_jumps(&mut image);
    let result = if has_label_jumps
        || image.procedures.iter().any(|method| {
            measured_pure_private_return(&image, method).is_some()
                || measured_numeric_private_write(&image, method).is_some()
        }) {
        recover_linear_module(&image)
    } else {
        recover_module(&image)
    };
    let recovered = if has_scoped_labels && let Err(error) = result {
        // Сначала сохраняется прежний путь. Только после отказа восстанавливаем
        // исходные таблицы, чтобы общие тела читали метки своего метода.
        // Сборщик проверяет весь состав до выдачи какого-либо исходника.
        let original = parse(bytes)?;
        recover_linear_module_with_scoped_bodies(&original, true).or(Err(error))
    } else {
        result
    };
    recovered.and_then(|source| source_cleanup::simplify(&source))
}

/// Выдаёт только экспортные сигнатуры с явно выбранными хостом заглушками.
/// Тела и инициализация не восстанавливаются и не считаются эквивалентными 1С.
pub(crate) fn exported_interface(bytes: &[u8]) -> Result<String, String> {
    declaration_source(bytes, false)
}

/// Сохраняет порядок собственных методов без восстановления их команд.
pub(crate) fn declarations(bytes: &[u8]) -> Result<String, String> {
    declaration_source(bytes, true)
}

fn declaration_source(bytes: &[u8], include_private: bool) -> Result<String, String> {
    let image = parse(bytes)?;
    let mut source = String::new();
    let mut names = Vec::<&str>::new();
    for (index, procedure) in image.procedures.iter().enumerate() {
        if procedure.flags & 16 == 0 {
            if include_private && !matches!(procedure.flags, 32 | 33) {
                if !matches!(procedure.flags, 0..=3) || !valid_name(&procedure.name) {
                    return Err(format!(
                        "image/Proc[{index}]: неподдерживаемое частное объявление"
                    ));
                }
                // Частный метод не входит в импортируемую поверхность, но
                // занимает тот же чанк перед последующими экспортами.
                source.push_str(&format_method_source(procedure, &[], &[], ""));
            }
            continue;
        }
        let at = format!("image/Proc[{index}]");
        if !matches!(procedure.flags, 16..=19)
            || !valid_name(&procedure.name)
            || names
                .iter()
                .any(|name| open_bsl::folded_eq(name, &procedure.name))
        {
            return Err(format!("{at}: неподдерживаемое экспортное объявление"));
        }
        let variables = procedure.variables.as_deref().unwrap_or_default();
        let defaults = procedure.defaults.as_deref().unwrap_or_default();
        let count = procedure.parameter_count as usize;
        // Хост явно разрешает упрощённую заглушку для непрозрачных
        // параметров без таблицы умолчаний. Флаги не трактуются как режимы
        // платформы; пустое тело сохраняет переданные значения аргументов.
        let opaque_placeholder = count > 0
            && procedure.defaults.is_none()
            && variables.len() >= count
            && variables[..count]
                .iter()
                .all(|variable| !matches!(variable.flags, 5 | 13) && variable.external_slot == 0);
        if count > variables.len() || (!opaque_placeholder && defaults.len() != count) {
            return Err(format!(
                "{at}: таблицы параметров не соответствуют объявлению"
            ));
        }
        let mut parameters = Vec::new();
        for (slot, variable) in variables[..count].iter().enumerate() {
            if !valid_name(&variable.name)
                || (!opaque_placeholder
                    && variable.external_slot != -1
                    && !(variable.flags == 13 && variable.external_slot == 0))
                || variables[..slot]
                    .iter()
                    .any(|previous| open_bsl::folded_eq(&previous.name, &variable.name))
            {
                return Err(format!("{at}/Var[{slot}]: неподдерживаемый параметр"));
            }
            parameters.push(if opaque_placeholder {
                variable.name.clone()
            } else {
                parameter_source(variable, &defaults[slot], &at, slot)?
            });
        }
        let parameter_names = variables[..count]
            .iter()
            .map(|variable| variable.name.as_str())
            .collect::<Vec<_>>();
        source.push_str(&format_method_source(
            procedure,
            &parameters,
            &parameter_names,
            // Контроль `method-flag2` измеряет экспорты `18/19`.
            // Заглушки по выбору хоста синхронны; полного восстановления
            // нативных обещаний и ссылочных эффектов это не разрешает.
            if matches!(procedure.flags, 17 | 19) {
                "Возврат Истина;"
            } else {
                ""
            },
        ));
        names.push(&procedure.name);
    }
    Ok(source)
}

// Контроли `private-return` и `private-frame-read` измеряют только чтение
// в чистой частной функции. Ни биты флагов, ни запись в слот не обобщаются.
fn measured_pure_private_return(image: &Image, method: &Procedure) -> Option<String> {
    if image.entry != u32::MAX
        || !image.variables.is_empty()
        || method.flags != 1
        || method.parameter_count != 1
        || image.instructions.iter().any(|command| command.code == 42)
    {
        return None;
    }
    let variables = method.variables.as_deref()?;
    if variables.iter().enumerate().any(|(index, variable)| {
        !valid_name(&variable.name)
            || variables[..index]
                .iter()
                .any(|previous| open_bsl::folded_eq(&previous.name, &variable.name))
    }) {
        return None;
    }
    let ordinary =
        matches!(variables, [variable] if variable.flags == 5 && variable.external_slot == -1);
    let one_alias = matches!(variables, [variable] if variable.flags == 1338 && matches!(variable.external_slot, 0 | 1));
    let four_aliases = variables.len() == 4
        && variables
            .iter()
            .zip([1338, 1459, 3091, 3027])
            .all(|(variable, flags)| variable.flags == flags && variable.external_slot == 0);
    if !ordinary && !one_alias && !four_aliases {
        return None;
    }
    let measured_defaults =
        method.defaults.as_deref() == Some(&[Value::List(vec![Value::String(String::new())])]);
    // Контроль `private-frame-no-defaults` измеряет отсутствие таблицы только
    // у этого четырёхслотового кадра. Арность всех статических вызовов ниже
    // проверяет обычное восстановление вызывающего тела; сигнатура не меняется.
    let measured_absent_defaults = method.defaults.is_none()
        && four_aliases
        && !image
            .instructions
            .iter()
            .any(|command| matches!(command.code, 52 | 109))
        && !image.procedures.iter().any(|procedure| {
            matches!(procedure.flags, 32 | 33)
                && ["Eval", "Вычислить", "Execute", "Выполнить"]
                    .iter()
                    .any(|name| open_bsl::folded_eq(name, &procedure.name))
        });
    if !measured_defaults && !measured_absent_defaults {
        return None;
    }
    let start = method.entry as usize;
    let end = method_end(image, method.entry);
    let mut commands = image.instructions.get(start..end)?;
    if commands.last()
        == Some(&Instruction {
            code: 22,
            operand: 0,
        })
    {
        commands = &commands[..commands.len() - 1];
    }
    if commands.len() != 12
        || commands[..6].iter().any(|command| {
            *command
                != (Instruction {
                    code: 1,
                    operand: 4,
                })
        })
        || commands[6]
            != (Instruction {
                code: 1,
                operand: 5,
            })
        || commands[7].code != 3
        || commands[8]
            != (Instruction {
                code: 17,
                operand: 0,
            })
        || commands[9]
            != (Instruction {
                code: 45,
                operand: 0,
            })
        || commands[10]
            != (Instruction {
                code: 39,
                operand: (start + 11) as i64,
            })
        || commands[11]
            != (Instruction {
                code: 1,
                operand: 6,
            })
        || image.instructions.iter().enumerate().any(|(at, command)| {
            !(start..end).contains(&at)
                && matches!(command.code, 30 | 31 | 39 | 40 | 41 | 44)
                && usize::try_from(command.operand)
                    .is_ok_and(|target| (start..end).contains(&target))
        })
    {
        return None;
    }
    let variable = variables.get(usize::try_from(commands[7].operand).ok()?)?;
    let value = if ordinary {
        variable.name.clone()
    } else {
        let slot = usize::try_from(variable.external_slot).ok()?;
        if !matches!(image.constants.get(slot)?, Value::List(fields)
            if matches!(fields.as_slice(), [Value::String(tag), Value::Atom(_)] if tag == "N"))
        {
            return None;
        }
        constant_literals(&image.constants).ok()?.get(slot)?.clone()
    };
    Some(format!(
        "Функция {}({})\nВозврат {value};\nКонецФункции\n",
        method.name, variables[0].name
    ))
}

// Контроль `private-alias-direct-observation` отдельно наблюдает запись в кадр
// и неизменную константу вызывающего метода. Слот восстанавливается локально.
fn measured_numeric_private_write(image: &Image, method: &Procedure) -> Option<String> {
    if image.entry != u32::MAX
        || !image.variables.is_empty()
        || method.flags != 1
        || method.parameter_count != 1
        || method.defaults.as_deref() != Some(&[Value::List(vec![Value::String(String::new())])])
        || image.instructions.iter().any(|command| command.code == 42)
    {
        return None;
    }
    let variables = method.variables.as_deref()?;
    let [variable] = variables else { return None };
    if variable.flags != 1338 || variable.external_slot != 0 || !valid_name(&variable.name) {
        return None;
    }
    if !matches!(image.constants.first()?, Value::List(fields)
        if matches!(fields.as_slice(), [Value::String(tag), Value::Atom(_)] if tag == "N"))
    {
        return None;
    }
    let start = method.entry as usize;
    let end = method_end(image, method.entry);
    let mut commands = image.instructions.get(start..end)?;
    if commands.last()
        == Some(&Instruction {
            code: 22,
            operand: 0,
        })
    {
        commands = &commands[..commands.len() - 1];
    }
    let expected = [
        (1, 4),
        (3, 0),
        (3, 0),
        (4, 0),
        (24, 0),
        (16, 0),
        (1, 5),
        (3, 0),
        (17, 0),
        (45, 0),
        (39, (start + 11) as i64),
        (1, 6),
    ];
    if commands.len() != expected.len()
        || commands
            .iter()
            .zip(expected)
            .any(|(command, (code, operand))| *command != Instruction { code, operand })
        || image.instructions.iter().enumerate().any(|(at, command)| {
            !(start..end).contains(&at)
                && matches!(command.code, 30 | 31 | 39 | 40 | 41 | 44)
                && usize::try_from(command.operand)
                    .is_ok_and(|target| (start..end).contains(&target))
        })
    {
        return None;
    }
    let constant = constant_literals(&image.constants).ok()?.first()?.clone();
    let mut local = "__ЧастныйСлот".to_owned();
    while open_bsl::folded_eq(&local, &variable.name) {
        local.push('_');
    }
    Some(format!(
        "Функция {}({})\nПерем {local};\n{local} = {constant};\n{local} = {local} + {constant};\nВозврат {local};\nКонецФункции\n",
        method.name, variable.name
    ))
}

// Контроль `shared-label-frame` измеряет разные цели одной команды в двух кадрах.
// Нормализуем только его чистый префикс; каждый метод сохраняет собственное тело.
fn normalize_frame_scoped_shared_labels(image: &mut Image) {
    if image.entry != u32::MAX || !image.variables.is_empty() {
        return;
    }
    for public_index in 0..image.procedures.len() {
        let public = &image.procedures[public_index];
        if !matches!(public.flags, 1 | 17) || public.parameter_count != 1 {
            continue;
        }
        let start = public.entry as usize;
        let Some(end) = start.checked_add(43) else {
            continue;
        };
        let Some(commands) = image.instructions.get(start..end) else {
            continue;
        };
        let Some(private_index) = image.procedures.iter().position(|private| {
            private.entry as usize == start + 19
                && private.flags == 2
                && private.parameter_count == 1
                && private.defaults == public.defaults
                && measured_unused_frames(public, private)
        }) else {
            continue;
        };
        let private = &image.procedures[private_index];
        if public.labels.get(4).copied() != Some((start + 32) as u32)
            || private.labels.get(4).copied() != Some((start + 42) as u32)
            || image.procedures.iter().enumerate().any(|(index, method)| {
                index != public_index
                    && index != private_index
                    && (start..end).contains(&(method.entry as usize))
            })
        {
            continue;
        }
        let expected = [
            (0, 6, 0),
            (1, 6, 0),
            (2, 33, 0),
            (3, 41, (start + 13) as i64),
            (13, 6, 0),
            (14, 41, end as i64),
            (15, 44, (start + 18) as i64),
            (16, 45, 1),
            (17, 39, (start + 19) as i64),
            (18, 46, 0),
            (19, 39, (start + 20) as i64),
            (20, 44, (start + 23) as i64),
            (21, 45, 1),
            (22, 39, (start + 24) as i64),
            (23, 46, 0),
            (24, 42, 4),
            (32, 3, 0),
            (33, 3, 0),
            (35, 24, 0),
            (36, 16, 0),
            (37, 3, 0),
            (38, 17, 0),
            (39, 45, 0),
            (40, 39, (start + 41) as i64),
            (42, 22, 0),
        ];
        if expected.into_iter().any(|(at, code, operand)| commands[at] != (Instruction { code, operand }))
            || (4..13).chain(25..32).any(|at| commands[at] != (Instruction { code: 1, operand: 1 }))
            || commands[41].code != 1 || commands[41].operand <= 0
            || commands[34].code != 4 || commands[34].operand < 0
            || !image.constants.get(commands[34].operand as usize).is_some_and(|constant| {
                matches!(constant, Value::List(fields) if matches!(fields.as_slice(), [Value::String(tag), Value::Atom(_)] if tag == "N"))
            })
            || image.instructions.iter().enumerate().any(|(at, command)| {
                !(start..end).contains(&at)
                    && (command.code == 42
                        || (matches!(command.code, 30 | 31 | 39 | 40 | 41 | 44)
                            && usize::try_from(command.operand)
                                .is_ok_and(|target| (start..end).contains(&target))))
            })
            || image.instructions.last() != Some(&Instruction { code: 22, operand: 0 })
        { continue; }
        let Ok(empty_entry) = u32::try_from(image.instructions.len() - 1) else {
            continue;
        };
        // Пустой частный путь получает отдельный End: его вход не обрезает
        // завершающую команду публичной функции. Объявление не удаляется.
        for command in &mut image.instructions[start..start + 32] {
            *command = Instruction {
                code: 1,
                operand: 1,
            };
        }
        image.procedures[private_index].entry = empty_entry;
        image.instructions.push(Instruction {
            code: 22,
            operand: 0,
        });
    }
}

fn measured_unused_frames(public: &Procedure, private: &Procedure) -> bool {
    public
        .variables
        .as_ref()
        .zip(private.variables.as_ref())
        .is_some_and(|(public, private)| {
            public.len() == 5
                && private.len() == 4
                && [public, private].into_iter().all(|variables| {
                    variables.iter().enumerate().all(|(slot, variable)| {
                        variable.flags == if slot == 0 { 5 } else { 1 }
                            && variable.external_slot == -1
                    })
                })
        })
}

fn resolve_local_label_jumps(image: &mut Image) -> bool {
    let has_jumps = image.instructions.iter().any(|command| command.code == 42);
    for procedure in &image.procedures {
        if !matches!(procedure.flags, 0 | 1 | 2 | 16 | 17) {
            continue;
        }
        let start = procedure.entry as usize;
        let end = method_end(image, procedure.entry);
        if start >= end || end > image.instructions.len() {
            continue;
        }
        for index in start..end {
            let command = &image.instructions[index];
            if command.code != 42 {
                continue;
            }
            let target = usize::try_from(command.operand)
                .ok()
                .and_then(|label| procedure.labels.get(label))
                .map(|&target| target as usize);
            if let Some(target) =
                target.filter(|&target| target >= start && target != index && target < end)
            {
                // Нативные `goto-labels-local` и `goto-labels-both` измеряют
                // локальный индекс и абсолютный адрес команды в этом методе.
                image.instructions[index] = Instruction {
                    code: 39,
                    operand: target as i64,
                };
            }
            // Неизвестная ссылка сохраняется для отказа обычного восстановления.
            // Глобальная таблица не подставляется вместо отсутствующей локальной.
        }
    }
    has_jumps
}

fn reachable_commands(image: &Image, entries: &[usize]) -> (Vec<bool>, Vec<Option<bool>>) {
    reachable_commands_with_labels(image, entries, None)
}

fn reachable_commands_with_labels(
    image: &Image,
    entries: &[usize],
    labels: Option<&[u32]>,
) -> (Vec<bool>, Vec<Option<bool>>) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Facts {
        Unknown,
        One(bool),
        Two(bool, bool),
    }
    use Facts::{One, Two, Unknown};
    let mut states = vec![None; image.instructions.len()];
    let mut queue = std::collections::VecDeque::new();
    for &entry in entries {
        if entry < states.len() && states[entry].is_none() {
            states[entry] = Some(Unknown);
            queue.push_back(entry);
        }
    }
    // Это только факты о чистом булевом суффиксе, не исполнение образа.
    // Любая иная операция сбрасывает их; слияние разных фактов неизвестно.
    while let Some(index) = queue.pop_front() {
        let before = states[index].unwrap();
        let command = &image.instructions[index];
        let after = match (command.code, command.operand, before) {
            (6 | 7, 0, previous) => {
                let value = command.code == 7;
                match previous {
                    One(old) | Two(_, old) => Two(old, value),
                    Unknown => One(value),
                }
            }
            (29, 0, One(value) | Two(_, value)) => One(!value),
            (33, 0, Two(left, right)) => One(left == right),
            (32, 0, previous) => previous,
            _ => Unknown,
        };
        let target = usize::try_from(command.operand).ok();
        let mut successors = [Some((index + 1, after)), None];
        match command.code {
            22 => successors = [None, None],
            // Без таблицы кадра переход остаётся неразрешённым и не удаляется.
            // Блочное восстановление передаёт собственную таблицу меток;
            // остальные пути отвергают неизвестное продолжение явно.
            42 => {
                let destination = labels.and_then(|labels| {
                    target
                        .and_then(|slot| labels.get(slot))
                        .map(|value| *value as usize)
                });
                successors = [destination.map(|target| (target, Unknown)), None];
            }
            39 => successors = [target.map(|target| (target, Unknown)), None],
            40 | 41 => {
                let truth = match before {
                    One(value) | Two(_, value) => Some(value),
                    Unknown => None,
                };
                successors = match truth {
                    Some(value) if value == (command.code == 41) => {
                        [target.map(|target| (target, Unknown)), None]
                    }
                    Some(_) => [Some((index + 1, Unknown)), None],
                    None => [
                        Some((index + 1, Unknown)),
                        target.map(|target| (target, Unknown)),
                    ],
                };
            }
            30 | 31 => {
                // Нативные пробы `short-circuit32` и `constant-short-circuit`
                // проверяют сохранение результата выбранной короткой ветви.
                successors = match before {
                    One(value) | Two(_, value) if value == (command.code == 31) => {
                        [target.map(|target| (target, before)), None]
                    }
                    One(_) => [Some((index + 1, Unknown)), None],
                    Two(previous, _) => [Some((index + 1, One(previous))), None],
                    Unknown => [
                        Some((index + 1, Unknown)),
                        target.map(|target| (target, Unknown)),
                    ],
                };
            }
            // Не предполагаем отсутствие исключения в попытке.
            44 => {
                successors = [
                    Some((index + 1, Unknown)),
                    target.map(|target| (target, Unknown)),
                ]
            }
            _ => {}
        }
        for (target, facts) in successors.into_iter().flatten() {
            if target >= states.len() {
                continue;
            }
            let joined = match states[target] {
                None => facts,
                Some(old) if old == facts => old,
                Some(_) => Unknown,
            };
            if states[target] != Some(joined) {
                states[target] = Some(joined);
                queue.push_back(target);
            }
        }
    }
    let reachable = states.iter().map(|state| state.is_some()).collect();
    let predicates = states
        .into_iter()
        .map(|state| match state {
            Some(One(value) | Two(_, value)) => Some(value),
            _ => None,
        })
        .collect();
    (reachable, predicates)
}

fn normalize_measured_jumps(image: &mut Image) {
    // `unreachable-probe` и `boolean-skips` измеряют чистые булевы
    // префиксы и прямой переход вперёд. Повторяем просмотр после удаления
    // невозможных входящих переходов; параметры не считаются константами.
    let mut entries: Vec<_> = image
        .procedures
        .iter()
        .map(|p| p.entry as usize)
        .chain((image.entry != u32::MAX).then_some(image.entry as usize))
        .collect();
    // Нативные `foreach-branch-trampoline` и `foreach-backedge-trampoline`
    // измеряют перенос цели через один безусловный переход в том же методе.
    // Собираем правки до применения, чтобы не обходить неизмеренные цепочки.
    let target_edits: Vec<_> = image
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let target = usize::try_from(command.operand).ok()?;
            if !matches!(command.code, 39 | 40) || target >= index || target + 1 == index {
                return None;
            }
            let owner = image
                .procedures
                .iter()
                .filter(|procedure| {
                    matches!(procedure.flags, 0 | 1 | 2 | 16 | 17)
                        && procedure.entry as usize <= index
                })
                .max_by_key(|procedure| procedure.entry)?;
            let frame = owner.entry as usize..method_end(image, owner.entry);
            if !frame.contains(&index) || !frame.contains(&target) || target == index {
                return None;
            }
            let trampoline = image.instructions.get(target)?;
            let destination = usize::try_from(trampoline.operand).ok()?;
            if trampoline.code != 39
                || !frame.contains(&destination)
                || destination == index
                || destination == target
                || image.instructions.get(destination)?.code == 39
            {
                return None;
            }
            Some((index, destination as i64))
        })
        .collect();
    for (index, target) in target_edits {
        image.instructions[index].operand = target;
    }
    let mut incoming: Vec<_> = image
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, instruction)| matches!(instruction.code, 30 | 31 | 39 | 40 | 41 | 42 | 44))
        .filter_map(|(source, instruction)| {
            usize::try_from(instruction.operand)
                .ok()
                .map(|target| (source, target))
        })
        .collect();
    entries.sort_unstable();
    let (reachable, predicates) = reachable_commands(image, &entries);
    incoming.sort_unstable_by_key(|&(_, target)| target);
    let ranges: Vec<_> = image
        .procedures
        .iter()
        .filter(|p| matches!(p.flags, 0 | 1 | 2 | 16 | 17))
        .map(|p| p.entry as usize..method_end(image, p.entry))
        .chain((image.entry != u32::MAX).then_some(image.entry as usize..image.instructions.len()))
        .collect();
    // `procedure-dead-condition` измеряет известный недостижимый префикс
    // после проверенного выхода. Неизвестные хвосты остаются диагностикой;
    // это не общий проход удаления любых недостижимых инструкций.
    for range in &ranges {
        if range.end > image.instructions.len()
            || !image.procedures.iter().any(|procedure| {
                procedure.entry as usize == range.start && matches!(procedure.flags, 0 | 2 | 16)
            })
        {
            continue;
        }
        for index in range.clone() {
            if !reachable[index]
                || image.instructions[index]
                    != (Instruction {
                        code: 45,
                        operand: 0,
                    })
                || index + 2 >= range.end
                || image.instructions[index + 1].code != 39
            {
                continue;
            }
            let Ok(target) = usize::try_from(image.instructions[index + 1].operand) else {
                continue;
            };
            let terminal = range.end - 1;
            if image.instructions[terminal]
                != (Instruction {
                    code: 22,
                    operand: 0,
                })
                || !(target == terminal
                    || target + 1 == terminal
                        && image.instructions[target].code == 1
                        && image.instructions[target].operand > 0)
                || target <= index + 1
            {
                continue;
            }
            let start = index + 2;
            let end = (start..target).find(|&at| reachable[at]).unwrap_or(target);
            if start < end
                && image.instructions[start..end].iter().all(|command| {
                    matches!(
                        command.code,
                        1 | 3 | 4 | 12 | 16 | 18 | 20 | 25 | 33 | 39 | 40 | 51 | 53 | 57
                    )
                })
            {
                for command in &mut image.instructions[start..end] {
                    *command = Instruction {
                        code: 1,
                        operand: 1,
                    };
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for range in &ranges {
            if range.start >= range.end || range.end > image.instructions.len() {
                continue;
            }
            let mut index = range.start;
            while index < range.end {
                let literal = |at: usize| {
                    let command = &image.instructions[at];
                    (command.operand == 0 && matches!(command.code, 6 | 7))
                        .then_some(command.code == 7)
                };
                let predicate = if index >= range.start + 4
                    && matches!(image.instructions[index - 3].code, 30 | 31)
                    && image.instructions[index - 3].operand == index as i64
                    && matches!(
                        image.instructions[index - 1],
                        Instruction {
                            code: 32 | 69,
                            operand: 0
                        }
                    ) {
                    literal(index - 4)
                        .zip(literal(index - 2))
                        .map(|(left, right)| {
                            let value = if image.instructions[index - 3].code == 30 {
                                left && right
                            } else {
                                left || right
                            };
                            (index - 4, value)
                        })
                } else if index >= range.start + 3
                    && image.instructions[index - 1]
                        == (Instruction {
                            code: 33,
                            operand: 0,
                        })
                {
                    literal(index - 3)
                        .zip(literal(index - 2))
                        .map(|(left, right)| (index - 3, left == right))
                } else if index >= range.start + 2
                    && image.instructions[index - 1]
                        == (Instruction {
                            code: 29,
                            operand: 0,
                        })
                {
                    literal(index - 2).map(|value| (index - 2, !value))
                } else if index > range.start {
                    literal(index - 1).map(|value| (index - 1, value))
                } else {
                    None
                };
                let (mut start, taken) = if matches!(image.instructions[index].code, 40 | 41)
                    && let Some((start, predicate)) = predicate
                {
                    if reachable[index] && predicates[index] != Some(predicate) {
                        index += 1;
                        continue;
                    }
                    (start, predicate == (image.instructions[index].code == 41))
                } else if image.instructions[index].code == 39
                    && (index == range.start || image.instructions[index - 1].code != 45)
                {
                    // Служебные переходы после выхода функции или попытки
                    // сохраняются: их хвост проверяет обычное восстановление.
                    (index, true)
                } else {
                    index += 1;
                    continue;
                };
                let Ok(target) = usize::try_from(image.instructions[index].operand) else {
                    index += 1;
                    continue;
                };
                if target >= image.instructions.len()
                    || (taken && (target <= index + 1 || target >= range.end))
                {
                    index += 1;
                    continue;
                }
                // `foreach-terminal-branches` измеряет пропущенный хвост
                // между выходом и структурным обратным переходом цикла.
                let loop_exit = taken
                    && start == index
                    && image.instructions[index].code == 39
                    && image.instructions[target]
                        == (Instruction {
                            code: 50,
                            operand: 1,
                        })
                    && image.instructions[target - 1].code == 39
                    && image.instructions[target - 1].operand >= range.start as i64
                    && image.instructions[target - 1].operand < index as i64;
                let end = if loop_exit {
                    start = index + 1;
                    target - 1
                } else if taken {
                    target
                } else {
                    index + 1
                };
                if start >= end
                    || image.instructions[start..end].iter().all(|command| {
                        *command
                            == (Instruction {
                                code: 1,
                                operand: 1,
                            })
                    })
                {
                    index += 1;
                    continue;
                }
                if taken && reachable[index + 1..end].iter().any(|&live| live) {
                    index += 1;
                    continue;
                }
                let first_entry = entries.partition_point(|&entry| entry <= start);
                let first_edge = incoming.partition_point(|&(_, target)| target <= start);
                if entries.get(first_entry).is_some_and(|&entry| entry < end)
                    || incoming[first_edge..]
                        .iter()
                        .take_while(|&&(_, target)| target < end)
                        .any(|&(source, _)| {
                            !(start..end).contains(&source)
                                && reachable[source]
                                && matches!(
                                    image.instructions[source].code,
                                    30 | 31 | 39 | 40 | 41 | 42 | 44
                                )
                        })
                {
                    index += 1;
                    continue;
                }
                // Адреса и исходные таблицы читателя не сдвигаются. Только
                // доказанно пропущенный участок заменяется пустыми командами
                // строки, которые существующий восстановитель уже принимает.
                for instruction in &mut image.instructions[start..end] {
                    *instruction = Instruction {
                        code: 1,
                        operand: 1,
                    };
                }
                changed = true;
                index = end;
            }
        }
    }
}

#[derive(Debug)]
struct Image {
    entry: u32,
    instructions: Vec<Instruction>,
    constants: Vec<Value>,
    variables: Vec<Variable>,
    procedures: Vec<Procedure>,
}

#[derive(Debug, PartialEq, Eq)]
struct Instruction {
    code: u32,
    operand: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct Variable {
    name: String,
    flags: u32,
    external_slot: i64,
}

#[derive(Debug)]
struct Procedure {
    name: String,
    flags: u32,
    parameter_count: u32,
    entry: u32,
    variables: Option<Vec<Variable>>,
    defaults: Option<Vec<Value>>,
    labels: Vec<u32>,
}

fn parse(bytes: &[u8]) -> Result<Image, String> {
    // Подписанный SCSU-образ сначала преобразуется целиком: имена и
    // константы нельзя восстанавливать с заменой недопустимых символов.
    let decoded;
    let bytes = if bytes.starts_with(&[0x0e, 0xfe, 0xff]) {
        decoded = crate::scsu::decode(bytes)?;
        decoded.as_bytes()
    } else {
        bytes
    };
    let value = parse_serialized(bytes).map_err(|error| format!("image: {error}"))?;
    let root = sequence(&value, "image")?;
    if root.first() != Some(&Value::Atom("1".into())) {
        return Err("image: неподдерживаемая версия корня".into());
    }
    let mut cmd = None;
    let mut constants = None;
    let mut variables = None;
    let mut procedures = None;
    let mut labels = false;
    for value in &root[1..] {
        let section = sequence(value, "image")?;
        let name = string(section.first(), "image/секция")?;
        match name {
            "Cmd" if cmd.is_none() => cmd = Some(section),
            "Const" if constants.is_none() => constants = Some(section),
            "Var" if variables.is_none() => variables = Some(section),
            "Proc" if procedures.is_none() => procedures = Some(section),
            "Lbl" if !labels => {
                validate_labels(section, "image/Lbl")?;
                labels = true;
            }
            _ => return Err(format!("image/{name}: неизвестная или повторная секция")),
        }
    }
    let cmd = required(cmd, "Cmd")?;
    let instructions = counted(cmd, 3, "image/Cmd")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let at = format!("image/Cmd[{index}]");
            let fields = sequence(value, &at)?;
            if fields.len() != 2 {
                return Err(format!("{at}: инструкция должна содержать два числа"));
            }
            Ok(Instruction {
                code: unsigned(fields.first(), &at)?,
                operand: signed(fields.get(1), &at)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let constants = counted(required(constants, "Const")?, 2, "image/Const")?.to_vec();
    // В образе авторского контрольного модуля, скомпилированном 1С,
    // пустая глобальная таблица отсутствует. Непустая таблица по-прежнему
    // разбирается полностью, с проверкой заявленного числа записей.
    let variables = match variables {
        Some(section) => parse_variables(section, "image/Var")?,
        None => Vec::new(),
    };
    let procedures = counted(required(procedures, "Proc")?, 2, "image/Proc")?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_procedure(value, &format!("image/Proc[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Image {
        entry: unsigned(cmd.get(2), "image/Cmd/начало")?,
        instructions,
        constants,
        variables,
        procedures,
    })
}

fn recover_function(image: &Image) -> Result<String, String> {
    let global_property = image.variables.len() == 1
        && image
            .procedures
            .first()
            .is_some_and(|p| p.parameter_count == 0)
        && image.instructions.len() == 8;
    let undefined_return = image.constants.is_empty()
        && image
            .procedures
            .first()
            .is_some_and(|p| p.parameter_count == 0);
    // Нативные скрипты `measure-native-image-constant`, `reference` и
    // `value` проверяют эти полные формы образов. Другие тела отвергаются.
    if image.entry != u32::MAX
        || (!global_property && !image.variables.is_empty())
        || image.procedures.len() != 1
        || (!undefined_return && !(1..=3).contains(&image.constants.len()))
    {
        return Err("image: восстановление этой формы модуля пока не реализовано".into());
    }
    let procedure = &image.procedures[0];
    if procedure.flags != 17 || procedure.entry != 0 {
        return Err("image/Proc[0]: неподдерживаемая форма заголовка".into());
    }
    if !valid_name(&procedure.name) {
        return Err("image/Proc[0]: имя нельзя поместить в объявление BSL".into());
    }
    let (parameter, parameter_name, expected): (String, Option<&str>, &[(u32, i64)]) =
        match procedure.parameter_count {
            0 if procedure.variables.as_ref().is_none_or(|v| v.is_empty())
                && procedure.defaults.as_ref().is_none_or(|v| v.is_empty()) =>
            {
                (
                    String::new(),
                    None,
                    &[(1, 1), (4, 0), (17, 0), (45, 0), (39, 5), (1, 3), (22, 0)],
                )
            }
            1 => {
                // Оба целых образа параметра проверены на 1С: `5` меняет
                // аргумент вызывающего кода, `13` сохраняет его значение.
                let variables = procedure.variables.as_deref().unwrap_or_default();
                let [variable] = variables else {
                    return Err("image/Proc[0]/Var: ожидался один параметр".into());
                };
                if !valid_name(&variable.name) || variable.external_slot != -1 {
                    return Err("image/Proc[0]/Var[0]: неподдерживаемый параметр".into());
                }
                let prefix = match variable.flags {
                    5 => "",
                    13 => "Знач ",
                    _ => return Err("image/Proc[0]/Var[0]: неподдерживаемый флаг".into()),
                };
                if procedure.defaults.as_deref()
                    != Some(&[Value::List(vec![Value::String(String::new())])])
                {
                    return Err("image/Proc[0]/DefPrm: неподдерживаемое умолчание".into());
                }
                (
                    format!("{prefix}{}", variable.name),
                    Some(variable.name.as_str()),
                    &[
                        (1, 1),
                        (3, 0),
                        (4, 0),
                        (16, 0),
                        (1, 2),
                        (3, 0),
                        (17, 0),
                        (45, 0),
                        (39, 9),
                        (1, 3),
                        (22, 0),
                    ],
                )
            }
            _ => return Err("image/Proc[0]: неподдерживаемая форма заголовка".into()),
        };
    // Арифметический вариант целиком измерен `measure-native-image-add`.
    let adds_to_parameter = parameter_name.is_some() && image.instructions.len() == 13;
    let previous_year = parameter_name.is_some()
        && image
            .instructions
            .get(3)
            .is_some_and(|instruction| instruction.code == 23);
    let conditional = parameter_name.is_some() && image.instructions.len() == 18;
    let while_loop = parameter_name.is_some() && image.instructions.len() == 19;
    let context_method = parameter_name.is_some() && image.instructions.len() == 10;
    let property_read = parameter_name.is_some() && image.instructions.len() == 8;
    let constructor = parameter_name.is_some()
        && image.instructions.len() == 15
        && image
            .instructions
            .get(3)
            .is_some_and(|instruction| instruction.code == 51);
    let context_procedure =
        parameter_name.is_some() && image.instructions.len() == 15 && !constructor;
    let call_order = parameter_name.is_some()
        && image.instructions.len() == 19
        && image
            .instructions
            .get(3)
            .is_some_and(|instruction| instruction.code == 21);
    let property_write = parameter_name.is_some()
        && image.instructions.len() == 13
        && image
            .instructions
            .get(2)
            .is_some_and(|instruction| instruction.code == 12);
    if image.constants.len()
        != if undefined_return {
            0
        } else if context_procedure || call_order {
            3
        } else if conditional || while_loop || property_write || constructor {
            2
        } else {
            1
        }
    {
        return Err("image/Const: неподдерживаемое число констант тела".into());
    }
    let expected = if global_property {
        &[
            (1, 1),
            (2, 0),
            (12, 0),
            (17, 0),
            (45, 0),
            (39, 6),
            (1, 3),
            (22, 0),
        ][..]
    } else if undefined_return {
        &[(1, 1), (8, 0), (17, 0), (45, 0), (39, 5), (1, 3), (22, 0)][..]
    } else if call_order {
        &[
            (1, 1),
            (3, 0),
            (18, 0),
            (21, 2),
            (5, 0),
            (3, 0),
            (4, 1),
            (18, 1),
            (20, 0),
            (3, 0),
            (18, 0),
            (21, 2),
            (5, 0),
            (24, 0),
            (17, 0),
            (45, 0),
            (39, 17),
            (1, 3),
            (22, 0),
        ][..]
    } else if constructor {
        &[
            (1, 1),
            (3, 0),
            (18, 0),
            (51, 0),
            (16, 0),
            (1, 2),
            (3, 0),
            (18, 0),
            (21, 1),
            (5, 0),
            (17, 0),
            (45, 0),
            (39, 13),
            (1, 3),
            (22, 0),
        ][..]
    } else if context_procedure {
        &[
            (1, 1),
            (3, 0),
            (4, 1),
            (18, 1),
            (20, 0),
            (1, 2),
            (3, 0),
            (18, 0),
            (21, 2),
            (5, 0),
            (17, 0),
            (45, 0),
            (39, 13),
            (1, 3),
            (22, 0),
        ][..]
    } else if property_write {
        &[
            (1, 1),
            (3, 0),
            (12, 0),
            (4, 1),
            (16, 0),
            (1, 2),
            (3, 0),
            (12, 0),
            (17, 0),
            (45, 0),
            (39, 11),
            (1, 3),
            (22, 0),
        ][..]
    } else if property_read {
        &[
            (1, 1),
            (3, 0),
            (12, 0),
            (17, 0),
            (45, 0),
            (39, 6),
            (1, 3),
            (22, 0),
        ][..]
    } else if context_method {
        &[
            (1, 1),
            (3, 0),
            (18, 0),
            (21, 0),
            (5, 0),
            (17, 0),
            (45, 0),
            (39, 8),
            (1, 3),
            (22, 0),
        ][..]
    } else if while_loop {
        &[
            (1, 1),
            (3, 0),
            (4, 0),
            (35, 0),
            (40, 12),
            (1, 2),
            (3, 0),
            (3, 0),
            (4, 1),
            (25, 0),
            (16, 0),
            (39, 0),
            (1, 3),
            (3, 0),
            (17, 0),
            (45, 0),
            (39, 17),
            (1, 4),
            (22, 0),
        ][..]
    } else if conditional {
        &[
            (1, 1),
            (3, 0),
            (40, 8),
            (3, 0),
            (4, 0),
            (16, 0),
            (1, 2),
            (39, 11),
            (3, 0),
            (4, 1),
            (16, 0),
            (1, 3),
            (3, 0),
            (17, 0),
            (45, 0),
            (39, 16),
            (1, 4),
            (22, 0),
        ][..]
    } else if adds_to_parameter {
        &[
            (1, 1),
            (3, 0),
            (3, 0),
            (4, 0),
            (24, 0),
            (16, 0),
            (1, 2),
            (3, 0),
            (17, 0),
            (45, 0),
            (39, 11),
            (1, 3),
            (22, 0),
        ][..]
    } else if previous_year {
        &[
            (1, 3400),
            (3, 0),
            (4, 0),
            (23, 0),
            (75, 0),
            (82, 0),
            (17, 0),
            (45, 0),
            (39, 9),
            (1, 3401),
            (22, 0),
        ][..]
    } else {
        expected
    };
    if image.instructions.len() != expected.len() {
        return Err("image/Cmd: неподдерживаемое тело функции".into());
    }
    for (index, (instruction, &(code, operand))) in
        image.instructions.iter().zip(expected).enumerate()
    {
        // `Line` хранит позицию в исходном модуле. Изолированный
        // возврат даты из buh содержит строки `3877` и `3879`;
        // форма исполняемых команд от номера исходной строки не зависит.
        let source_position = code == 1 && instruction.operand > 0;
        if instruction.code != code || (!source_position && instruction.operand != operand) {
            return Err(format!(
                "image/Cmd[{index}]: инструкция не соответствует проверенному телу"
            ));
        }
    }
    let numbers = constant_literals(&image.constants)?;
    let body = if while_loop && !call_order {
        let condition = recover_expressions(1..4, parameter_name, &numbers, true, image)?;
        let body = recover_expressions(5..11, parameter_name, &numbers, false, image)?;
        let tail = recover_expressions(12..15, parameter_name, &numbers, false, image)?;
        format!("Пока {condition} Цикл\n{body}\nКонецЦикла;\n{tail}")
    } else if conditional {
        // Адреса двух ветвей и их общего продолжения проверены выше
        // по целому нативному образу; произвольный граф пока не принимается.
        let yes = recover_expressions(3..7, parameter_name, &numbers, false, image)?;
        let no = recover_expressions(8..11, parameter_name, &numbers, false, image)?;
        let tail = recover_expressions(11..14, parameter_name, &numbers, false, image)?;
        format!(
            "Если {} Тогда\n{yes}\nИначе\n{no}\nКонецЕсли;\n{tail}",
            parameter_name.unwrap()
        )
    } else {
        recover_expressions(
            0..image.instructions.len(),
            parameter_name,
            &numbers,
            false,
            image,
        )?
    };
    Ok(format!(
        "Функция {}({parameter}) Экспорт\n{body}\nКонецФункции\n",
        procedure.name
    ))
}

fn recover_module(image: &Image) -> Result<String, String> {
    // Внешние объявления не являются вторым локальным телом. Даже малая
    // таблица с таким объявлением требует общего восстановления, иначе
    // проверенный частный шаблон двух локальных методов отвергает модуль.
    if image.entry != u32::MAX
        || image.procedures.len() > 2
        || image
            .procedures
            .iter()
            .any(|procedure| procedure.parameter_count > 1 || matches!(procedure.flags, 2 | 32 | 33))
        || image.procedures.iter().any(|procedure| {
            procedure.variables.as_deref().is_some_and(|variables| {
                variables
                    .iter()
                    .take(procedure.parameter_count as usize)
                    .any(|variable| variable.flags == 13 && variable.external_slot == 0)
            })
        })
        || image
            .instructions
            .iter()
            .any(|instruction| matches!(instruction.code, 29 | 32 | 41 | 52 | 66 | 67 | 70 | 73..=81 | 93..=98 | 109))
    {
        return recover_linear_module(image);
    }
    if image.procedures.len() == 1 {
        return recover_function(image);
    }
    // Пока многопроцедурный случай допускается только в полных формах,
    // проверенных `measure-native-image-local-call`, `local-function`
    // и `export-procedure`.
    if image.entry != u32::MAX
        || !image.variables.is_empty()
        || image.constants.len() != 1
        || image.procedures.len() != 2
    {
        return Err("image: неподдерживаемая форма многопроцедурного модуля".into());
    }
    let local_function = image.procedures[1].flags == 1;
    let expected: &[(u32, i64)] = if local_function {
        &[
            (1, 1),
            (3, 0),
            (18, 1),
            (19, 1),
            (5, 0),
            (17, 0),
            (45, 0),
            (39, 8),
            (1, 3),
            (22, 0),
            (1, 4),
            (3, 0),
            (3, 0),
            (4, 0),
            (24, 0),
            (16, 0),
            (1, 5),
            (3, 0),
            (17, 0),
            (45, 0),
            (39, 21),
            (1, 6),
            (22, 0),
        ]
    } else {
        &[
            (1, 1),
            (3, 0),
            (18, 1),
            (19, 1),
            (1, 2),
            (3, 0),
            (17, 0),
            (45, 0),
            (39, 9),
            (1, 3),
            (22, 0),
            (1, 4),
            (3, 0),
            (3, 0),
            (4, 0),
            (24, 0),
            (16, 0),
            (1, 5),
            (22, 0),
        ]
    };
    if image.instructions.len() != expected.len() {
        return Err("image/Cmd: неподдерживаемое тело локального вызова".into());
    }
    for (index, (instruction, &(code, operand))) in
        image.instructions.iter().zip(expected).enumerate()
    {
        if instruction.code != code || instruction.operand != operand {
            return Err(format!(
                "image/Cmd[{index}]: инструкция не соответствует проверенному вызову"
            ));
        }
    }
    let mut parameters = Vec::new();
    let exported_procedure = image.procedures[1].flags == 16;
    let callee_header = if local_function {
        (1, 10)
    } else if exported_procedure {
        (16, 11)
    } else {
        (0, 11)
    };
    for (index, (procedure, (flags, entry))) in image
        .procedures
        .iter()
        .zip([(17, 0), callee_header])
        .enumerate()
    {
        let at = format!("image/Proc[{index}]");
        let variables = procedure.variables.as_deref().unwrap_or_default();
        let [variable] = variables else {
            return Err(format!("{at}/Var: ожидался один параметр"));
        };
        if procedure.flags != flags
            || procedure.entry != entry
            || procedure.parameter_count != 1
            || !valid_name(&procedure.name)
            || !valid_name(&variable.name)
            || variable.flags != 5
            || variable.external_slot != -1
            || procedure.defaults.as_deref()
                != Some(&[Value::List(vec![Value::String(String::new())])])
        {
            return Err(format!(
                "{at}: неподдерживаемый заголовок локального вызова"
            ));
        }
        parameters.push(variable.name.as_str());
    }
    if open_bsl::folded_eq(&image.procedures[0].name, &image.procedures[1].name) {
        return Err("image/Proc: повторное имя процедуры".into());
    }
    let numbers = constant_literals(&image.constants)?;
    let (caller_range, callee_range, kind, end_kind) = if local_function {
        (0..6, 10..19, "Функция", "КонецФункции")
    } else {
        (0..7, 11..18, "Процедура", "КонецПроцедуры")
    };
    let caller = recover_expressions(caller_range, Some(parameters[0]), &numbers, false, image)?;
    let callee = recover_expressions(callee_range, Some(parameters[1]), &numbers, false, image)?;
    // Внешний вызов процедуры с флагом `16` измерен отдельно;
    // закрытая процедура сохраняет прежнюю видимость.
    let export = if exported_procedure {
        " Экспорт"
    } else {
        ""
    };
    Ok(format!(
        "Функция {}({}) Экспорт\n{caller}\nКонецФункции\n\n{kind} {}({}){export}\n{callee}\n{end_kind}\n",
        image.procedures[0].name, parameters[0], image.procedures[1].name, parameters[1]
    ))
}

fn parameter_source(
    variable: &Variable,
    default: &Value,
    at: &str,
    slot: usize,
) -> Result<String, String> {
    let prefix = match variable.flags {
        5 => "",
        13 => "Знач ",
        _ => return Err(format!("{at}/Var[{slot}]: неподдерживаемый параметр")),
    };
    let mut parameter = format!("{prefix}{}", variable.name);
    if *default != Value::List(vec![Value::String(String::new())]) {
        let literal = if *default == Value::List(vec![Value::String("U".into())]) {
            // Проба локального вызова без аргумента измеряет
            // именно `DefPrm U`, а не константу иной таблицы.
            "Неопределено".to_owned()
        } else if *default == Value::List(vec![Value::String("B".into()), Value::Atom("1".into())])
        {
            // `measure-native-image-boolean-defaults` измеряет
            // булевы значения только в `DefPrm`, не в `Const`.
            "Истина".to_owned()
        } else if *default == Value::List(vec![Value::String("B".into()), Value::Atom("0".into())])
        {
            "Ложь".to_owned()
        } else {
            constant_literals(std::slice::from_ref(default))
                .map_err(|error| format!("{at}/DefPrm[{slot}]: {error}"))?[0]
                .clone()
        };
        // Значение по умолчанию в объявлении 1С — константа,
        // а не выражение в скобках, как в теле метода.
        let literal = literal
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .unwrap_or(&literal);
        parameter.push_str(&format!(" = {literal}"));
    }
    Ok(parameter)
}

fn format_method_source(
    procedure: &Procedure,
    parameters: &[String],
    names: &[&str],
    body: &str,
) -> String {
    let parameter_count = parameters.len();
    let is_function = matches!(procedure.flags, 1 | 3 | 17 | 19);
    let locals = if parameter_count < names.len() {
        format!("Перем {};\n", names[parameter_count..].join(", "))
    } else {
        String::new()
    };
    let (kind, end_kind) = if is_function {
        ("Функция", "КонецФункции")
    } else {
        ("Процедура", "КонецПроцедуры")
    };
    let export = if matches!(procedure.flags, 16..=19) {
        " Экспорт"
    } else {
        ""
    };
    let asynchronous = if procedure.flags == 3 {
        "Асинх "
    } else {
        ""
    };
    format!(
        "{asynchronous}{kind} {}({}){export}\n{locals}{body}\n{end_kind}\n",
        procedure.name,
        parameters.join(", ")
    )
}

fn recover_linear_module(image: &Image) -> Result<String, String> {
    recover_linear_module_with_scoped_bodies(image, false)
}

fn recover_linear_module_with_scoped_bodies(
    image: &Image,
    scoped_bodies: bool,
) -> Result<String, String> {
    // Контроли `private-procedure-eval-outcome` и `private-procedure-execute`
    // измеряют команды 109 и 52. Внешнее объявление с именем `Execute`
    // не подменяем ими: оно требует отдельного нативного контроля.
    if image
        .procedures
        .iter()
        .any(|procedure| procedure.flags == 2)
        && image.procedures.iter().any(|procedure| {
            matches!(procedure.flags, 32 | 33)
                && (open_bsl::folded_eq(&procedure.name, "Выполнить")
                    || open_bsl::folded_eq(&procedure.name, "Execute"))
        })
    {
        return Err(
            "image: динамический вызов частной процедуры с флагом 2 пока не поддерживается".into(),
        );
    }
    let initializer = if image.entry == u32::MAX {
        None
    } else {
        let entry = image.entry as usize;
        // `measure-native-image-module-initializer` проверяет тело после
        // обычных методов и отдельное завершение инициализации.
        if entry >= image.instructions.len()
            || image.procedures.iter().any(|procedure| {
                matches!(procedure.flags, 0 | 1 | 2 | 16 | 17) && procedure.entry >= image.entry
            })
            || image.instructions.last()
                != Some(&Instruction {
                    code: 22,
                    operand: 0,
                })
        {
            return Err("image: неподдерживаемые границы инициализации модуля".into());
        }
        Some(entry)
    };
    for (index, procedure) in image.procedures.iter().enumerate() {
        if matches!(procedure.flags, 0 | 1 | 2 | 16 | 17)
            && image.procedures[..index].iter().any(|previous| {
                matches!(previous.flags, 0 | 1 | 2 | 16 | 17) && previous.entry == procedure.entry
            })
        {
            return Err(format!("image/Proc[{index}]: повторный адрес тела метода"));
        }
    }
    for (index, variable) in image.variables.iter().enumerate() {
        // Внешние имена с флагом `2` измерены нативной пробой глобальной
        // ссылки. Их связывает существующий резолвер. Проба инициализатора
        // отдельно проверяет частную модульную переменную с флагом `0`.
        if !matches!(variable.flags, 0 | 2)
            || (variable.flags == 0 && initializer.is_none())
            || variable.external_slot != -1
            || !valid_name(&variable.name)
            || image.variables[..index]
                .iter()
                .any(|previous| open_bsl::folded_eq(&previous.name, &variable.name))
        {
            return Err(format!(
                "image/Var[{index}]: неподдерживаемое глобальное имя"
            ));
        }
    }
    if image
        .procedures
        .iter()
        .filter(|procedure| matches!(procedure.flags, 0 | 1 | 2 | 16 | 17))
        .map(|procedure| procedure.entry)
        .min()
        != Some(0)
    {
        return Err("image/Cmd: неподдерживаемое начало линейного модуля".into());
    }
    let constants = constant_literals(&image.constants)?;
    let mut methods = Vec::new();
    for (index, procedure) in image.procedures.iter().enumerate() {
        let at = format!("image/Proc[{index}]");
        if !matches!(procedure.flags, 0 | 1 | 2 | 3 | 16 | 17 | 32 | 33)
            || !valid_name(&procedure.name)
        {
            return Err(format!("{at}: неподдерживаемое объявление метода"));
        }
        if image.procedures[..index]
            .iter()
            .any(|previous| open_bsl::folded_eq(&previous.name, &procedure.name))
        {
            return Err(format!("{at}: повторное имя метода"));
        }
        if matches!(procedure.flags, 32 | 33) {
            // Нативная проба внешних методов измеряет записи без тела.
            // Сохраняем их индексы для вызовов, но не создаем заглушек:
            // имя связывает обычный резолвер с функцией runtime или хоста.
            if procedure.entry != 0 || procedure.variables.is_some() || procedure.defaults.is_some()
            {
                return Err(format!("{at}: неподдерживаемое объявление внешнего метода"));
            }
            continue;
        }
        if let Some(source) = measured_pure_private_return(image, procedure)
            .or_else(|| measured_numeric_private_write(image, procedure))
        {
            methods.push(source);
            continue;
        }
        if scoped_bodies
            && let Some(source) = scoped_functions::recover_private_method(image, index, &constants)
        {
            // Контекстный контроль сохраняет тот же частный кадр вместе
            // с глобальными и внешними именами. Весь состав проверяется
            // до выдачи исходника; неизвестные объявления не пропускаются.
            methods.push(source?);
            continue;
        }
        let variables = procedure.variables.as_deref().unwrap_or_default();
        let parameter_count = procedure.parameter_count as usize;
        let defaults = procedure.defaults.as_deref().unwrap_or_default();
        if parameter_count > variables.len() || defaults.len() != parameter_count {
            return Err(format!(
                "{at}: таблицы параметров не соответствуют объявлению"
            ));
        }
        let mut parameters = Vec::new();
        for (slot, variable) in variables.iter().enumerate() {
            let iterator_slot = variable.flags == 1
                && variable.name.strip_prefix('0').is_some_and(|name| {
                    variables
                        .iter()
                        .any(|other| other.name == name && other.flags == 1)
                });
            if (!valid_name(&variable.name) && !iterator_slot)
                // `parameter-slot-zero` измеряет параметр по значению,
                // `private-variable-slot-only` — ссылочный параметр частной
                // процедуры с флагом 2. Локальные слоты этим не разрешаются.
                || (variable.external_slot != -1
                    && !(slot < parameter_count
                        && (variable.flags == 13
                            || (variable.flags == 5 && procedure.flags == 2))
                        && variable.external_slot == 0))
                || variables[..slot]
                    .iter()
                    .any(|previous| open_bsl::folded_eq(&previous.name, &variable.name))
            {
                return Err(format!("{at}/Var[{slot}]: неподдерживаемое локальное имя"));
            }
            if slot < parameter_count {
                parameters.push(parameter_source(variable, &defaults[slot], &at, slot)?);
            } else if variable.flags != 1 {
                return Err(format!(
                    "{at}/Var[{slot}]: неподдерживаемая локальная переменная"
                ));
            }
        }
        let local_names: Vec<_> = variables
            .iter()
            .enumerate()
            .map(|(slot, variable)| {
                if valid_name(&variable.name) {
                    variable.name.clone()
                } else {
                    temporary_name(image, image.instructions.len() + slot)
                }
            })
            .collect();
        let names: Vec<_> = local_names.iter().map(String::as_str).collect();
        if scoped_bodies {
            if !matches!(procedure.flags, 0 | 1 | 16 | 17) {
                return Err(format!(
                    "{at}: неподдерживаемый частный кадр при восстановлении блоков"
                ));
            }
            let body = scoped_functions::recover_blocks(image, index, &names, &constants, false)
                .map_err(|error| format!("{at}: {error}"))?;
            methods.push(format_method_source(procedure, &parameters, &names, &body));
            continue;
        }
        let shared_body = shared_reference_body(image, procedure.entry);
        let start = shared_body.map_or(procedure.entry as usize, |(start, _)| start);
        let end = method_end(image, procedure.entry);
        if start >= end || end > image.instructions.len() {
            return Err(format!("{at}: неверные границы тела"));
        }
        if procedure.flags == 3 {
            // Нативный `local-async-eval` измеряет частную функцию без
            // параметров и эффектов. Формы со ссылками нельзя переводить
            // в `Асинх`: их поведение отличается от исходного async-BSL.
            if parameter_count != 0
                || !variables.is_empty()
                || !defaults.is_empty()
                || !async_constant_tail(image, start, end)
            {
                return Err(format!(
                    "{at}: неподдерживаемое тело частной асинхронной функции"
                ));
            }
        }
        let is_function = matches!(procedure.flags, 1 | 3 | 17);
        let has_loops = image.instructions[start..end]
            .iter()
            .any(|instruction| instruction.code == 48);
        let has_branches = !has_loops
            && image.instructions[start..end]
                .iter()
                .any(|instruction| matches!(instruction.code, 40 | 41));
        let has_try = image.instructions[start].code == 44;
        let has_statement_try = image.instructions[start..end]
            .iter()
            .any(|instruction| instruction.code == 44);
        // Нативная проба допускает отсутствие номера строки перед `End`.
        // Адрес выхода берётся из фактического хвоста, а переход проверяется точно.
        let exit = if end - start >= 2
            && image.instructions[end - 2].code == 1
            && image.instructions[end - 2].operand > 0
        {
            end - 2
        } else {
            end - 1
        };
        let has_terminal_return = exit - start >= 3
            && image.instructions[exit - 3]
                == (Instruction {
                    code: 17,
                    operand: 0,
                });
        let body_end = if is_function && has_try {
            end
        } else if is_function && has_branches && !has_terminal_return {
            if end - start < 3
                || image.instructions[end - 1]
                    != (Instruction {
                        code: 22,
                        operand: 0,
                    })
                || image.instructions[end - 2].code != 1
                || image.instructions[end - 2].operand <= 0
                || image.instructions[end - 3].code != 1
                || image.instructions[end - 3].operand <= 0
            {
                return Err(format!("{at}: неподдерживаемое завершение ветвления"));
            }
            end - 3
        } else if is_function {
            if exit - start < 3 {
                return Err(format!("{at}: отсутствует завершение функции"));
            }
            if image.instructions[exit - 2]
                != (Instruction {
                    code: 45,
                    operand: 0,
                })
                || (image.instructions[exit - 1]
                    != (Instruction {
                        code: 39,
                        operand: exit as i64,
                    })
                    && !(exit == end - 2
                        && image.instructions[exit - 1]
                            == (Instruction {
                                code: 39,
                                operand: (end - 1) as i64,
                            })
                        && image.procedures.iter().any(|method| {
                            method.entry as usize == exit
                                && method.flags == 2
                                && method.parameter_count == 0
                                && method.variables.as_ref().is_none_or(Vec::is_empty)
                                && method.defaults.as_ref().is_none_or(Vec::is_empty)
                        })))
                || image.instructions[end - 1]
                    != (Instruction {
                        code: 22,
                        operand: 0,
                    })
                || !has_terminal_return
                || (!has_branches
                    && !has_loops
                    && image.instructions[start..exit - 3]
                        .iter()
                        .any(|instruction| instruction.code == 17))
            {
                return Err(format!(
                    "{at}: неподдерживаемое завершение линейной функции"
                ));
            }
            exit - 2
        } else if shared_body.is_some() {
            // В измеренной процедуре возврат читает только её параметр.
            // Эффекты присваивания сохраняются, чтение без эффекта опускается.
            exit - 4
        } else {
            if image.instructions[end - 1]
                != (Instruction {
                    code: 22,
                    operand: 0,
                })
            {
                return Err(format!("{at}: отсутствует завершение процедуры"));
            }
            if image.instructions[start..end - 1]
                .iter()
                .any(|instruction| instruction.code == 17)
            {
                return Err(format!("{at}: неподдерживаемый возврат линейной процедуры"));
            }
            end - 1
        };
        let body_result = if has_try && is_function {
            recover_caught_return(start..end, &names, &constants, image).or_else(|error| {
                recover_structured_caught_return(start..end, &names, &constants, image)
                    .or(Err(error))
            })
        } else if has_loops {
            recover_loop_sequence(start..body_end, &names, &constants, image)
        } else if has_branches || has_statement_try {
            if !is_function || has_terminal_return {
                recover_statement_branches(start..body_end, &names, &constants, image)
            } else {
                recover_return_branches(start..body_end, end - 2, &names, &constants, image)
            }
        } else {
            recover_variable_expressions(start..body_end, &names, &constants, false, image)
        };
        let body = body_result
            .or_else(|error| {
                if !is_function {
                    recover_labelled_statements(start..body_end, &names, &constants, image)
                        .or(Err(error))
                } else if has_try
                    && has_terminal_return
                    && image.instructions[start].operand == (start + 3) as i64
                    && image.instructions.get(start + 1)
                        == Some(&Instruction {
                            code: 45,
                            operand: 1,
                        })
                    && image.instructions.get(start + 2)
                        == Some(&Instruction {
                            code: 39,
                            operand: (start + 4) as i64,
                        })
                    && image.instructions.get(start + 3)
                        == Some(&Instruction {
                            code: 46,
                            operand: 0,
                        })
                    && image.instructions[exit - 2]
                        == (Instruction {
                            code: 45,
                            operand: 0,
                        })
                    && image.instructions[exit - 1]
                        == (Instruction {
                            code: 39,
                            operand: exit as i64,
                        })
                    && image.instructions[end - 1]
                        == (Instruction {
                            code: 22,
                            operand: 0,
                        })
                {
                    // `measure-native-image-function-guard` сохраняет пустую
                    // попытку перед возвратом. Проверенный хвост функции
                    // не становится отдельным блоком исполнения.
                    recover_labelled_statements(start..exit - 2, &names, &constants, image)
                        .or(Err(error))
                } else {
                    Err(error)
                }
            })
            .map_err(|error| format!("{at}: {error}"))?;
        methods.push(format_method_source(procedure, &parameters, &names, &body));
    }
    let declarations = image
        .variables
        .iter()
        .filter(|variable| variable.flags == 0)
        .map(|variable| format!("Перем {};\n", variable.name))
        .collect::<String>();
    let initialization = match initializer {
        Some(entry) => {
            let mut end = image.instructions.len() - 1;
            // Нативный `initializer-double-end` измеряет отдельные окончания
            // инициализации и модуля. Внутренние команды не отбрасываются.
            if end > entry
                && image.instructions[end - 1]
                    == (Instruction {
                        code: 22,
                        operand: 0,
                    })
            {
                end -= 1;
            }
            recover_statement_branches(entry..end, &[], &constants, image)
                .or_else(|error| {
                    if scoped_bodies {
                        scoped_functions::recover_initializer(image, &constants).or(Err(error))
                    } else {
                        Err(error)
                    }
                })
                .map_err(|error| format!("image/инициализация: {error}"))?
        }
        None => String::new(),
    };
    // Ни один метод не выдаётся до проверки всех тел и инициализатора.
    Ok(format!(
        "{declarations}{}{initialization}",
        methods.join("\n")
    ))
}

// `shared-reference-body` измеряет два входа с одинаковым ссылочным кадром.
// `shared-try-body` дополнительно измеряет два пустых обработчика перед телом.
// `shared-unused-frame` измеряет разные обычные кадры с доступом только к слоту 0.
// Распознаём целую форму, чтобы чужое объявление не расширяло произвольное тело.
fn shared_reference_body(image: &Image, entry: u32) -> Option<(usize, usize)> {
    for function in &image.procedures {
        if !matches!(function.flags, 1 | 17)
            || function.parameter_count != 1
            || !function.variables.as_ref().is_some_and(|variables| {
                matches!(variables.len(), 1 | 5) && variables[0].flags == 5
            })
        {
            continue;
        }
        let guarded = image
            .instructions
            .get(function.entry as usize)
            .is_some_and(|command| {
                *command
                    == Instruction {
                        code: 44,
                        operand: function.entry as i64 + 3,
                    }
            });
        let private_entry = function.entry.checked_add(if guarded { 4 } else { 1 })?;
        let Some(private) = image.procedures.iter().find(|method| {
            method.entry == private_entry
                && method.flags == 2
                && method.parameter_count == 1
                && ((method.variables == function.variables
                    && function
                        .variables
                        .as_ref()
                        .is_some_and(|variables| variables.len() == 1))
                    || (guarded && measured_unused_frames(function, method)))
                && method.defaults == function.defaults
        }) else {
            continue;
        };
        if entry != function.entry && entry != private.entry {
            continue;
        }
        let start = private_entry as usize + if guarded { 4 } else { 2 };
        let end = start.checked_add(11)?;
        let prefix = image.instructions.get(function.entry as usize..start)?;
        let body = image.instructions.get(start..end)?;
        let prefix_matches = if guarded {
            prefix.iter().enumerate().all(|(offset, command)| {
                let base = function.entry as usize + (offset / 4) * 4;
                *command
                    == match offset % 4 {
                        0 => Instruction {
                            code: 44,
                            operand: (base + 3) as i64,
                        },
                        1 => Instruction {
                            code: 45,
                            operand: 1,
                        },
                        2 => Instruction {
                            code: 39,
                            operand: (base + 4) as i64,
                        },
                        _ => Instruction {
                            code: 46,
                            operand: 0,
                        },
                    }
            })
        } else {
            prefix[0]
                == (Instruction {
                    code: 39,
                    operand: start as i64,
                })
                && prefix[1].code == 1
                && prefix[1].operand > 0
                && prefix[2]
                    == (Instruction {
                        code: 39,
                        operand: start as i64,
                    })
        };
        if !prefix_matches
            || body[2].code != 4
            || body[2].operand < 0
            || !image.constants.get(body[2].operand as usize).is_some_and(|constant| {
                matches!(constant, Value::List(fields) if matches!(fields.as_slice(), [Value::String(tag), Value::Atom(_)] if tag == "N"))
            })
            || body[9].code != 1 || body[9].operand <= 0
        { continue; }
        let expected = [
            (0, 3, 0),
            (1, 3, 0),
            (3, 24, 0),
            (4, 16, 0),
            (5, 3, 0),
            (6, 17, 0),
            (7, 45, 0),
            (8, 39, (end - 2) as i64),
            (10, 22, 0),
        ];
        if expected
            .iter()
            .any(|&(index, code, operand)| body[index] != (Instruction { code, operand }))
            || image.procedures.iter().any(|method| {
                matches!(method.flags, 0 | 1 | 2 | 3 | 16 | 17)
                    && method.entry != function.entry
                    && method.entry != private.entry
                    && method.entry >= function.entry
                    && (method.entry as usize) < end
            })
            || (image.entry != u32::MAX
                && (function.entry as usize..end).contains(&(image.entry as usize)))
        {
            continue;
        }
        return Some((start, end));
    }
    None
}

fn method_end(image: &Image, entry: u32) -> usize {
    if let Some((_, end)) = shared_reference_body(image, entry) {
        return end;
    }
    // Таблица методов не упорядочена по адресам; внешние объявления
    // не имеют собственного тела и не задают границу соседнего метода.
    let end = image
        .procedures
        .iter()
        .filter(|next| matches!(next.flags, 0 | 1 | 2 | 3 | 16 | 17) && next.entry > entry)
        .map(|next| next.entry as usize)
        // Тело инициализации не становится продолжением последнего метода.
        .chain((image.entry != u32::MAX && image.entry > entry).then_some(image.entry as usize))
        .min()
        .unwrap_or_else(|| {
            let end = image.instructions.len();
            // Независимый компиляторный контроль заканчивается отдельными
            // `End` метода и модуля. Общая граница учитывает оба, а не только
            // частный случай функции с обработчиком исключения.
            if end >= 2
                && image.instructions[end - 2..].iter().all(|instruction| {
                    *instruction
                        == Instruction {
                            code: 22,
                            operand: 0,
                        }
                })
            {
                end - 1
            } else {
                end
            }
        });
    // `shared-tail-both` измеряет частный вход на строке общего `End`.
    // Сохраняем оба объявления; расширяется только измеренный хвост возврата.
    if end >= entry as usize + 3
        && image.procedures.iter().any(|method| {
            method.entry as usize == end
                && method.flags == 2
                && method.parameter_count == 0
                && method.variables.as_ref().is_none_or(Vec::is_empty)
                && method.defaults.as_ref().is_none_or(Vec::is_empty)
        })
        && image.instructions.get(end - 3..end).is_some_and(|tail| {
            tail == [
                Instruction {
                    code: 17,
                    operand: 0,
                },
                Instruction {
                    code: 45,
                    operand: 0,
                },
                Instruction {
                    code: 39,
                    operand: (end + 1) as i64,
                },
            ]
        })
        && image
            .instructions
            .get(end)
            .is_some_and(|line| line.code == 1 && line.operand > 0)
        && image.instructions.get(end + 1)
            == Some(&Instruction {
                code: 22,
                operand: 0,
            })
        && !image
            .procedures
            .iter()
            .any(|method| method.entry as usize == end + 1)
        && !(image.entry != u32::MAX && (end..end + 2).contains(&(image.entry as usize)))
    {
        return end + 2;
    }
    // Нативный `overlapping-constant-tail` проверяет общий хвост обычной
    // и частной async-функции. Вход второго метода не обрезает первый.
    let shared_end = end.checked_add(5);
    let ordinary = image.procedures.iter().any(|method| {
        method.entry == entry
            && matches!(method.flags, 1 | 17)
            && method.parameter_count == 0
            && method.variables.as_ref().is_none_or(Vec::is_empty)
            && method.defaults.as_ref().is_none_or(Vec::is_empty)
    });
    let private = image.procedures.iter().any(|method| {
        method.entry as usize == end
            && method.flags == 3
            && method.parameter_count == 0
            && method.variables.as_ref().is_none_or(Vec::is_empty)
            && method.defaults.as_ref().is_none_or(Vec::is_empty)
    });
    let start = entry as usize;
    if ordinary
        && private
        && start < end
        && (image.instructions.get(start)
            == Some(&Instruction {
                code: 39,
                operand: end as i64,
            })
            || image.instructions.get(start..end).is_some_and(|prefix| {
                prefix.iter().all(|command| {
                    *command
                        == Instruction {
                            code: 1,
                            operand: 1,
                        }
                })
            }))
        && let Some(shared_end) = shared_end
        && async_constant_tail(image, end, shared_end)
        && !image
            .procedures
            .iter()
            .any(|method| method.entry as usize > end && (method.entry as usize) < shared_end)
        && !(image.entry != u32::MAX
            && image.entry as usize >= end
            && (image.entry as usize) < shared_end)
    {
        shared_end
    } else {
        end
    }
}

fn async_constant_tail(image: &Image, start: usize, end: usize) -> bool {
    let Some(body) = image.instructions.get(start..end) else {
        return false;
    };
    body.len() == 5
        && body[0].code == 4
        && body[1]
            == Instruction {
                code: 17,
                operand: 0,
            }
        && body[2]
            == Instruction {
                code: 45,
                operand: 0,
            }
        && body[3]
            == Instruction {
                code: 39,
                operand: (end - 1) as i64,
            }
        && body[4]
            == Instruction {
                code: 22,
                operand: 0,
            }
}

fn constant_literals(values: &[Value]) -> Result<Vec<String>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let at = format!("image/Const[{index}]");
            let fields = sequence(value, &at)?;
            if let [Value::String(tag), Value::String(text)] = fields
                && tag == "S"
            {
                // В `measure-native-image-string` платформа сворачивает
                // CRLF в LF, но сохраняет одиночный CR. Управляющие символы
                // печатаются вызовами `Символ`, чтобы лексер BSL не менял их.
                let text = text
                    .replace("\r\n", "\n")
                    .replace('"', "\"\"")
                    .replace('\r', "\" + Символ(13) + \"")
                    .replace('\n', "\" + Символ(10) + \"");
                return Ok(format!("(\"{text}\")"));
            }
            let [Value::String(tag), Value::Atom(number)] = fields else {
                return Err(format!("{at}: неподдерживаемая форма числовой константы"));
            };
            if tag != "N" {
                if tag == "D" && number.len() == 14 {
                    open_bsl::BslDate::parse_digits(number)
                        .ok_or_else(|| format!("{at}: некорректная дата"))?;
                    // Значение и тип сверены с изолированным телом
                    // исходного buh в `measure-native-image-date`.
                    return Ok(format!("'{number}'"));
                }
                return Err(format!("{at}: неподдерживаемый тип константы"));
            }
            let number = open_bsl::BslNumber::parse_canonical(number)
                .map_err(|error| format!("{at}: {error}"))?;
            Ok(number.to_canonical())
        })
        .collect::<Result<Vec<_>, String>>()
}

fn recover_expressions(
    range: std::ops::Range<usize>,
    parameter: Option<&str>,
    numbers: &[String],
    condition: bool,
    image: &Image,
) -> Result<String, String> {
    recover_variable_expressions(range, parameter.as_slice(), numbers, condition, image)
}

// Нативный `procedure-shared-loop` и независимый BSL с метками проверяют
// общую область цикла. Это текст для существующего компилятора, а не
// исполнение нативных инструкций; выражения восстанавливает тот же код.
fn backward_boolean_join(image: &Image, index: usize) -> Option<usize> {
    let command = image.instructions.get(index)?;
    let target = usize::try_from(command.operand).ok()?;
    let exact = |at: usize, code, operand| {
        image.instructions.get(at) == Some(&Instruction { code, operand })
    };
    if command.code != 40
        || target == 0
        || target + 2 >= index
        || !exact(target, 6, 0)
        || !exact(target + 1, 16, 0)
        || !exact(index + 1, 7, 0)
        || !exact(index + 2, 39, (target + 1) as i64)
        || image.instructions[target - 1].code != 39
    {
        return None;
    }
    let procedure = image
        .procedures
        .iter()
        .filter(|procedure| {
            matches!(procedure.flags, 0 | 2 | 16) && procedure.entry as usize <= index
        })
        .max_by_key(|procedure| procedure.entry)?;
    if target < procedure.entry as usize || index + 3 >= method_end(image, procedure.entry) {
        return None;
    }
    let protected =
        |entry| entry == target || entry == target + 1 || entry == index + 1 || entry == index + 2;
    if protected(image.entry as usize)
        || image
            .procedures
            .iter()
            .any(|procedure| protected(procedure.entry as usize))
        || image
            .instructions
            .iter()
            .enumerate()
            .any(|(source, command)| {
                matches!(command.code, 39..=44)
                    && usize::try_from(command.operand).is_ok_and(protected)
                    && !((source == index
                        && command.code == 40
                        && command.operand == target as i64)
                        || source == index + 2)
            })
    {
        return None;
    }
    Some(target)
}

fn recover_labelled_statements(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
) -> Result<String, String> {
    let commands = image
        .instructions
        .get(range.clone())
        .ok_or_else(|| "image/Cmd: неверная область блоков".to_owned())?;
    // Измеренный пустой обработчик остаётся обычной `Попытка`: контекст
    // исключения обслуживает существующий runtime. Внутрь блока входить нельзя.
    let mut guards = Vec::new();
    let mut index = range.start;
    while index < range.end {
        let command = &image.instructions[index];
        if command.code == 44
            && index + 4 <= range.end
            && command.operand == (index + 3) as i64
            && image.instructions[index + 1].code == 45
            && image.instructions[index + 1].operand == 1
            && image.instructions[index + 2].code == 39
            && image.instructions[index + 2].operand == (index + 4) as i64
            && image.instructions[index + 3].code == 46
            && image.instructions[index + 3].operand == 0
        {
            guards.push(index..index + 4);
            index += 4;
        } else if matches!(command.code, 22 | 44 | 46 | 48..=50) {
            return Err("image/Cmd: неподдерживаемая область блоков".into());
        } else {
            index += 1;
        }
    }
    let interior = |target: usize| {
        guards
            .iter()
            .any(|guard| target > guard.start && target < guard.end)
    };
    if interior(image.entry as usize)
        || image
            .procedures
            .iter()
            .any(|procedure| interior(procedure.entry as usize))
    {
        return Err("image/Cmd: вход в пустой обработчик".into());
    }
    let mut leaders = vec![range.start, range.end];
    let joins: Vec<_> = range
        .clone()
        .filter_map(|index| {
            backward_boolean_join(image, index)
                .filter(|&target| target >= range.start && index + 3 <= range.end)
                .map(|target| (index, target))
        })
        .collect();
    for &(_, target) in &joins {
        leaders.extend([target, target + 2]);
    }
    for guard in &guards {
        leaders.extend([guard.start, guard.end]);
    }
    for (offset, command) in commands.iter().enumerate() {
        let index = range.start + offset;
        if guards.iter().any(|guard| guard.contains(&index)) {
            continue;
        }
        if joins.iter().any(|&(branch, _)| branch == index) {
            continue;
        }
        if matches!(command.code, 39..=41) {
            let target = usize::try_from(command.operand)
                .ok()
                .filter(|&target| target >= range.start && target <= range.end && !interior(target))
                .ok_or_else(|| format!("image/Cmd[{index}]: неверная цель блока"))?;
            let target = joins
                .iter()
                .find(|&&(branch, _)| branch + 2 == index)
                .map_or(target, |&(_, target)| target + 2);
            leaders.extend([target, index + 1]);
        }
    }
    leaders.sort_unstable();
    leaders.dedup();
    let label = |index| format!("МеткаОбраза_{index}");
    let mut output = Vec::new();
    for pair in leaders.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        output.push(format!("~{}:", label(start)));
        if joins
            .iter()
            .any(|&(_, target)| start == target && end == target + 2)
        {
            continue;
        }
        if guards
            .iter()
            .any(|guard| guard.start == start && guard.end == end)
        {
            output.push("Попытка\nИсключение\nКонецПопытки;".to_owned());
            continue;
        }
        let last = &image.instructions[end - 1];
        if matches!(last.code, 40 | 41) {
            let mut prelude = Vec::new();
            let condition = recover_expression_sequence_with_prelude(
                start..end - 1,
                variables,
                constants,
                true,
                false,
                image,
                Some(&mut prelude),
            )?;
            output.extend(prelude);
            let condition = if last.code == 40 {
                format!("НЕ ({condition})")
            } else {
                condition
            };
            output.push(format!(
                "Если {condition} Тогда\nПерейти ~{};\nКонецЕсли;",
                label(last.operand as usize)
            ));
        } else {
            let body_end = if last.code == 39 { end - 1 } else { end };
            output.push(recover_variable_expressions(
                start..body_end,
                variables,
                constants,
                false,
                image,
            )?);
            if last.code == 39 {
                let target = joins
                    .iter()
                    .find(|&&(branch, _)| branch + 2 == end - 1)
                    .map_or(last.operand as usize, |&(_, target)| target + 2);
                output.push(format!("Перейти ~{};", label(target)));
            }
        }
    }
    output.push(format!("~{}:", label(range.end)));
    Ok(output.join("\n"))
}

fn recover_statement_branches(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
) -> Result<String, String> {
    recover_statement_branches_with_loop_exits(range, variables, constants, image, None)
}

fn recover_statement_branches_with_loop_exits(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
    loop_targets: Option<(usize, usize)>,
) -> Result<String, String> {
    let mut search_start = range.start;
    let branch = loop {
        let Some(candidate) = (search_start..range.end)
            .find(|&index| matches!(image.instructions[index].code, 40 | 41 | 44 | 48))
        else {
            break None;
        };
        if matches!(image.instructions[candidate].code, 40 | 41)
            && let Ok(alternative) = usize::try_from(image.instructions[candidate].operand)
            && alternative > candidate + 1
            && alternative <= range.end
            && image.instructions[alternative - 1].code == 39
            && let Ok(join) = usize::try_from(image.instructions[alternative - 1].operand)
            && join > alternative
            && join <= range.end
            && recover_expression_sequence(
                candidate + 1..alternative - 1,
                variables,
                constants,
                true,
                true,
                image,
            )
            .is_ok()
            && recover_expression_sequence(
                alternative..join,
                variables,
                constants,
                true,
                true,
                image,
            )
            .is_ok()
        {
            // `measure-native-image-mixed-conditional` сочетает присваивание
            // `?(...)` с последующим `Если`. Чистые ветви оставляют значение,
            // поэтому продолжаем поиск операторного блока за их слиянием.
            // Само выражение восстановится в префиксе без перестановки команд.
            search_start = join;
            continue;
        }
        break Some(candidate);
    };
    let Some(branch) = branch else {
        if let Some((continue_target, break_target)) = loop_targets
            && let Some(jump) = (range.start..range.end).find(|&index| {
                image.instructions[index].code == 39
                    && (image.instructions[index].operand == continue_target as i64
                        || image.instructions[index].operand == break_target as i64)
            })
        {
            // Пробы `measure-native-image-foreach-continue` и `foreach-break`
            // сопоставляют переходы к концу итерации и очистке ближайшего
            // обхода с `Продолжить` и `Прервать`. Не скрываем неизвестное
            // тело после перехода как якобы недостижимое.
            if image.instructions[jump + 1..range.end]
                .iter()
                .any(|instruction| instruction.code != 1 || instruction.operand <= 0)
            {
                return Err(format!(
                    "image/Cmd[{jump}]: неподдерживаемое тело после выхода из итерации"
                ));
            }
            let prefix = recover_variable_expressions(
                range.start..jump,
                variables,
                constants,
                false,
                image,
            )?;
            let keyword = if image.instructions[jump].operand == continue_target as i64 {
                "Продолжить"
            } else {
                "Прервать"
            };
            return Ok(format!("{prefix}\n{keyword};"));
        }
        return recover_variable_expressions(range, variables, constants, false, image);
    };
    if image.instructions[branch].code == 44 {
        let at = format!("image/Cmd[{branch}]");
        let handler = usize::try_from(image.instructions[branch].operand)
            .ok()
            .filter(|&handler| handler > branch + 2 && handler < range.end)
            .ok_or_else(|| format!("{at}: неверная граница обработчика"))?;
        let close = &image.instructions[handler - 2];
        let jump = &image.instructions[handler - 1];
        let join = usize::try_from(jump.operand)
            .ok()
            .filter(|&join| jump.code == 39 && join > handler && join <= range.end)
            .ok_or_else(|| format!("{at}: неверное продолжение попытки"))?;
        if close
            != &(Instruction {
                code: 45,
                operand: 1,
            })
            || image.instructions[join - 1]
                != (Instruction {
                    code: 46,
                    operand: 0,
                })
        {
            return Err(format!("{at}: неподдерживаемое завершение попытки"));
        }
        // Проба `measure-native-image-statement-try` проверяет обе ветви,
        // снимок ошибки и продолжение после блока. Исключения исполняет
        // обычная VM после компиляции восстановленных операторов BSL.
        let prefix = recover_statement_branches_with_loop_exits(
            range.start..branch,
            variables,
            constants,
            image,
            loop_targets,
        )?;
        let body = recover_statement_branches_with_loop_exits(
            branch + 1..handler - 2,
            variables,
            constants,
            image,
            loop_targets,
        )?;
        let caught = recover_statement_branches_with_loop_exits(
            handler..join - 1,
            variables,
            constants,
            image,
            loop_targets,
        )?;
        let suffix = recover_statement_branches_with_loop_exits(
            join..range.end,
            variables,
            constants,
            image,
            loop_targets,
        )?;
        return Ok(format!(
            "{prefix}\nПопытка\n{body}\nИсключение\n{caught}\nКонецПопытки;\n{suffix}"
        ));
    }
    if image.instructions[branch].code == 48 {
        return recover_loop_sequence_with_loop_exits(
            range,
            variables,
            constants,
            image,
            loop_targets,
        );
    }
    let at = format!("image/Cmd[{branch}]");
    if image.instructions[branch].code == 40
        && let Some((back_edge, _)) = loop_targets
        && image.instructions[back_edge].code == 39
        && image.instructions[branch].operand == image.instructions[back_edge].operand
        && image.instructions[branch].operand < range.start as i64
    {
        // `foreach-terminal-branches` сверяет ложную ветвь с `Продолжить`.
        // Цель проверки уже подтверждена восстановлением ближайшего цикла.
        let mut prelude = Vec::new();
        let condition = recover_expression_sequence_with_prelude(
            range.start..branch,
            variables,
            constants,
            true,
            false,
            image,
            Some(&mut prelude),
        )?;
        let selected = recover_statement_branches_with_loop_exits(
            branch + 1..range.end,
            variables,
            constants,
            image,
            loop_targets,
        )?;
        prelude.push(format!(
            "Если {condition} Тогда\n{selected}\nИначе\nПродолжить;\nКонецЕсли;"
        ));
        return Ok(prelude.join("\n"));
    }
    let alternative = usize::try_from(image.instructions[branch].operand)
        .ok()
        .filter(|&target| target > branch + 1 && target <= range.end)
        .ok_or_else(|| format!("{at}: неверная граница ветвления"))?;
    let last = &image.instructions[alternative - 1];
    if last.code == 39 && last.operand >= range.start as i64 && last.operand < branch as i64 {
        return recover_loop_sequence_with_loop_exits(
            range,
            variables,
            constants,
            image,
            loop_targets,
        );
    }
    let mut prelude = Vec::new();
    let condition = recover_expression_sequence_with_prelude(
        range.start..branch,
        variables,
        constants,
        true,
        false,
        image,
        Some(&mut prelude),
    );
    let condition = match condition {
        Ok(condition) => condition,
        Err(error) if error.starts_with("image/Cmd: условие должно содержать одно выражение") =>
        {
            // У `?(...)` под условием уже может лежать цель присваивания.
            // Принимаем только проверяемое выражение целиком; обычная ветка
            // с операторами не проходит проверку чистоты обеих альтернатив.
            return recover_variable_expressions(range, variables, constants, false, image)
                .map_err(|expression_error| {
                    format!("{at}: условное выражение: {expression_error}")
                });
        }
        Err(error) => return Err(format!("{at}: условие ветвления: {error}")),
    };
    // `measure-native-image-jump-true` проверяет обратную полярность
    // перехода, ссылочные изменения и ленивость невыбранной ветки.
    let condition = if image.instructions[branch].code == 41 {
        format!("НЕ ({condition})")
    } else {
        condition
    };
    let (yes_end, join) =
        if last.code == 39 && loop_targets.is_some_and(|(_, exit)| last.operand == exit as i64) {
            // Нативный `foreach-compact-break` не содержит лишнего перехода
            // после `Прервать`. Выход оставляется внутри выбранной ветви.
            (alternative, alternative)
        } else if last.code == 39 {
            // Нативный `procedure-dead-join` оставляет после выхода лишний
            // переход внутрь альтернативы. Он не задаёт её границу только
            // при доказанной недостижимости из всех объявленных входов.
            let dead_return_join = alternative >= branch + 4
                && image.instructions[alternative - 3]
                    == (Instruction {
                        code: 45,
                        operand: 0,
                    })
                && image.instructions[alternative - 2].code == 39
                && {
                    let entries: Vec<_> = image
                        .procedures
                        .iter()
                        .map(|procedure| procedure.entry as usize)
                        .chain((image.entry != u32::MAX).then_some(image.entry as usize))
                        .collect();
                    let (reachable, _) = reachable_commands(image, &entries);
                    !reachable[alternative - 1]
                };
            let join = if dead_return_join {
                alternative
            } else {
                usize::try_from(last.operand)
                    .ok()
                    .filter(|&target| target >= alternative && target <= range.end)
                    .ok_or_else(|| format!("{at}: неверная граница продолжения"))?
            };
            (alternative - 1, join)
        } else {
            (alternative, alternative)
        };
    let yes = recover_statement_branches_with_loop_exits(
        branch + 1..yes_end,
        variables,
        constants,
        image,
        loop_targets,
    )?;
    let no = recover_statement_branches_with_loop_exits(
        alternative..join,
        variables,
        constants,
        image,
        loop_targets,
    )?;
    // Ветка продолжает исполнение общего хвоста. Нативная проба
    // присваивает ссылочному параметру разные значения в обеих ветках.
    prelude.push(if no.is_empty() {
        format!("Если {condition} Тогда\n{yes}\nКонецЕсли;")
    } else {
        format!("Если {condition} Тогда\n{yes}\nИначе\n{no}\nКонецЕсли;")
    });
    prelude.push(recover_statement_branches_with_loop_exits(
        join..range.end,
        variables,
        constants,
        image,
        loop_targets,
    )?);
    Ok(prelude.join("\n"))
}

fn recover_structured_caught_return(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
) -> Result<String, String> {
    let at = format!("image/Cmd[{}]", range.start);
    let command = |index: usize, code: u32, operand: i64| {
        image.instructions.get(index) == Some(&Instruction { code, operand })
    };
    let end = if range.end == image.instructions.len()
        && range.len() >= 2
        && command(range.end - 1, 22, 0)
        && command(range.end - 2, 22, 0)
    {
        range.end - 1
    } else {
        range.end
    };
    let exit = end.checked_sub(2).ok_or_else(|| at.clone())?;
    let handler = usize::try_from(image.instructions[range.start].operand)
        .ok()
        .filter(|&handler| handler >= range.start + 3 && handler < exit)
        .ok_or_else(|| format!("{at}: неверная граница обработчика"))?;
    let join = usize::try_from(image.instructions[handler - 1].operand)
        .ok()
        .filter(|&join| join > handler && join < exit)
        .ok_or_else(|| format!("{at}: неверное продолжение попытки"))?;
    if !command(range.start, 44, handler as i64)
        || !command(handler - 2, 45, 1)
        || !command(handler - 1, 39, join as i64)
        || !command(join - 1, 46, 0)
        || !command(exit - 3, 17, 0)
        || !command(exit - 2, 45, 0)
        || !command(exit - 1, 39, exit as i64)
        || image.instructions[exit].code != 1
        || image.instructions[exit].operand <= 0
        || !command(end - 1, 22, 0)
    {
        return Err(format!("{at}: неподдерживаемое завершение попытки"));
    }
    // Настоящий компиляторный контроль `try-return-compiled` различает
    // возврат из тела, возврат обработчика и продолжение после попытки.
    // Убираем только проверенную очистку перед общим выходом функции.
    let part = |mut block: std::ops::Range<usize>, cleanup: i64| {
        if let Some(returned) = block
            .clone()
            .find(|&index| image.instructions[index].code == 17)
        {
            if !command(returned, 17, 0)
                || !command(returned + 1, 45, cleanup)
                || !command(returned + 2, 39, exit as i64)
                || returned + 3 > block.end
                || image.instructions[returned + 3..block.end]
                    .iter()
                    .any(|line| line.code != 1 || line.operand <= 0)
            {
                return Err(format!(
                    "image/Cmd[{returned}]: неподдерживаемый выход из попытки"
                ));
            }
            block.end = returned + 1;
        }
        recover_statement_branches(block, variables, constants, image)
    };
    let body = part(range.start + 1..handler - 2, 1)?;
    let caught = part(handler..join - 1, 0)?;
    let suffix = recover_statement_branches(join..exit - 2, variables, constants, image)?;
    Ok(format!(
        "Попытка\n{body}\nИсключение\n{caught}\nКонецПопытки;\n{suffix}"
    ))
}

fn recover_caught_return(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
) -> Result<String, String> {
    let at = format!("image/Cmd[{}]", range.start);
    let command = |index: usize, code: u32, operand: i64| {
        image.instructions.get(index) == Some(&Instruction { code, operand })
    };
    // Последний метод нативного образа завершен отдельной командой
    // конца модуля. Она не является частью обработчика исключения.
    let end = if range.end == image.instructions.len()
        && range.len() >= 2
        && command(range.end - 1, 22, 0)
        && command(range.end - 2, 22, 0)
    {
        range.end - 1
    } else {
        range.end
    };
    let handler = usize::try_from(image.instructions[range.start].operand)
        .ok()
        .filter(|&handler| handler >= range.start + 3 && handler + 9 == end)
        .ok_or_else(|| format!("{at}: неподдерживаемая граница обработчика"))?;
    let exit = end - 2;
    if !command(range.start, 44, handler as i64)
        || !command(handler - 2, 45, 1)
        || !command(handler - 1, 39, exit as i64)
        || !command(handler + 2, 17, 0)
        || !command(handler + 3, 45, 0)
        || !command(handler + 4, 39, exit as i64)
        || image.instructions[handler + 5].code != 1
        || image.instructions[handler + 5].operand <= 0
        || !command(handler + 6, 46, 0)
        || image.instructions[exit].code != 1
        || image.instructions[exit].operand <= 0
        || !command(end - 1, 22, 0)
    {
        return Err(format!("{at}: неподдерживаемая форма завершения попытки"));
    }
    // Нативная проба проверяет отказ внутри попытки и возврат обработчика.
    // Другие формы выхода и вложенные обработчики здесь не принимаются.
    let body = recover_variable_expressions(
        range.start + 1..handler - 2,
        variables,
        constants,
        false,
        image,
    )?;
    let caught =
        recover_variable_expressions(handler..handler + 3, variables, constants, false, image)?;
    Ok(format!(
        "Попытка\n{body}\nИсключение\n{caught}\nКонецПопытки;"
    ))
}

fn recover_loop_sequence(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
) -> Result<String, String> {
    recover_loop_sequence_with_loop_exits(range, variables, constants, image, None)
}

// Пересекающееся тело использует кадр текущего метода, а не ближайший entry.
fn recover_loop_sequence_in_frame(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
    frame: &[Variable],
) -> Result<String, String> {
    recover_loop_sequence_with_frame(range, variables, constants, image, None, Some(frame))
}

fn recover_loop_sequence_with_loop_exits(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
    loop_targets: Option<(usize, usize)>,
) -> Result<String, String> {
    recover_loop_sequence_with_frame(range, variables, constants, image, loop_targets, None)
}

fn recover_loop_sequence_with_frame(
    range: std::ops::Range<usize>,
    variables: &[&str],
    constants: &[String],
    image: &Image,
    loop_targets: Option<(usize, usize)>,
    frame: Option<&[Variable]>,
) -> Result<String, String> {
    // Формы временного стека и очистки итератора взяты из собственного
    // компиляторного образа. Восстанавливаем циклы, а не исполняем их.
    let mut cursor = range.start;
    let mut statements = Vec::new();
    while cursor < range.end {
        let Some(control) = (cursor..range.end)
            .find(|&index| matches!(image.instructions[index].code, 48 | 40 | 44))
        else {
            statements.push(recover_statement_branches_with_loop_exits(
                cursor..range.end,
                variables,
                constants,
                image,
                loop_targets,
            )?);
            break;
        };
        if image.instructions[control].code == 44 {
            statements.push(recover_statement_branches_with_loop_exits(
                cursor..range.end,
                variables,
                constants,
                image,
                loop_targets,
            )?);
            break;
        }
        let at = format!("image/Cmd[{control}]");
        let command = |index: usize, code: u32, operand: i64| {
            image.instructions.get(index) == Some(&Instruction { code, operand })
        };
        if image.instructions[control].code == 40 {
            let end = usize::try_from(image.instructions[control].operand)
                .ok()
                .filter(|&end| end > control + 1 && end <= range.end)
                .ok_or_else(|| format!("{at}: неверный выход цикла"))?;
            let begin = usize::try_from(image.instructions[end - 1].operand)
                .ok()
                .filter(|&begin| {
                    begin >= cursor && begin < control && command(end - 1, 39, begin as i64)
                });
            let Some(begin) = begin else {
                // Условный переход перед циклом может быть обычной веткой.
                // Направление проверенного завершающего перехода различает
                // эти формы; нативная проба охватывает выполнение и пропуск.
                statements.push(recover_statement_branches_with_loop_exits(
                    cursor..range.end,
                    variables,
                    constants,
                    image,
                    loop_targets,
                )?);
                break;
            };
            statements.push(recover_statement_branches_with_loop_exits(
                cursor..begin,
                variables,
                constants,
                image,
                loop_targets,
            )?);
            // Вызов в условии повторяется перед каждой итерацией,
            // что измеряет `measure-native-image-while-context-branch`.
            let condition = recover_expression_sequence(
                begin..control,
                variables,
                constants,
                true,
                true,
                image,
            )
            .map_err(|error| format!("{at}: условие цикла: {error}"))?;
            // Пробы `measure-native-image-while-break` и `while-continue`
            // проверяют выход и повтор условия именно этого цикла.
            // Цель `Продолжить` — начало условия, а не его середина.
            let body = recover_statement_branches_with_loop_exits(
                control + 1..end - 1,
                variables,
                constants,
                image,
                Some((begin, end)),
            )?;
            statements.push(format!("Пока {condition} Цикл\n{body}\nКонецЦикла;"));
            cursor = end;
            continue;
        }
        statements.push(recover_statement_branches_with_loop_exits(
            cursor..control,
            variables,
            constants,
            image,
            loop_targets,
        )?);
        let branch = (control + 1..range.end)
            .find(|&index| image.instructions[index].code == 40)
            .ok_or_else(|| format!("{at}: отсутствует выход цикла"))?;
        let end = usize::try_from(image.instructions[branch].operand)
            .ok()
            .filter(|&end| end > branch + 1 && end < range.end)
            .ok_or_else(|| format!("{at}: неверный выход цикла"))?;
        if !command(control, 48, 0) || !command(control + 1, 49, 0) || !command(end, 50, 1) {
            return Err(format!("{at}: неподдерживаемый временный стек цикла"));
        }
        let assignment = (control + 2..branch)
            .find(|&index| image.instructions[index].code == 16)
            .ok_or_else(|| format!("{at}: отсутствует значение временной переменной"))?;
        let iterator = command(assignment - 1, 14, 0);
        if iterator {
            let slot = image
                .instructions
                .get(branch - 2)
                .filter(|instruction| instruction.code == 3)
                .map(|instruction| instruction.operand)
                .ok_or_else(|| format!("{at}: отсутствует переменная итератора"))?;
            let element = image
                .instructions
                .get(branch + 1)
                .filter(|instruction| instruction.code == 3)
                .and_then(|instruction| usize::try_from(instruction.operand).ok())
                .and_then(|slot| variables.get(slot))
                .ok_or_else(|| format!("{at}: неверная переменная элемента"))?;
            let iterator_name = if let Some(frame) = frame {
                usize::try_from(slot).ok().and_then(|slot| frame.get(slot))
            } else {
                image
                    .procedures
                    .iter()
                    // Таблицы внешних и пока неподдерживаемых методов не задают
                    // локальные слоты тела обычного метода.
                    .filter(|procedure| {
                        matches!(procedure.flags, 0 | 1 | 2 | 16 | 17)
                            && procedure.entry as usize <= control
                    })
                    .max_by_key(|procedure| procedure.entry)
                    .and_then(|procedure| procedure.variables.as_deref())
                    .and_then(|locals| usize::try_from(slot).ok().and_then(|slot| locals.get(slot)))
            };
            // Проба `foreach-no-line-trampoline` измеряет отсутствие команды
            // строки перед проверкой и один дополнительный обратный переход.
            let check = if image.instructions[assignment + 1].code == 1
                && image.instructions[assignment + 1].operand > 0
            {
                assignment + 2
            } else {
                assignment + 1
            };
            let check_entry = assignment + 1;
            let outside_trampoline = usize::try_from(image.instructions[end - 1].operand)
                .ok()
                .filter(|&target| {
                    target < control
                        && command(end - 1, 39, target as i64)
                        && command(target, 39, check_entry as i64)
                });
            let body_end = if command(end - 1, 39, check_entry as i64) {
                if end >= 2 && command(end - 2, 39, (end - 1) as i64) {
                    end - 2
                } else {
                    end - 1
                }
            } else if end >= 2
                && command(end - 1, 39, (end - 2) as i64)
                && command(end - 2, 39, check_entry as i64)
            {
                // Исправленный нативный `foreach-backward-tail` проверяет
                // завершающий переход к предыдущему обратному переходу.
                end - 2
            } else if outside_trampoline.is_some() {
                // Нативные scoped-пробы сохраняют вынесенный прямой переход
                // к проверке итератора и условное продолжение через него.
                end - 1
            } else {
                return Err(format!("{at}: неверный обратный переход обхода коллекции"));
            };
            let continue_edge = body_end;
            let body_end = if let Some(trampoline) = outside_trampoline
                && end >= 4
                && command(end - 4, 40, trampoline as i64)
                && command(end - 3, 39, end as i64)
                && image.instructions[end - 2].code == 39
                && image.instructions[end - 2].operand >= 0
                && image.instructions[end - 2].operand < control as i64
                && !image.instructions.iter().any(|instruction| {
                    matches!(instruction.code, 39..=44) && instruction.operand == (end - 2) as i64
                }) {
                // Между безусловным выходом и обратным переходом находится
                // измеренная недостижимая команда; входов в неё быть не должно.
                end - 2
            } else {
                body_end
            };
            if check + 5 != branch
                || end + 7 > range.end
                || branch + 7 > body_end
                // Проба `foreach-renamed-iterator` сохраняет обход при
                // переименовании скрытого слота; проверяется его роль, а не имя.
                || iterator_name.is_none_or(|variable| variable.flags != 1)
                || !command(check, 3, slot)
                || !command(check + 1, 49, 0)
                || !command(check + 2, 16, 0)
                || !command(branch - 1, 15, 0)
                || !command(branch + 2, 3, slot)
                || !command(branch + 3, 16, 0)
                || !command(branch + 4, 3, slot)
                || !command(branch + 5, 8, 0)
                || !command(branch + 6, 16, 0)
                || image.instructions[branch + 7..body_end]
                    .iter()
                    .any(|instruction| instruction.code == 3 && instruction.operand == slot)
                || !(0..2).all(|offset| {
                    command(end + 1 + offset * 3, 3, slot)
                        && command(end + 2 + offset * 3, 8, 0)
                        && command(end + 3 + offset * 3, 16, 0)
                })
            {
                return Err(format!("{at}: неподдерживаемая форма обхода коллекции"));
            }
            // Нативная проба `measure-native-image-foreach-call`
            // получает коллекцию вызовом метода до начала обхода.
            // Сохранение результата относится к прологу, а не к телу цикла.
            let collection = recover_expression_sequence_with_prelude(
                control + 2..assignment - 1,
                variables,
                constants,
                true,
                false,
                image,
                Some(&mut statements),
            )?;
            let body = recover_statement_branches_with_loop_exits(
                branch + 7..body_end,
                variables,
                constants,
                image,
                Some((continue_edge, end)),
            )?;
            statements.push(format!(
                "Для Каждого {element} Из {collection} Цикл\n{body}\nКонецЦикла;"
            ));
            cursor = end + 7;
        } else {
            let slot = image
                .instructions
                .get(branch - 2)
                .filter(|instruction| instruction.code == 3)
                .map(|instruction| instruction.operand)
                .ok_or_else(|| format!("{at}: отсутствует счетчик цикла"))?;
            let counter = usize::try_from(slot)
                .ok()
                .and_then(|slot| variables.get(slot))
                .ok_or_else(|| format!("{at}: неверный счетчик цикла"))?;
            if assignment + 5 != branch
                || image.instructions[assignment + 1].code != 1
                || image.instructions[assignment + 1].operand <= 0
                || !command(assignment + 2, 49, 0)
                || !command(branch - 1, 37, 0)
                || !command(end - 3, 3, slot)
                || !command(end - 2, 43, 0)
                || !command(end - 1, 39, (assignment + 1) as i64)
            {
                return Err(format!("{at}: неподдерживаемая форма счетного цикла"));
            }
            // Проба `measure-native-image-counted-limit-call` меняет
            // коллекцию в теле, но граница определяется до обхода.
            // Пролог вызова сохраняется перед циклом, как у `Для Каждого`.
            let limit = recover_expression_sequence_with_prelude(
                control + 2..assignment,
                variables,
                constants,
                true,
                false,
                image,
                Some(&mut statements),
            )?;
            // `measure-native-image-counted-loop-exits` проверяет переход
            // продолжения к увеличению счётчика и прерывания к очистке кадра.
            let body = recover_statement_branches_with_loop_exits(
                branch + 1..end - 3,
                variables,
                constants,
                image,
                Some((end - 3, end)),
            )?;
            statements.push(format!(
                "Для {counter} = {counter} По {limit} Цикл\n{body}\nКонецЦикла;"
            ));
            cursor = end + 1;
        }
    }
    Ok(statements.join("\n"))
}

fn recover_return_branches(
    range: std::ops::Range<usize>,
    exit: usize,
    variables: &[&str],
    constants: &[String],
    image: &Image,
) -> Result<String, String> {
    // Нативный контрольный модуль проверяет все три пути возврата.
    // Принимаем только структурированное ветвление с полными ветками:
    // переходы должны вести к началу альтернативы и общему выходу.
    let commands = &image.instructions[range.clone()];
    if let Some(relative) = commands.iter().position(|command| command.code == 40) {
        let branch = range.start + relative;
        let alternative = usize::try_from(image.instructions[branch].operand)
            .map_err(|_| format!("image/Cmd[{branch}]: неверный переход"))?;
        if alternative <= branch + 1
            || alternative >= range.end
            || image.instructions[alternative - 1]
                != (Instruction {
                    code: 39,
                    operand: range.end as i64,
                })
        {
            return Err(format!(
                "image/Cmd[{branch}]: неподдерживаемые границы ветвления"
            ));
        }
        let condition =
            recover_variable_expressions(range.start..branch, variables, constants, true, image)?;
        let yes = recover_return_branches(
            branch + 1..alternative - 1,
            exit,
            variables,
            constants,
            image,
        )?;
        let no =
            recover_return_branches(alternative..range.end, exit, variables, constants, image)?;
        return Ok(format!(
            "Если {condition} Тогда\n{yes}\nИначе\n{no}\nКонецЕсли;"
        ));
    }
    if range.len() < 3
        || image.instructions[range.end - 3]
            != (Instruction {
                code: 17,
                operand: 0,
            })
        || image.instructions[range.end - 2]
            != (Instruction {
                code: 45,
                operand: 0,
            })
        || image.instructions[range.end - 1]
            != (Instruction {
                code: 39,
                operand: exit as i64,
            })
        || image.instructions[range.start..range.end - 3]
            .iter()
            .any(|command| command.code == 17)
    {
        return Err(format!(
            "image/Cmd[{}]: неподдерживаемое завершение ветки",
            range.start
        ));
    }
    recover_variable_expressions(
        range.start..range.end - 2,
        variables,
        constants,
        false,
        image,
    )
}

fn recover_variable_expressions(
    range: std::ops::Range<usize>,
    variables: &[&str],
    numbers: &[String],
    condition: bool,
    image: &Image,
) -> Result<String, String> {
    recover_expression_sequence(range, variables, numbers, condition, false, image)
}

fn recover_expression_sequence(
    range: std::ops::Range<usize>,
    variables: &[&str],
    numbers: &[String],
    condition: bool,
    inline_calls: bool,
    image: &Image,
) -> Result<String, String> {
    recover_expression_sequence_with_prelude(
        range,
        variables,
        numbers,
        condition,
        inline_calls,
        image,
        None,
    )
}

fn recover_expression_sequence_with_prelude(
    range: std::ops::Range<usize>,
    variables: &[&str],
    numbers: &[String],
    condition: bool,
    inline_calls: bool,
    image: &Image,
    prelude: Option<&mut Vec<String>>,
) -> Result<String, String> {
    // Это восстановление выражений, а не исполнение байткода. Значения
    // вычисляет обычный BSL после компиляции полученного текста.
    // Второй элемент отмечает ссылку, допустимую слева от присваивания.
    let mut stack: Vec<(String, bool)> = Vec::new();
    let mut statements = Vec::new();
    let mut argument_count = None;
    // В обычном блоке вызов уже записан отдельным оператором: его результат
    // может остаться непрочитанным или замениться следующим вызовом.
    // `measure-native-image-discarded-results` проверяет побочные эффекты.
    // В чистом условии такая замена потеряла бы сам вызов, поэтому запрещена.
    let mut returned_expression = None;
    let mut skip_until = range.start;
    let instructions = image
        .instructions
        .get(range.clone())
        .ok_or_else(|| "image/Cmd: неверные границы блока".to_owned())?;
    for (relative, instruction) in instructions.iter().enumerate() {
        let index = range.start + relative;
        if index < skip_until {
            continue;
        }
        let at = format!("image/Cmd[{index}]");
        if matches!(
            instruction.code,
            5 | 6 | 7 | 8 | 10 | 13 | 16 | 17 | 23 | 24 | 25 | 26 | 27 | 28 | 29 | 32 | 33 | 34 | 35 | 36 | 37 | 38 | 52 | 53 | 56 | 57 | 58 | 59 | 61 | 65..=67 | 70 | 71 | 72
                ..=84 | 93..=98 | 100 | 103 | 104 | 107 | 108 | 109 | 110 | 128
        ) && instruction.operand != 0
            // Нативный `opcode32-operand` подтверждает именно 1393;
            // остальные ненулевые операнды ещё не измерены.
            && !(instruction.code == 32 && instruction.operand == 1393)
        {
            return Err(format!("{at}: неподдерживаемый операнд команды"));
        }
        match instruction.code {
            1 if instruction.operand > 0 => {}
            2 => {
                let variable = usize::try_from(instruction.operand)
                    .ok()
                    .and_then(|index| image.variables.get(index))
                    .ok_or_else(|| format!("{at}: неверный индекс глобального имени"))?;
                if !matches!(variable.flags, 0 | 2)
                    || (variable.flags == 0 && image.entry == u32::MAX)
                    || variable.external_slot != -1
                    || !valid_name(&variable.name)
                {
                    return Err(format!("{at}: неподдерживаемое глобальное имя"));
                }
                if variables
                    .iter()
                    .any(|name| open_bsl::folded_eq(name, &variable.name))
                {
                    return Err(format!(
                        "{at}: глобальное имя перекрыто локальной переменной"
                    ));
                }
                // Внешнее имя с флагом `2` связывает обычный резолвер.
                // Частное имя с флагом `0` относится к объявленной модульной
                // переменной; проба инициализатора проверяет чтение и запись.
                stack.push((variable.name.clone(), variable.flags == 0));
            }
            3 => {
                let name = usize::try_from(instruction.operand)
                    .ok()
                    .and_then(|slot| variables.get(slot))
                    .ok_or_else(|| format!("{at}: неверный индекс локальной переменной"))?;
                stack.push(((*name).into(), true));
            }
            4 => {
                let number = usize::try_from(instruction.operand)
                    .ok()
                    .and_then(|index| numbers.get(index))
                    .ok_or_else(|| format!("{at}: неверный индекс константы"))?;
                stack.push((number.clone(), false));
            }
            10 => {
                // `measure-native-image-omitted-arguments` отличает пропуск
                // от `Неопределено`. Пустой текст обозначает только позицию
                // аргумента; `pop_expression` запрещает его как значение.
                stack.push((String::new(), false));
            }
            5 => {
                let value = returned_expression
                    .take()
                    .ok_or_else(|| format!("{at}: отсутствует результат вызова"))?;
                stack.push((value, false));
            }
            // Значение и тип проверены `measure-native-image-undefined`.
            8 => stack.push(("Неопределено".into(), false)),
            // Оба литерала присутствуют в нативном контрольном образе.
            6 => stack.push(("Ложь".into(), false)),
            7 => stack.push(("Истина".into(), false)),
            29 => {
                // Проба `measure-native-image-not` проверяет выбор обеих
                // противоположных веток и изменения ссылочного параметра.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("(Не {value})"), false));
            }
            30 | 31 => {
                // `scoped-initializer` измеряет пустую правую ветвь `И`
                // после литерала `Ложь`: переход сохраняет это значение.
                // Для прочих значений отсутствие правого операнда не допускаем.
                if instruction.code == 30
                    && instruction.operand == (index + 1) as i64
                    && matches!(stack.as_slice(), [(value, false)] if value == "Ложь")
                    && argument_count.is_none()
                    && returned_expression.is_none()
                {
                    continue;
                }

                let target = usize::try_from(instruction.operand)
                    .ok()
                    .filter(|&target| target > index + 1 && target <= range.end)
                    .ok_or_else(|| format!("{at}: неверный выход сокращенного вычисления"))?;
                if !matches!(
                    image.instructions[target - 1],
                    Instruction {
                        code: 32 | 69,
                        operand: 0,
                    }
                ) || argument_count.is_some()
                    || returned_expression.is_some()
                {
                    return Err(format!(
                        "{at}: неподдерживаемая граница логического выражения"
                    ));
                }
                let (left, _) = pop_expression(&mut stack, &at)?;
                // Вызов правого операнда остается внутри выражения BSL,
                // чтобы его побочный эффект происходил только при выборе
                // этой ветви. Нативная проба проверяет счетчик вызовов.
                // `measure-native-image-short-circuit32` подтверждает ту же
                // границу с явным преобразованием `32` вместо `69`.
                let right = recover_expression_sequence(
                    index + 1..target - 1,
                    variables,
                    numbers,
                    true,
                    true,
                    image,
                )?;
                let operator = if instruction.code == 30 {
                    "И"
                } else {
                    "Или"
                };
                stack.push((format!("({left} {operator} {right})"), false));
                skip_until = target;
            }
            12 => {
                let name = member_name(image, instruction.operand, &at)?;
                let (receiver, _) = pop_expression(&mut stack, &at)?;
                // Чтение и присваивание измерены отдельными образами.
                // Ссылку превращает в значение или цель присваивания
                // обычный компилятор BSL по месту использования.
                stack.push((format!("{receiver}.{name}"), true));
            }
            13 => {
                // Нативная проба `measure-native-image-index` проверяет
                // чтение, запись через индексную ссылку и выход за границы.
                // Доступ выполняет обычный BSL, сохраняя его ошибки.
                let (index, _) = pop_expression(&mut stack, &at)?;
                let (receiver, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("{receiver}[{index}]"), true));
            }
            16 => {
                let (value, _) = pop_expression(&mut stack, &at)?;
                let (target, assignable) = pop_expression(&mut stack, &at)?;
                if !assignable {
                    return Err(format!("{at}: цель присваивания не является переменной"));
                }
                statements.push(format!("{target} = {value};"));
            }
            17 => {
                if condition
                    || argument_count.is_some()
                    || (returned_expression.is_some() && inline_calls)
                {
                    return Err(format!("{at}: неподдерживаемое состояние перед возвратом"));
                }
                let (value, _) = pop_expression(&mut stack, &at)?;
                if !stack.is_empty() {
                    return Err(format!("{at}: остаток стека перед возвратом"));
                }
                if index + 1 < range.end {
                    let procedure = image
                        .procedures
                        .iter()
                        .filter(|procedure| {
                            matches!(procedure.flags, 0 | 1 | 2 | 16 | 17)
                                && procedure.entry as usize <= index
                        })
                        .max_by_key(|procedure| procedure.entry)
                        .filter(|procedure| matches!(procedure.flags, 1 | 17))
                        .ok_or_else(|| format!("{at}: возврат вне функции"))?;
                    let exit = method_end(image, procedure.entry)
                        .checked_sub(2)
                        .ok_or_else(|| format!("{at}: отсутствует выход функции"))?;
                    // Проба `measure-native-image-function-early-return`
                    // проверяет несколько возвратов с общей точкой выхода.
                    // Не отбрасываем произвольные команды после возврата.
                    if image.instructions.get(index + 1)
                        != Some(&Instruction {
                            code: 45,
                            operand: 0,
                        })
                        || image.instructions.get(index + 2)
                            != Some(&Instruction {
                                code: 39,
                                operand: exit as i64,
                            })
                        || image
                            .instructions
                            .get(exit)
                            .is_none_or(|line| line.code != 1 || line.operand <= 0)
                        || image.instructions.get(exit + 1)
                            != Some(&Instruction {
                                code: 22,
                                operand: 0,
                            })
                        || index + 3 > range.end
                        || image.instructions[index + 3..range.end]
                            .iter()
                            .enumerate()
                            .any(|(offset, line)| {
                                !(line.code == 1 && line.operand > 0
                                    || index + 3 + offset == exit + 1
                                        && range.end == exit + 2
                                        && line.code == 22
                                        && line.operand == 0)
                            })
                    {
                        return Err(format!("{at}: неподдерживаемый хвост возврата функции"));
                    }
                }
                statements.push(format!("Возврат {value};"));
                return Ok(statements.join("\n"));
            }
            45 if instruction.operand == 0 => {
                let procedure = image
                    .procedures
                    .iter()
                    .filter(|procedure| {
                        matches!(procedure.flags, 0 | 1 | 2 | 16 | 17)
                            && procedure.entry as usize <= index
                    })
                    .max_by_key(|procedure| procedure.entry)
                    .filter(|procedure| matches!(procedure.flags, 0 | 2 | 16))
                    .ok_or_else(|| format!("{at}: неподдерживаемый выход из процедуры"))?;
                let end = method_end(image, procedure.entry);
                let terminal = end
                    .checked_sub(1)
                    .filter(|&terminal| {
                        image.instructions.get(terminal)
                            == Some(&Instruction {
                                code: 22,
                                operand: 0,
                            })
                    })
                    .ok_or_else(|| format!("{at}: неверный конец процедуры"))?;
                // Нативный `procedure-exit-no-line` измеряет переход прямо
                // к концу; прежняя проба переходила к строке перед ним.
                let exit = terminal
                    .checked_sub(1)
                    .filter(|&line| {
                        image
                            .instructions
                            .get(line)
                            .is_some_and(|command| command.code == 1 && command.operand > 0)
                            && image.instructions.get(index + 1)
                                == Some(&Instruction {
                                    code: 39,
                                    operand: line as i64,
                                })
                    })
                    .unwrap_or(terminal);
                if condition
                    || argument_count.is_some()
                    || (returned_expression.is_some() && inline_calls)
                    || !stack.is_empty()
                    || exit < index + 2
                    || image.instructions.get(index + 1)
                        != Some(&Instruction {
                            code: 39,
                            operand: exit as i64,
                        })
                    || image
                        .instructions
                        .get((index + 2).min(range.end)..range.end)
                        .is_none_or(|tail| {
                            tail.iter().any(|line| line.code != 1 || line.operand <= 0)
                        })
                {
                    return Err(format!("{at}: неподдерживаемый выход из процедуры"));
                }
                // В `procedure-compact-return` проверенный переход также
                // служит границей ветви, поэтому её хвост после `Возврат` пуст.
                // Нативная проба `measure-native-image-procedure-early-return`
                // проверяет ссылочное присваивание перед выходом и пропуск
                // последующих операторов. Принимаем переход только к концу
                // именно этой процедуры, а не произвольное завершение блока.
                statements.push("Возврат;".into());
                return Ok(statements.join("\n"));
            }
            39 if instruction.operand == (index + 1) as i64
                && argument_count.is_none()
                && returned_expression.is_none() =>
            {
                // Нативная проба `measure-native-image-next-jumps` проверяет
                // переход к следующей команде со значением, внутри условия
                // и со ссылочной целью присваивания. Стек сохраняется;
                // незавершённый вызов эта проба не покрывает.
            }
            40 if backward_boolean_join(image, index).is_some() => {
                // `measure-native-image-backward-bool` сохраняет ссылочную
                // цель до выбора литерала и общего обратного присваивания.
                if argument_count.is_some()
                    || returned_expression.is_some()
                    || index + 2 > range.end
                {
                    return Err(format!("{at}: неверное состояние булевого присваивания"));
                }
                let (condition, _) = pop_expression(&mut stack, &at)?;
                let (target, assignable) = pop_expression(&mut stack, &at)?;
                if !assignable {
                    return Err(format!("{at}: цель присваивания не является переменной"));
                }
                statements.push(format!("{target} = ?({condition}, Истина, Ложь);"));
                skip_until = index + 2;
            }
            40 | 41 => {
                if argument_count.is_some() || returned_expression.is_some() {
                    return Err(format!(
                        "{at}: неподдерживаемое состояние условного выражения"
                    ));
                }
                let alternative = usize::try_from(instruction.operand)
                    .ok()
                    .filter(|&target| target > index + 1 && target < range.end)
                    .ok_or_else(|| format!("{at}: неверная граница условного выражения"))?;
                let jump = &image.instructions[alternative - 1];
                let join = usize::try_from(jump.operand)
                    .ok()
                    .filter(|&target| {
                        jump.code == 39 && target > alternative && target <= range.end
                    })
                    .ok_or_else(|| format!("{at}: неверное продолжение условного выражения"))?;
                let (predicate, _) = pop_expression(&mut stack, &at)?;
                // Нативная проба `measure-native-image-jump-true` сохраняет
                // полярность операции 41 без вычисления второй альтернативы.
                let predicate = if instruction.code == 41 {
                    format!("НЕ ({predicate})")
                } else {
                    predicate
                };
                let yes = recover_expression_sequence(
                    index + 1..alternative - 1,
                    variables,
                    numbers,
                    true,
                    true,
                    image,
                )?;
                let no = recover_expression_sequence(
                    alternative..join,
                    variables,
                    numbers,
                    true,
                    true,
                    image,
                )?;
                // Нативная проба `measure-native-image-conditional-expression`
                // сохраняет ссылочную цель под условием и не вычисляет деление
                // на ноль в невыбранной ветке. Ленивость обеспечивает обычный
                // компилятор BSL, а не отдельный исполнитель образа.
                stack.push((format!("?({predicate}, {yes}, {no})"), false));
                skip_until = join;
            }
            47 if instruction.operand == 1 => {
                if condition || argument_count.is_some() || returned_expression.is_some() {
                    return Err(format!(
                        "{at}: неподдерживаемое состояние перед исключением"
                    ));
                }
                let (value, _) = pop_expression(&mut stack, &at)?;
                if !stack.is_empty() {
                    return Err(format!("{at}: остаток стека перед исключением"));
                }
                statements.push(format!("ВызватьИсключение {value};"));
            }
            51 => {
                let count = argument_count
                    .take()
                    .filter(|count| matches!(count, 0..=6))
                    .ok_or_else(|| {
                        format!("{at}: неподдерживаемое число аргументов конструктора")
                    })?;
                let name = member_name(image, instruction.operand, &at)?;
                let arguments = pop_arguments(&mut stack, count, &at, false)?;
                // `measure-native-image-new` проверяет создание нового
                // объекта и замену аргумента. Сохраняем момент создания:
                // последующие выражения используют уже готовый объект.
                let temporary = temporary_name(image, index);
                // Проба `measure-native-image-constructor-arguments`
                // проверяет порядок нуля–шести аргументов и независимость
                // вложенных массивов после двух отдельных созданий.
                statements.push(format!("{temporary} = Новый {name}({arguments});"));
                stack.push((temporary, false));
            }
            18 => {
                if argument_count.replace(instruction.operand).is_some() {
                    return Err(format!("{at}: повторное число аргументов"));
                }
            }
            19 => {
                let procedure = usize::try_from(instruction.operand)
                    .ok()
                    .and_then(|index| image.procedures.get(index))
                    .ok_or_else(|| format!("{at}: неверный индекс процедуры"))?;
                let count = argument_count
                    .take()
                    .filter(|&count| {
                        let Ok(count) = usize::try_from(count) else {
                            return false;
                        };
                        let parameters = procedure.parameter_count as usize;
                        if count > parameters {
                            return false;
                        }
                        match procedure.flags {
                            32 | 33 => true,
                            0 | 1 | 2 | 16 | 17 => {
                                count == parameters
                                    || procedure.defaults.as_deref().is_some_and(|defaults| {
                                        defaults.len() == parameters
                                            && defaults[count..].iter().all(|value| {
                                                *value
                                                    != Value::List(vec![Value::String(
                                                        String::new(),
                                                    )])
                                            })
                                    })
                            }
                            _ => false,
                        }
                    })
                    .ok_or_else(|| format!("{at}: неподдерживаемый локальный вызов"))?;
                if returned_expression.is_some() && inline_calls {
                    return Err(format!("{at}: неподдерживаемый локальный вызов"));
                }
                // Нативная проба `measure-native-image-local-arguments`
                // проверяет порядок нескольких аргументов, ссылочные
                // изменения и опущенный хвост необязательных параметров.
                // `measure-native-image-local-omitted-arguments` проверяет
                // восемь параметров локальных функций и процедур, пропуски
                // в разных позициях и границы ссылочных изменений.
                let argument = pop_arguments(
                    &mut stack,
                    count,
                    &at,
                    // `measure-native-image-external-omitted` проверяет
                    // пропущенный хвост внешней процедуры. Значения
                    // умолчаний по-прежнему определяет обычный резолвер.
                    // `measure-native-image-external-function-hole` проверяет
                    // пропущенный первый аргумент внешней функции, а проба
                    // `external-function-nstr-hole` — пропущенный хвост.
                    matches!(procedure.flags, 0 | 1 | 2 | 16 | 17 | 32 | 33),
                )?;
                if matches!(procedure.flags, 1 | 17 | 33) {
                    // Возврат и ссылочный параметр закрытой функции
                    // проверены `measure-native-image-local-function`.
                    if inline_calls {
                        returned_expression = Some(format!("{}({argument})", procedure.name));
                    } else {
                        let temporary = temporary_name(image, index);
                        statements.push(format!("{temporary} = {}({argument});", procedure.name));
                        returned_expression = Some(temporary);
                    }
                } else {
                    // Чтение регистра после процедуры пока не измерено.
                    returned_expression = None;
                    statements.push(format!("{}({argument});", procedure.name));
                }
            }
            21 => {
                // `measure-native-image-context-nine-arguments` дополняет
                // прежние пробы арностями 7–9 и пропущенными позициями.
                let count = argument_count
                    .take()
                    .filter(|count| matches!(count, 0..=9))
                    .ok_or_else(|| format!("{at}: неподдерживаемая форма контекстного вызова"))?;
                if returned_expression.is_some() && inline_calls {
                    return Err(format!("{at}: неподдерживаемая форма контекстного вызова"));
                }
                let name = member_name(image, instruction.operand, &at)?;
                let arguments = pop_arguments(&mut stack, count, &at, true)?;
                let (receiver, _) = pop_expression(&mut stack, &at)?;
                // Нативная проба `measure-native-image-context-function-arguments`
                // проверяет все арности до шести, необязательный хвост
                // и две ссылки на одну переменную.
                if inline_calls {
                    // В условии `Пока` вызов нельзя вынести перед циклом:
                    // он повторяется после изменений в теле цикла.
                    returned_expression = Some(format!("{receiver}.{name}({arguments})"));
                } else {
                    // Результат фиксируется в месте вызова: последующая
                    // процедура может изменить получателя до использования
                    // этого значения (`measure-native-image-call-order`).
                    let temporary = temporary_name(image, index);
                    statements.push(format!("{temporary} = {receiver}.{name}({arguments});"));
                    returned_expression = Some(temporary);
                }
            }
            20 => {
                // `measure-native-image-context-nine-arguments` дополняет
                // прежние пробы арностями 7–9 и пропущенными позициями.
                let count = argument_count
                    .take()
                    .filter(|count| matches!(count, 0..=9))
                    .ok_or_else(|| format!("{at}: неподдерживаемая форма контекстной процедуры"))?;
                if returned_expression.is_some() && inline_calls {
                    return Err(format!(
                        "{at}: неподдерживаемая форма контекстной процедуры"
                    ));
                }
                // Не выдаём непроверенный результат процедуры через `LdRet`.
                returned_expression = None;
                let name = member_name(image, instruction.operand, &at)?;
                // Два и четыре аргумента измерены пробой
                // `measure-native-image-context-arguments`; три, пять и шесть,
                // включая ссылочные изменения и опущенные параметры, —
                // `measure-native-image-context-procedures`.
                let arguments = pop_arguments(&mut stack, count, &at, true)?;
                let (receiver, _) = pop_expression(&mut stack, &at)?;
                // Процедура печатается отдельным оператором в месте вызова,
                // чтобы следующий вызов видел уже изменённый объект.
                statements.push(format!("{receiver}.{name}({arguments});"));
            }
            23 => {
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("(-{value})"), false));
            }
            32 => {
                // Нативная проба `measure-native-image-opcode32-values`
                // проверяет тип и значение преобразования, ошибки и запись
                // через ссылочный параметр. Преобразует существующий BSL.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("Булево({value})"), false));
            }
            66 => {
                // Нативные `int` и `int-snapshot` проверяют усечение,
                // преобразования, ошибки, ссылочную запись и момент результата.
                // Числовое приведение сохраняет измеренные строки и булево.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("Цел(Число({value}))"), false));
            }
            70 => {
                // Нативная проба `number-conversion` проверяет тип, значения,
                // перехватываемые ошибки и запись через ссылочный параметр.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("Число({value})"), false));
            }
            72..=74 => {
                // Нативная проба `measure-native-image-date-construction`
                // проверяет строковую форму, календарные части, время
                // и тип результата. Дату создает существующая функция BSL.
                let count = match instruction.code {
                    72 => 1,
                    73 => 3,
                    _ => 6,
                };
                let arguments = pop_arguments(&mut stack, count, &at, false)?;
                stack.push((format!("Дата({arguments})"), false));
            }
            24 | 25 | 26 | 27 | 28 | 33 | 34 | 35 | 36 | 37 | 38 | 75 => {
                let (right, _) = pop_expression(&mut stack, &at)?;
                let (left, _) = pop_expression(&mut stack, &at)?;
                let text = if instruction.code == 75 {
                    format!("ДобавитьМесяц({left}, {right})")
                } else {
                    let operator = match instruction.code {
                        24 => "+",
                        25 => "-",
                        26 => "*",
                        // Порядок операндов, дробные значения, знаки
                        // и ошибки нуля измерены `measure-native-image-division`.
                        27 => "/",
                        28 => "%",
                        33 => "=",
                        // Граница равенства отдельно проверена нативной
                        // пробой `measure-native-image-comparisons`.
                        34 => "<>",
                        36 => "<",
                        37 => ">=",
                        38 => "<=",
                        _ => ">",
                    };
                    format!("({left} {operator} {right})")
                };
                stack.push((text, false));
            }
            82..=84 => {
                // `measure-native-image-scalar-builtins` сверяет месяц и день
                // на високосной дате и крайних годах. Части даты читает runtime.
                let name = ["Год", "Месяц", "День"][(instruction.code - 82) as usize];
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("{name}({value})"), false));
            }
            53 | 56 => {
                // `measure-native-image-length-trim-snapshots` проверяет длину
                // строки вне BMP и сокращение краёв до изменения аргумента.
                // Unicode и пробелы обрабатывает существующий runtime.
                let name = if instruction.code == 53 {
                    "СтрДлина"
                } else {
                    "СокрЛП"
                };
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("{name}({value})"), false));
            }
            57 => {
                // Нативная проба `measure-native-image-left` проверяет
                // кириллицу, пустой результат и длину больше исходной строки.
                let arguments = pop_arguments(&mut stack, 2, &at, false)?;
                stack.push((format!("Лев({arguments})"), false));
            }
            58 => {
                // `measure-native-image-remaining-string-snapshots` проверяет
                // правую подстроку и её сохранение до изменения исходной строки.
                let arguments = pop_arguments(&mut stack, 2, &at, false)?;
                stack.push((format!("Прав({arguments})"), false));
            }
            65 => {
                // Та же нативная проба подтверждает булев результат проверки
                // пробельной строки до следующего изменяющего вызова.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("ПустаяСтрока({value})"), false));
            }
            71 => {
                // `measure-native-image-string-snapshot` подтверждает конверсию
                // и сохранение текста до изменения ссылочного аргумента.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("Строка({value})"), false));
            }
            61 => {
                // `measure-native-image-upper-snapshot` проверяет кириллицу,
                // латиницу и сохранение результата до изменения аргумента.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("ВРег({value})"), false));
            }
            59 => {
                // Нативная проба скалярных функций проверяет порядок аргументов,
                // кириллицу, пустую строку и выход за конец строки.
                let arguments = pop_arguments(&mut stack, 3, &at, false)?;
                stack.push((format!("Сред({arguments})"), false));
            }
            67 => {
                // Та же проба проверяет знаки, половинные значения
                // и отрицательную разрядность. Округляет существующая `Окр`.
                let arguments = pop_arguments(&mut stack, 2, &at, false)?;
                stack.push((format!("Окр({arguments})"), false));
            }
            76..=81 => {
                // Нативная проба `measure-native-image-date-boundaries`
                // проверяет високосный день и границы квартала и года.
                // Календарные вычисления остаются в существующих функциях BSL.
                let name = [
                    "НачалоМесяца",
                    "КонецМесяца",
                    "НачалоКвартала",
                    "КонецКвартала",
                    "НачалоГода",
                    "КонецГода",
                ][(instruction.code - 76) as usize];
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("{name}({value})"), false));
            }
            93..=98 => {
                // Нативная проба `measure-native-image-day-time-boundaries`
                // проверяет шесть границ на високосном дне и смене года.
                // Недельные команды остаются неподдерживаемыми: файловый
                // и серверный оракулы дают разные результаты.
                let name = [
                    "НачалоДня",
                    "КонецДня",
                    "НачалоЧаса",
                    "КонецЧаса",
                    "НачалоМинуты",
                    "КонецМинуты",
                ][(instruction.code - 93) as usize];
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("{name}({value})"), false));
            }
            100 => {
                // Нативная проба `measure-native-image-string-replace`
                // проверяет порядок трёх аргументов, пустой поиск
                // и неизменность переданного по ссылке исходного текста.
                let arguments = pop_arguments(&mut stack, 3, &at, false)?;
                stack.push((format!("СтрЗаменить({arguments})"), false));
            }
            110 => {
                // Нативная проба `measure-native-image-format` проверяет
                // порядок значения и форматной строки для нескольких типов.
                let arguments = pop_arguments(&mut stack, 2, &at, false)?;
                stack.push((format!("Формат({arguments})"), false));
            }
            103 | 104 => {
                let count = argument_count
                    .take()
                    .filter(|count| matches!(count, 1..=6))
                    .ok_or_else(|| {
                        format!("{at}: неподдерживаемое число аргументов Мин или Макс")
                    })?;
                // Нативная проба `measure-native-image-min-max` измеряет
                // переменную арность, числа, строки, даты и неизменность
                // ссылочного параметра. Сравнение выполняет runtime.
                let name = if instruction.code == 103 {
                    "Мин"
                } else {
                    "Макс"
                };
                let arguments = pop_arguments(&mut stack, count, &at, false)?;
                stack.push((format!("{name}({arguments})"), false));
            }
            52 => {
                if condition
                    || inline_calls
                    || argument_count.is_some()
                    || returned_expression.is_some()
                {
                    return Err(format!(
                        "{at}: неподдерживаемое состояние перед выполнением фрагмента"
                    ));
                }
                let (fragment, _) = pop_expression(&mut stack, &at)?;
                if !stack.is_empty() {
                    return Err(format!("{at}: остаток стека перед выполнением фрагмента"));
                }
                // Независимый образ `execute-compiled` измеряет ссылочное
                // изменение из фрагмента. Его компилирует существующее ядро.
                statements.push(format!("Выполнить({fragment});"));
            }
            109 => {
                // `measure-local-eval` подтверждает поиск частной функции
                // в текущем модуле. Компиляцию текста выполняет ядро BSL.
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("Вычислить({value})"), false));
            }
            128 => {
                if inline_calls {
                    return Err(format!(
                        "{at}: неподдерживаемый снимок ошибки в чистом выражении"
                    ));
                }
                // `measure-native-image-statement-try` проверяет описание
                // текущей ошибки. Снимок берётся в момент команды, до любых
                // последующих вызовов, через существующий контекст BSL.
                let temporary = temporary_name(image, index);
                statements.push(format!("{temporary} = ИнформацияОбОшибке();"));
                stack.push((temporary, false));
            }
            107 | 108 => {
                // Нативная проба `measure-native-image-type-functions`
                // проверяет типы значений, псевдоним, регистр имени
                // и отказ для неизвестного типа. Поиск выполняет runtime.
                let name = if instruction.code == 107 {
                    "ТипЗнч"
                } else {
                    "Тип"
                };
                let (value, _) = pop_expression(&mut stack, &at)?;
                stack.push((format!("{name}({value})"), false));
            }
            _ => return Err(format!("{at}: неподдерживаемая инструкция выражения")),
        }
        if !inline_calls
            && (!condition || prelude.is_some())
            && matches!(instruction.code, 23..=29 | 32..=38 | 53 | 56 | 57 | 58 | 59 | 61 | 65..=67 | 70 | 71 | 72..=84 | 93..=98 | 100 | 103 | 104 | 107..=110)
        {
            // `measure-native-image-operator-snapshots` проверяет, что результат
            // операции уже сохранён, когда следующий вызов меняет её операнд.
            // `measure-native-image-builtin-snapshots` подтверждает тот же
            // порядок для измеренных встроенных функций дат, строк и типов.
            // `native-calendar-snapshots` проверяет двенадцать границ даты
            // до следующего вызова, изменяющего ссылочный аргумент.
            // Ссылки остаются ссылками; чистые сокращённые ветки сохраняют
            // выражения внутри выбранной ветки, без предварительного вычисления.
            let (value, _) = pop_expression(&mut stack, &at)?;
            let temporary = temporary_name(image, index);
            statements.push(format!("{temporary} = {value};"));
            stack.push((temporary, false));
        }
    }
    if argument_count.is_some() || (returned_expression.is_some() && inline_calls) {
        return Err("image/Cmd: число аргументов без вызова".into());
    }
    if condition {
        if stack.len() != 1 {
            return Err(format!(
                "image/Cmd: условие должно содержать одно выражение (Cmd[{}]..Cmd[{}])",
                range.start, range.end
            ));
        }
        if let Some(prelude) = prelude {
            prelude.extend(statements);
        } else if !statements.is_empty() {
            return Err(format!(
                "image/Cmd: условие должно содержать одно выражение (Cmd[{}]..Cmd[{}])",
                range.start, range.end
            ));
        }
        Ok(pop_expression(&mut stack, "image/Cmd")?.0)
    } else if stack.is_empty() {
        Ok(statements.join("\n"))
    } else {
        Err(format!(
            "image/Cmd[{}]: остаток стека на границе блока",
            range.end
        ))
    }
}

fn pop_arguments(
    stack: &mut Vec<(String, bool)>,
    count: i64,
    at: &str,
    allow_omitted: bool,
) -> Result<String, String> {
    let mut arguments = Vec::new();
    for _ in 0..count {
        if allow_omitted && stack.last().is_some_and(|(text, _)| text.is_empty()) {
            stack.pop();
            arguments.push(String::new());
        } else {
            arguments.push(pop_expression(stack, at)?.0);
        }
    }
    arguments.reverse();
    Ok(arguments.join(", "))
}

fn pop_expression(stack: &mut Vec<(String, bool)>, at: &str) -> Result<(String, bool), String> {
    let expression = stack
        .pop()
        .ok_or_else(|| format!("{at}: недостаточно операндов выражения"))?;
    if expression.0.is_empty() {
        return Err(format!("{at}: пропуск аргумента не является значением"));
    }
    Ok(expression)
}

fn member_name<'a>(image: &'a Image, operand: i64, at: &str) -> Result<&'a str, String> {
    let constant = usize::try_from(operand)
        .ok()
        .and_then(|index| image.constants.get(index))
        .ok_or_else(|| format!("{at}: неверный индекс имени члена"))?;
    let fields = sequence(constant, at)?;
    let [Value::String(tag), Value::String(name)] = fields else {
        return Err(format!("{at}: имя члена должно быть строковой константой"));
    };
    if tag != "S" || !valid_name(name) {
        return Err(format!("{at}: имя члена нельзя поместить в BSL"));
    }
    Ok(name)
}

fn temporary_name(image: &Image, index: usize) -> String {
    let mut name = format!("__open_bsl_image_result_{index}");
    while image.procedures.iter().any(|procedure| {
        open_bsl::folded_eq(&procedure.name, &name)
            || procedure
                .variables
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|variable| open_bsl::folded_eq(&variable.name, &name))
    }) || image
        .variables
        .iter()
        .any(|variable| open_bsl::folded_eq(&variable.name, &name))
    {
        name.push('_');
    }
    name
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

fn parse_procedure(value: &Value, at: &str) -> Result<Procedure, String> {
    let fields = sequence(value, at)?;
    if fields.len() < 4 {
        return Err(format!("{at}: неполный заголовок процедуры"));
    }
    let mut variables = None;
    let mut defaults = None;
    let mut labels = None;
    for value in &fields[4..] {
        let section = sequence(value, at)?;
        let name = string(section.first(), at)?;
        match name {
            "Var" if variables.is_none() => {
                variables = Some(parse_variables(section, &format!("{at}/Var"))?);
            }
            "DefPrm" if defaults.is_none() => {
                defaults = Some(counted(section, 2, &format!("{at}/DefPrm"))?.to_vec());
            }
            "Lbl" if labels.is_none() => {
                labels = Some(parse_labels(section, &format!("{at}/Lbl"))?);
            }
            _ => return Err(format!("{at}/{name}: неизвестная или повторная секция")),
        }
    }
    Ok(Procedure {
        name: string(fields.first(), at)?.into(),
        flags: unsigned(fields.get(1), at)?,
        parameter_count: unsigned(fields.get(2), at)?,
        entry: unsigned(fields.get(3), at)?,
        variables,
        defaults,
        labels: labels.unwrap_or_default(),
    })
}

fn validate_labels(section: &[Value], at: &str) -> Result<(), String> {
    parse_labels(section, at).map(|_| ())
}

fn parse_labels(section: &[Value], at: &str) -> Result<Vec<u32>, String> {
    // Неиспользуемые записи проверяются по форме. Адресное значение
    // применяется только для измеренного локального перехода операции 42.
    let mut targets = Vec::new();
    for (index, value) in counted(section, 2, at)?.iter().enumerate() {
        let at = format!("{at}[{index}]");
        let fields = sequence(value, &at)?;
        if fields.len() != 2 {
            return Err(format!("{at}: метка должна содержать два поля"));
        }
        string(fields.first(), &at)?;
        targets.push(unsigned(fields.get(1), &at)?);
    }
    Ok(targets)
}

fn parse_variables(section: &[Value], at: &str) -> Result<Vec<Variable>, String> {
    counted(section, 2, at)?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let at = format!("{at}[{index}]");
            let fields = sequence(value, &at)?;
            if fields.len() != 3 {
                return Err(format!("{at}: переменная должна содержать три поля"));
            }
            Ok(Variable {
                name: string(fields.first(), &at)?.into(),
                flags: unsigned(fields.get(1), &at)?,
                external_slot: signed(fields.get(2), &at)?,
            })
        })
        .collect()
}

fn required<'a>(section: Option<&'a [Value]>, name: &str) -> Result<&'a [Value], String> {
    section.ok_or_else(|| format!("image/{name}: обязательная секция отсутствует"))
}

fn counted<'a>(section: &'a [Value], prefix: usize, at: &str) -> Result<&'a [Value], String> {
    // Нативные пробы `measure-native-image-table-count-*` подтверждают:
    // числовой заголовок может быть больше или меньше списка записей.
    // Его назначение не выводим; читаем все фактически присутствующие
    // записи, не дополняя таблицу вымышленными элементами.
    unsigned(section.get(1), at)?;
    let values = section
        .get(prefix..)
        .ok_or_else(|| format!("{at}: неполный заголовок таблицы"))?;
    Ok(values)
}

fn sequence<'a>(value: &'a Value, at: &str) -> Result<&'a [Value], String> {
    match value {
        Value::List(values) => Ok(values),
        _ => Err(format!("{at}: ожидалась последовательность")),
    }
}

fn string<'a>(value: Option<&'a Value>, at: &str) -> Result<&'a str, String> {
    match value {
        Some(Value::String(value)) => Ok(value),
        _ => Err(format!("{at}: ожидалась строка")),
    }
}

fn unsigned(value: Option<&Value>, at: &str) -> Result<u32, String> {
    match value {
        Some(Value::Atom(value)) => value.parse().ok(),
        _ => None,
    }
    .ok_or_else(|| format!("{at}: ожидалось неотрицательное 32-разрядное целое"))
}

fn signed(value: Option<&Value>, at: &str) -> Result<i64, String> {
    match value {
        Some(Value::Atom(value)) => value.parse().ok(),
        _ => None,
    }
    .ok_or_else(|| format!("{at}: ожидалось целое"))
}

#[cfg(test)]
#[path = "native_image_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_initializer_tests.rs"]
mod initializer_tests;

#[cfg(test)]
#[path = "native_declaration_tests.rs"]
mod declaration_tests;
