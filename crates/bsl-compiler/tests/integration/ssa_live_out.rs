use bsl_compiler::{cfg, regalloc, ssa};

#[test]
fn a_value_defined_and_used_inside_a_block_is_not_live_in_its_predecessor() {
    let parsed = bsl_syntax::parse(
        "Условие = Истина;\nЕсли Условие Тогда\n А = 1;\n Сообщить(А);\nКонецЕсли;",
    )
    .expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("разрешение имён");
    let slot = resolved
        .top_level
        .locals
        .iter()
        .position(|name| name == "А")
        .expect("локальный слот А") as u32;
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, resolved.top_level.locals.len());
    let (value, block) = form
        .values
        .iter()
        .enumerate()
        .find_map(|(id, value)| match value {
            ssa::Value::Def {
                block,
                slot: defined,
            } if *defined == slot => Some((id, *block)),
            _ => None,
        })
        .expect("значение А определено в ветке");
    assert!(!graph.blocks[block].preds.is_empty());
    let live_out = regalloc::live_out(&graph, &form);
    for &predecessor in &graph.blocks[block].preds {
        assert!(
            !live_out[predecessor].contains(&value),
            "значение из ветки не существует до входа в неё"
        );
    }
}

#[test]
fn an_unused_phi_does_not_keep_its_branch_operand_live() {
    let parsed = bsl_syntax::parse("Условие = Истина;\nЕсли Условие Тогда\n А = 1;\nКонецЕсли;")
        .expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("разрешение имён");
    let slot = resolved
        .top_level
        .locals
        .iter()
        .position(|name| name == "А")
        .expect("локальный слот А") as u32;
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, resolved.top_level.locals.len());
    let (value, block) = form
        .values
        .iter()
        .enumerate()
        .find_map(|(id, value)| match value {
            ssa::Value::Def {
                block,
                slot: defined,
            } if *defined == slot => Some((id, *block)),
            _ => None,
        })
        .expect("значение А определено в ветке");
    assert!(
        form.values
            .iter()
            .any(|value| matches!(value, ssa::Value::Phi { slot: merged, .. } if *merged == slot))
    );
    let live_out = regalloc::live_out(&graph, &form);
    assert!(
        !live_out[block].contains(&value),
        "мёртвое слияние не должно удерживать значение ветки"
    );
}

#[test]
fn a_used_phi_keeps_each_reachable_branch_operand_live() {
    let parsed = bsl_syntax::parse(
        "Условие = Истина;\nА = 0;\nЕсли Условие Тогда\n А = 1;\nКонецЕсли;\nСообщить(А);",
    )
    .expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("разрешение имён");
    let slot = resolved
        .top_level
        .locals
        .iter()
        .position(|name| name == "А")
        .expect("локальный слот А") as u32;
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, resolved.top_level.locals.len());
    let (block, operands) = form
        .values
        .iter()
        .find_map(|value| match value {
            ssa::Value::Phi {
                block,
                slot: merged,
                operands,
            } if *merged == slot => Some((*block, operands)),
            _ => None,
        })
        .expect("слияние значения А");
    let live_out = regalloc::live_out(&graph, &form);
    for (&predecessor, &operand) in graph.blocks[block].preds.iter().zip(operands) {
        if !matches!(form.values[operand], ssa::Value::Bottom) {
            assert!(live_out[predecessor].contains(&operand));
        }
    }
}

#[test]
fn dead_phis_at_one_join_do_not_fill_the_frame() {
    let mut source = String::from("Условие = Истина;\nЕсли Условие Тогда\n");
    for index in 0..300 {
        source.push_str(&format!(" Локальная{index} = {index};\n"));
    }
    source.push_str("КонецЕсли;\nСообщить(1);");
    let parsed = bsl_syntax::parse(&source).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("разрешение имён");
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, resolved.top_level.locals.len());
    assert!(form.phis.iter().any(|phis| phis.len() >= 300));
    let allocation =
        regalloc::allocate_slots_sparse(&graph, &form, resolved.top_level.locals.len(), 0)
            .expect("мёртвые слияния не занимают кадр");
    regalloc::verify_slots(&graph, &form, resolved.top_level.locals.len(), &allocation)
        .expect("раскладка корректна");
}

#[test]
fn numeric_for_counter_phi_is_used_by_the_back_edge() {
    let parsed = bsl_syntax::parse(
        "Сумма = 0;\nДля Сч = 1 По 3 Цикл\n Сумма = Сумма + Сч;\nКонецЦикла;\nСообщить(Сумма);",
    )
    .expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("разрешение имён");
    let slot = resolved
        .top_level
        .locals
        .iter()
        .position(|name| name == "Сч")
        .expect("слот счётчика") as u32;
    let graph = cfg::build(&resolved.top_level.body);
    let form = ssa::build(&graph, resolved.top_level.locals.len());
    let phi = form
        .values
        .iter()
        .enumerate()
        .find_map(|(id, value)| match value {
            ssa::Value::Phi { slot: merged, .. } if *merged == slot => Some(id),
            _ => None,
        })
        .expect("слияние счётчика");
    assert!(
        form.uses.iter().any(|use_site| use_site.value == phi),
        "обратное ребро числового цикла читает счётчик"
    );
}
