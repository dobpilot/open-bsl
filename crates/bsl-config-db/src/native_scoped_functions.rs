//! Восстановление измеренных пересекающихся методов через метки обычного BSL.

use super::{
    Image, Instruction, constant_literals, recover_expression_sequence,
    recover_expression_sequence_with_prelude, valid_name,
};
use open_sdbl::metadata::Value;
use std::collections::{BTreeSet, VecDeque};

pub(super) fn recover(image: &Image) -> Option<Result<String, String>> {
    let candidate = image.entry == u32::MAX
        && image.variables.is_empty()
        && matches!(image.procedures.len(), 3 | 4)
        && image.instructions.iter().any(|command| command.code == 42)
        && image.procedures.iter().any(|method| {
            matches!(method.flags, 1 | 2)
                && method.variables.as_ref().is_some_and(|variables| {
                    variables.len() == 4 && variables.iter().any(|variable| variable.flags == 1338)
                })
                && !method.labels.is_empty()
        });
    candidate.then(|| recover_module(image))
}

fn recover_module(image: &Image) -> Result<String, String> {
    if image.constants.iter().any(|constant| {
        !matches!(constant, Value::List(fields)
            if matches!(fields.as_slice(), [Value::String(tag), Value::Atom(_)] if tag == "N")
                || matches!(fields.as_slice(), [Value::String(tag), Value::String(_)] if tag == "S"))
    }) {
        return Err("image/Const: неизмеренные константы пересекающихся методов".into());
    }
    let constants = constant_literals(&image.constants)?;
    let mut source = String::new();
    for index in 0..image.procedures.len() {
        source.push_str(&recover_method(image, index, &constants)?);
    }
    Ok(source)
}

fn measured_private_frame(method: &super::Procedure) -> bool {
    let variables = method.variables.as_deref().unwrap_or_default();
    matches!(method.flags, 1 | 2)
        && method.defaults.is_none()
        && variables.len() == 4
        && variables
            .iter()
            .zip([1338, 1459, 3091, 3027])
            .all(|(variable, flags)| variable.flags == flags && variable.external_slot == 0)
        // Контроль `zero-label-procedure-27` сохраняет оба пути и очередь
        // сообщений при исходной для Proc36 длине нулевой таблицы меток.
        && (matches!(method.labels.as_slice(), [0, 0, 0, 0, 0 | 32])
            || (method.flags == 2
                && method.labels.len() == 27
                && method.labels.iter().all(|&label| label == 0)))
}

pub(super) fn recover_private_method(
    image: &Image,
    index: usize,
    constants: &[String],
) -> Option<Result<String, String>> {
    let method = &image.procedures[index];
    measured_private_frame(method).then(|| recover_method(image, index, constants))
}

fn recover_method(image: &Image, index: usize, constants: &[String]) -> Result<String, String> {
    let method = &image.procedures[index];
    let mut source = String::new();
    let at = format!("image/Proc[{index}]");
    if !valid_name(&method.name)
        || image.procedures[..index]
            .iter()
            .any(|previous| open_bsl::folded_eq(&previous.name, &method.name))
        || !matches!(method.flags, 1 | 2 | 16 | 17 | 32)
        || method.parameter_count != 1
    {
        return Err(format!(
            "{at}: неизмеренный заголовок пересекающейся функции"
        ));
    }
    if method.flags == 32 {
        if !message_declaration(method) {
            return Err(format!("{at}: неизмеренный внешний callback"));
        }
        // Внешнее имя связывает обычный резолвер с уже имеющимся runtime.
        // Отдельного объявления или заглушки в исходнике не создаём.
        return Ok(String::new());
    }
    let variables = method.variables.as_deref().unwrap_or_default();
    if variables.iter().enumerate().any(|(slot, variable)| {
        !valid_name(&variable.name)
            || variables[..slot]
                .iter()
                .any(|previous| open_bsl::folded_eq(&previous.name, &variable.name))
    }) {
        return Err(format!("{at}: неподдерживаемые имена кадра"));
    }
    let ordinary = matches!(variables.len(), 1 | 4)
        && variables[0].flags == 5
        && variables
            .iter()
            .all(|variable| variable.external_slot == -1)
        && variables[1..].iter().all(|variable| variable.flags == 1)
        && method.defaults.as_deref() == Some(&[Value::List(vec![Value::String(String::new())])]);
    let private = measured_private_frame(method);
    if !ordinary && !private {
        return Err(format!("{at}: неизмеренный кадр пересекающейся функции"));
    }
    let names: Vec<_> = variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect();
    let mut argument = names[0].to_owned();
    if private {
        // Нативные слоты читают константу, а входящий аргумент не меняют.
        // Его отдельное имя исключает запись через ссылку при инициализации.
        argument = "__ВходящийАргумент".into();
        while names
            .iter()
            .any(|name| open_bsl::folded_eq(name, &argument))
        {
            argument.push('_');
        }
    }
    let export = if matches!(method.flags, 16 | 17) {
        " Экспорт"
    } else {
        ""
    };
    let kind = if matches!(method.flags, 2 | 16) {
        "Процедура"
    } else {
        "Функция"
    };
    source.push_str(&format!("{kind} {}({argument}){export}\n", method.name));
    if ordinary && names.len() > 1 {
        source.push_str(&format!("Перем {};\n", names[1..].join(", ")));
    }
    if private {
        // Строковый алиас измерен отдельно для процедуры с 27 нулевыми
        // метками, включая объявленный размер кадра `Var18`.
        if !matches!(image.constants.first(), Some(Value::List(fields))
            if matches!(fields.as_slice(), [Value::String(tag), Value::Atom(_)] if tag == "N")
                || (method.flags == 2
                    && method.labels.len() == 27
                    && method.labels.iter().all(|&label| label == 0)
                    && matches!(fields.as_slice(), [Value::String(tag), Value::String(_)] if tag == "S")))
        {
            return Err(format!("{at}: неизмеренный тип константы кадра"));
        }
        let constant = constants
            .first()
            .ok_or_else(|| format!("{at}: нет константы кадра"))?;
        source.push_str(&format!("Перем {};\n", names.join(", ")));
        for name in &names {
            source.push_str(&format!("{name} = {constant};\n"));
        }
    }
    source.push_str(&recover_blocks(image, index, &names, constants, private)?);
    let ending = if matches!(method.flags, 2 | 16) {
        "КонецПроцедуры"
    } else {
        "КонецФункции"
    };
    source.push_str(ending);
    source.push('\n');
    Ok(source)
}

fn message_declaration(method: &super::Procedure) -> bool {
    method.flags == 32
        && method.parameter_count == 1
        && method.entry == 0
        && method.variables.is_none()
        && method.defaults.is_none()
        && method.labels.is_empty()
        && ["Сообщить", "Message"]
            .iter()
            .any(|name| open_bsl::folded_eq(name, &method.name))
}

fn literal_message_call(image: &Image, pc: usize) -> bool {
    let Some(commands) = pc
        .checked_sub(2)
        .and_then(|start| image.instructions.get(start..=pc))
    else {
        return false;
    };
    matches!(commands, [Instruction { code: 4, operand: constant }, Instruction { code: 18, operand: 1 }, Instruction { code: 19, operand: method }]
        if usize::try_from(*constant).ok().and_then(|slot| image.constants.get(slot)).is_some_and(|value| {
            matches!(value, Value::List(fields)
                if matches!(fields.as_slice(), [Value::String(tag), Value::String(_)] if tag == "S"))
        }) && usize::try_from(*method).ok().and_then(|slot| image.procedures.get(slot)).is_some_and(message_declaration))
}

fn same_constant_slot_comparison(image: &Image, pc: usize) -> bool {
    let Some(commands) = pc
        .checked_sub(3)
        .and_then(|start| image.instructions.get(start..pc))
    else {
        return false;
    };
    // Частный кадр доступен только для чтения; все четыре слота содержат
    // одну константу. Для числа и измеренной строковой формы с 27 метками
    // сравнение с ней сохраняет подтверждённый платформой истинный путь.
    matches!(
        commands,
        [
            Instruction {
                code: 3,
                operand: 0..=3
            },
            Instruction {
                code: 4,
                operand: 0
            },
            Instruction {
                code: 33,
                operand: 0
            }
        ]
    )
}

fn label(method: usize, pc: usize) -> String {
    format!("__НативныйБлок_{method}_{pc}")
}

fn target(
    image: &Image,
    method: usize,
    descriptor: &super::Procedure,
    command: &Instruction,
) -> Result<usize, String> {
    let value = if command.code == 42 {
        usize::try_from(command.operand)
            .ok()
            .and_then(|slot| descriptor.labels.get(slot).map(|value| *value as usize))
    } else {
        usize::try_from(command.operand).ok()
    };
    value
        .filter(|value| *value < image.instructions.len())
        .ok_or_else(|| format!("image/Proc[{method}]: неверная цель перехода"))
}

fn pure_return_tail(image: &Image, pc: usize) -> bool {
    let mut next = pc + 1;
    let mut seen = BTreeSet::new();
    while seen.insert(next) {
        match image.instructions.get(next) {
            Some(Instruction {
                code: 22,
                operand: 0,
            }) => return true,
            Some(Instruction { code: 1, operand }) if *operand > 0 => next += 1,
            Some(Instruction {
                code: 45,
                operand: 0,
            }) => next += 1,
            Some(Instruction { code: 39, operand }) => {
                let Ok(destination) = usize::try_from(*operand) else {
                    return false;
                };
                next = destination;
            }
            _ => return false,
        }
    }
    false
}

fn shared_conditional_return(image: &Image, pc: usize) -> Option<usize> {
    let branch = image.instructions.get(pc)?;
    if !matches!(branch.code, 40 | 41) {
        return None;
    }
    let alternative = usize::try_from(branch.operand)
        .ok()
        .filter(|&alternative| alternative > pc + 1)?;
    let jump = image.instructions.get(alternative.checked_sub(1)?)?;
    if jump.code != 39 {
        return None;
    }
    let join = usize::try_from(jump.operand)
        .ok()
        .filter(|&join| join > alternative)?;
    (image.instructions.get(join)
        == Some(&Instruction {
            code: 17,
            operand: 0,
        })
        && pure_return_tail(image, join))
    .then_some(join)
}

pub(super) fn recover_blocks(
    image: &Image,
    method: usize,
    names: &[&str],
    constants: &[String],
    private: bool,
) -> Result<String, String> {
    recover_blocks_in_frame(
        image,
        method,
        &image.procedures[method],
        names,
        constants,
        private,
    )
}

pub(super) fn recover_initializer(image: &Image, constants: &[String]) -> Result<String, String> {
    // Это контекст перевода верхнего уровня, а не новое объявление.
    // Его метки получают отдельное пространство после существующих методов.
    let descriptor = super::Procedure {
        name: String::new(),
        flags: 0,
        parameter_count: 0,
        entry: image.entry,
        variables: None,
        defaults: None,
        labels: Vec::new(),
    };
    recover_blocks_in_frame(
        image,
        image.procedures.len(),
        &descriptor,
        &[],
        constants,
        false,
    )
}

fn recover_blocks_in_frame(
    image: &Image,
    method: usize,
    descriptor: &super::Procedure,
    names: &[&str],
    constants: &[String],
    private: bool,
) -> Result<String, String> {
    let entry = descriptor.entry as usize;
    let (reachable, predicates) =
        super::reachable_commands_with_labels(image, &[entry], Some(&descriptor.labels));
    let mut leaders = BTreeSet::from([entry]);
    // Недостижимая команда другого участка не разрезает выражение кадра.
    // Заведомо невыполнимая ветвь также не разрезает подготовку вызова;
    // остальные переходы сохраняют цели и таблицу меток текущего метода.
    for (pc, command) in image.instructions.iter().enumerate() {
        if reachable[pc]
            && matches!(command.code, 39..=42)
            && (!matches!(command.code, 40 | 41)
                || predicates[pc].is_none_or(|truth| truth == (command.code == 41)))
            && let Ok(destination) = target(image, method, descriptor, command)
        {
            leaders.insert(destination);
        }
    }
    let mut pending = VecDeque::from([entry]);
    let mut visited = BTreeSet::new();
    let mut source = String::new();
    while let Some(start) = pending.pop_front() {
        if !visited.insert(start) {
            continue;
        }
        source.push_str(&format!("~{}:\n", label(method, start)));
        let mut end = start;
        while let Some(command) = image.instructions.get(end) {
            if matches!(command.code, 17 | 22 | 39..=42 | 44 | 48)
                || (end != start && leaders.contains(&end))
            {
                break;
            }
            if matches!(command.code, 52 | 109)
                || (private && matches!(command.code, 13 | 16 | 20))
                || (private && command.code == 19 && !literal_message_call(image, end))
            {
                return Err(format!(
                    "image/Cmd[{end}]: неизмеренная запись или динамический код кадра"
                ));
            }
            end += 1;
        }
        let command = image
            .instructions
            .get(end)
            .ok_or_else(|| format!("image/Cmd[{end}]: нет завершения блока"))?;
        if !private && let Some(join) = super::backward_boolean_join(image, end) {
            // Нативная scoped-проба сохраняет ссылочную цель под условием.
            // Общее присваивание уже представлено существующим восстановителем;
            // продолжение начинается после пары `False/Assign`, а не внутри неё.
            let body =
                recover_expression_sequence(start..end + 2, names, constants, false, false, image)?;
            source.push_str(&body);
            source.push_str(&format!("\nПерейти ~{};\n", label(method, join + 2)));
            pending.push_back(join + 2);
            continue;
        }
        if !private
            && descriptor.flags != 2
            && let Some(join) = shared_conditional_return(image, end)
        {
            // Обе ветви оставляют значение для общего `Ret`. Уже измеренный
            // восстановитель выражений сохраняет их ленивость и полярность.
            let mut prelude = Vec::new();
            let value = recover_expression_sequence_with_prelude(
                start..join,
                names,
                constants,
                true,
                false,
                image,
                Some(&mut prelude),
            )?;
            for statement in prelude {
                source.push_str(&statement);
                source.push('\n');
            }
            source.push_str(&format!("Возврат {value};\n"));
            continue;
        }
        let control = end == start
            || !leaders.contains(&end)
            || matches!(command.code, 17 | 22 | 39..=42 | 44 | 48);
        let expression = control && matches!(command.code, 17 | 40 | 41);
        let mut prelude = Vec::new();
        let body = if expression {
            recover_expression_sequence_with_prelude(
                start..end,
                names,
                constants,
                true,
                false,
                image,
                Some(&mut prelude),
            )?
        } else {
            recover_expression_sequence(
                start..end,
                names,
                constants,
                expression,
                expression,
                image,
            )?
        };
        for statement in prelude {
            source.push_str(&statement);
            source.push('\n');
        }
        let known_predicate = if private && same_constant_slot_comparison(image, end) {
            Some(true)
        } else {
            predicates[end]
        };
        if !expression && !body.is_empty() {
            source.push_str(&body);
            source.push('\n');
        }
        let destinations = if !control {
            source.push_str(&format!("Перейти ~{};\n", label(method, end)));
            vec![end]
        } else {
            match command.code {
                17 if method < image.procedures.len()
                    && descriptor.flags != 2
                    && pure_return_tail(image, end) =>
                {
                    source.push_str(&format!("Возврат {body};\n"));
                    Vec::new()
                }
                22 if command.operand == 0 => {
                    if method == image.procedures.len() {
                        // Инициализация находится вне процедуры: `Возврат`
                        // здесь запрещён синтаксисом платформы. Все выходы
                        // сходятся к концу текста, не завершая чужой метод.
                        source.push_str(&format!(
                            "Перейти ~{};\n",
                            label(method, image.instructions.len())
                        ));
                    } else {
                        source.push_str("Возврат;\n");
                    }
                    Vec::new()
                }
                39 | 42 => {
                    let destination = target(image, method, descriptor, command)?;
                    source.push_str(&format!("Перейти ~{};\n", label(method, destination)));
                    vec![destination]
                }
                40 | 41 if known_predicate.is_some() => {
                    // Общий анализ уже измеренных чистых булевых префиксов
                    // исключает невозможную ветвь до восстановления её тела.
                    let truth = known_predicate.unwrap();
                    let destination = if truth == (command.code == 41) {
                        target(image, method, descriptor, command)?
                    } else {
                        end + 1
                    };
                    source.push_str(&format!("Перейти ~{};\n", label(method, destination)));
                    vec![destination]
                }
                40 | 41 => {
                    let destination = target(image, method, descriptor, command)?;
                    let condition = if command.code == 40 {
                        format!("НЕ ({body})")
                    } else {
                        body
                    };
                    source.push_str(&format!(
                        "Если {condition} Тогда\nПерейти ~{};\nКонецЕсли;\nПерейти ~{};\n",
                        label(method, destination),
                        label(method, end + 1)
                    ));
                    vec![destination, end + 1]
                }
                48 if !private => {
                    let branch = (end + 1..image.instructions.len())
                        .find(|&pc| image.instructions[pc].code == 40)
                        .ok_or_else(|| format!("image/Cmd[{end}]: нет выхода foreach"))?;
                    let after = usize::try_from(image.instructions[branch].operand)
                        .ok()
                        .and_then(|exit| exit.checked_add(7))
                        .filter(|&after| after < image.instructions.len())
                        .ok_or_else(|| format!("image/Cmd[{end}]: неверная очистка foreach"))?;
                    // Внутренние метки и динамический код цикла не измерены этим профилем.
                    if image.instructions[end..after]
                        .iter()
                        .any(|command| matches!(command.code, 42 | 52 | 109))
                    {
                        return Err(format!(
                            "image/Cmd[{end}]: неизмеренные метки или динамический код foreach"
                        ));
                    }
                    let frame = descriptor.variables.as_deref().unwrap_or_default();
                    let body = super::recover_loop_sequence_in_frame(
                        end..after,
                        names,
                        constants,
                        image,
                        frame,
                    )?;
                    source.push_str(&body);
                    source.push_str(&format!("\nПерейти ~{};\n", label(method, after)));
                    vec![after]
                }
                44 if command.operand == (end + 3) as i64
                    && image.instructions.get(end + 1)
                        == Some(&Instruction {
                            code: 45,
                            operand: 1,
                        })
                    && image.instructions.get(end + 2)
                        == Some(&Instruction {
                            code: 39,
                            operand: (end + 4) as i64,
                        })
                    && image.instructions.get(end + 3)
                        == Some(&Instruction {
                            code: 46,
                            operand: 0,
                        }) =>
                {
                    // Измеренный пустой try не содержит действия, способного
                    // вызвать обработчик; сохраняется только его продолжение.
                    source.push_str(&format!("Перейти ~{};\n", label(method, end + 4)));
                    vec![end + 4]
                }
                _ => {
                    return Err(format!(
                        "image/Cmd[{end}]: неизмеренное управление пересекающихся методов"
                    ));
                }
            }
        };
        pending.extend(destinations);
    }
    if method == image.procedures.len() {
        source.push_str(&format!(
            "~{}:\n;\n",
            label(method, image.instructions.len())
        ));
    }
    Ok(source)
}
