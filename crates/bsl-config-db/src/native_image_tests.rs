use super::{Instruction, Variable, parse};
use crate::native_test_output::Output;
use open_sdbl::metadata::Value;

const NATIVE_CONSTANT_IMAGE: &str =
    include_str!("../../../tests/conformance/measure/1c/native-image-constant.image");

#[test]
fn zero_argument_private_async_lookup_matches_native_promise_type() {
    let bytes = include_bytes!("../../../docs/research/fixtures/native-local-async-eval/image");
    assert_native_driver(
        bytes,
        include_str!(
            "../../../docs/research/fixtures/native-local-async-eval/measure-local-async-eval.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/native-local-async-eval/measure-local-async-eval.platform.txt"
        ),
    );
    for (command, operand) in [(7, 1), (8, 1), (9, 9), (10, 1)] {
        let mut malformed = parse(bytes).unwrap();
        malformed.instructions[command].operand = operand;
        assert!(super::recover_module(&malformed).is_err());
    }
    let mut malformed = parse(bytes).unwrap();
    malformed.procedures[1].entry = 10_000;
    assert!(super::recover_module(&malformed).is_err());
    let mut malformed = parse(bytes).unwrap();
    malformed.procedures[1].parameter_count = 1;
    assert!(super::recover_module(&malformed).is_err());
}

#[test]
fn dynamic_private_lookup_matches_native_and_rejects_bad_operands() {
    let bytes = include_bytes!("../../../docs/research/fixtures/native-local-eval/image");
    assert_native_driver(
        bytes,
        include_str!("../../../docs/research/fixtures/native-local-eval/measure-local-eval.bsl"),
        include_str!(
            "../../../docs/research/fixtures/native-local-eval/measure-local-eval.platform.txt"
        ),
    );
    let mut image = parse(bytes).unwrap();
    image.instructions[1].operand = 1;
    assert!(super::recover_module(&image).is_err());
    image.instructions[1].operand = 0;
    image.instructions[0] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn empty_function_handler_matches_native_and_rejects_bad_return_tails() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-function-guard.image");
    assert_native_driver(
        bytes,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-function-guard.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-function-guard.platform.txt"
        ),
    );
    for (index, operand) in [(6, 1), (7, 9), (8, 1)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(
            super::recover_module(&image).is_err(),
            "Cmd[{index}]={operand}"
        );
    }
}

#[test]
fn backward_boolean_assignment_matches_native_reference_updates() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-backward-bool.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-backward-bool.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-backward-bool.platform.txt"
        ),
    );
}

#[test]
fn backward_boolean_join_rejects_malformed_targets_and_other_entries() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-backward-bool.image");
    for (index, operand) in [(10, 2), (12, 1), (1, 1), (2, 1), (11, 1), (4, 1)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        if index == 4 {
            image.instructions[index].code = 39;
        }
        assert!(
            super::backward_boolean_join(&image, 10).is_none(),
            "Cmd[{index}]={operand}"
        );
        assert!(super::recover_module(&image).is_err());
    }
    for entry in [1, 2, 11, 12] {
        let mut image = parse(bytes).unwrap();
        image.entry = entry;
        assert!(super::backward_boolean_join(&image, 10).is_none());
    }
}

#[test]
fn left_substring_matches_native_and_rejects_malformed_operands() {
    let bytes = include_bytes!("../../../tests/conformance/measure/1c/native-image-left.image");
    assert_native_driver(
        bytes,
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-left.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-left.platform.txt"
        ),
    );
    let mut image = parse(bytes).unwrap();
    image.instructions[2].operand = 1;
    assert!(super::recover_module(&image).is_err());
    image.instructions[2].operand = 0;
    image.instructions[1] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn empty_handler_in_shared_blocks_preserves_native_reference_effects() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-empty-guard.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-empty-guard.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-empty-guard.platform.txt"
        ),
    );
}

#[test]
fn empty_handler_blocks_reject_malformed_boundaries_and_interior_entries() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-empty-guard.image"
    );
    let recover = |image: &super::Image| {
        super::recover_labelled_statements(
            0..25,
            &["Остановить", "Аргумент"],
            &["10".into(), "1".into(), "0".into(), "3".into()],
            image,
        )
    };
    for (index, operand) in [(0, 2), (1, 0), (2, 5), (3, 1), (5, 1), (5, 2), (5, 3)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(recover(&image).is_err(), "Cmd[{index}]={operand}");
    }
    for entry in 1..4 {
        let mut image = parse(bytes).unwrap();
        image.entry = entry;
        assert!(recover(&image).is_err(), "entry={entry}");
        image.entry = 0;
        image.procedures[0].entry = entry;
        assert!(recover(&image).is_err(), "procedure entry={entry}");
    }
}

#[test]
fn shared_procedure_blocks_match_native_and_independent_labelled_bsl() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-shared-loop.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-shared-loop.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-shared-loop.platform.txt"
        ),
    );
    assert_recovered_source(
        String::new(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-procedure-shared-loop-source.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-procedure-shared-loop-source.platform.txt"
        ),
    );
}

#[test]
fn known_dead_condition_after_exit_preserves_native_reference_results() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-dead-condition.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-dead-condition.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-dead-condition.platform.txt"
        ),
    );
}

#[test]
fn terminal_iteration_branches_preserve_native_reference_effects() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-foreach-terminal-branches.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-terminal-branches.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-terminal-branches.platform.txt"
        ),
    );
}

#[test]
fn single_jump_branch_and_backedge_targets_preserve_native_collection_receipts() {
    for (image, driver, expected) in [
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-branch-trampoline.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-foreach-branch-trampoline.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-foreach-branch-trampoline.platform.txt"),
        ),
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-backedge-trampoline.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-foreach-backedge-trampoline.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-foreach-backedge-trampoline.platform.txt"),
        ),
    ] {
        assert_native_driver(image, driver, expected);
    }
}

#[test]
fn terminal_iteration_branches_keep_structural_jumps_and_reject_wrong_checks() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-terminal-branches.image"
    );
    let mut image = parse(bytes).unwrap();
    super::normalize_measured_jumps(&mut image);
    assert_eq!(
        image.instructions[37],
        Instruction {
            code: 39,
            operand: 46
        }
    );
    assert_eq!(
        image.instructions[45],
        Instruction {
            code: 39,
            operand: 6
        }
    );
    for target in [0, 5, 7, 48, 99] {
        let mut image = parse(bytes).unwrap();
        image.instructions[29].operand = target;
        super::normalize_measured_jumps(&mut image);
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
}

#[test]
fn dead_condition_normalization_keeps_executable_entries_and_unknown_tails() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-dead-condition.image"
    );
    for entry in [10, 11] {
        let mut image = parse(bytes).unwrap();
        image.entry = entry;
        super::normalize_measured_jumps(&mut image);
        assert_eq!(
            image.instructions[entry as usize].code,
            if entry == 10 { 3 } else { 40 }
        );
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[2].operand = 10;
    super::normalize_measured_jumps(&mut image);
    assert_eq!(image.instructions[10].code, 3);
    let mut image = parse(bytes).unwrap();
    image.instructions[10] = Instruction {
        code: 99,
        operand: 9999,
    };
    super::normalize_measured_jumps(&mut image);
    assert_eq!(image.instructions[10].code, 99);
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn shared_procedure_blocks_reject_bad_targets_stack_transfers_and_unknown_commands() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-shared-loop.image"
    );
    for (index, operand) in [(1, -1), (1, 22), (1, 99), (12, 18), (20, 10)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(
            super::recover_module(&image).is_err(),
            "Cmd[{index}]={operand}"
        );
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[7] = Instruction {
        code: 99,
        operand: 9999,
    };
    assert!(super::recover_module(&image).is_err());
    for code in [44, 46, 48, 49, 50] {
        let mut image = parse(bytes).unwrap();
        image.instructions[7].code = code;
        assert!(
            super::recover_labelled_statements(
                0..21,
                &["Остановить", "Аргумент"],
                &["10".into(), "1".into(), "0".into(), "3".into()],
                &image,
            )
            .is_err(),
            "code={code}"
        );
    }
}

#[test]
fn trampoline_normalization_keeps_cycles_chains_and_foreign_targets() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-branch-trampoline.image"
    );
    for target in [1, 31, 99] {
        let mut image = parse(bytes).unwrap();
        image.instructions[1].operand = target;
        super::normalize_measured_jumps(&mut image);
        assert_eq!(image.instructions[31].operand, 1, "target={target}");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[1].operand = 60;
    assert_eq!(image.instructions[60].code, 39);
    super::normalize_measured_jumps(&mut image);
    assert_eq!(image.instructions[31].operand, 1);
    let mut image = parse(bytes).unwrap();
    image.procedures.push(super::Procedure {
        name: "ДругаяПроба".into(),
        flags: 17,
        parameter_count: 0,
        entry: 2,
        variables: None,
        defaults: None,
        labels: Vec::new(),
    });
    super::normalize_measured_jumps(&mut image);
    assert_eq!(image.instructions[31].operand, 1);
}

#[test]
fn unreachable_join_after_procedure_return_preserves_the_complete_alternative() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-dead-join.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-dead-join.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-dead-join.platform.txt"
        ),
    );
}

#[test]
fn dead_return_join_keeps_other_entries_and_unknown_alternative_commands() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-dead-join.image"
    );
    let mut image = parse(bytes).unwrap();
    image.entry = 10;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[11] = Instruction {
        code: 99,
        operand: 9999,
    };
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[9].operand = 10;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn compact_procedure_return_matches_native_reference_effects() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-compact-return.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-compact-return.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-compact-return.platform.txt"
        ),
    );
}

#[test]
fn compact_procedure_return_rejects_bad_exits_and_unknown_alternative_commands() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-compact-return.image"
    );
    for target in [0, 10, 14, 17, 99] {
        let mut image = parse(bytes).unwrap();
        image.instructions[9].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[10] = Instruction {
        code: 99,
        operand: 9999,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn procedure_early_exit_directly_to_end_preserves_native_reference_updates() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-exit-no-line.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-exit-no-line.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-exit-no-line.platform.txt"
        ),
    );
}

#[test]
fn procedure_exit_to_end_rejects_nonterminal_targets_and_damaged_end() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-exit-no-line.image"
    );
    for target in [0, 11, 14, 17, 99] {
        let mut image = parse(bytes).unwrap();
        image.instructions[9].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[16].operand = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn compact_conditional_collection_break_matches_native_parameter_effects() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-foreach-compact-break.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-compact-break.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-compact-break.platform.txt"
        ),
    );
}

#[test]
fn compact_conditional_break_rejects_unvalidated_cleanup_targets() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-compact-break.image"
    );
    for target in [0, 12, 47, 48, 99] {
        let mut image = parse(bytes).unwrap();
        assert_eq!(image.instructions[37].code, 39);
        image.instructions[37].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
}

#[test]
fn source_numeric_conversion_matches_the_independent_native_receipt() {
    assert_recovered_source(
        String::new(),
        include_str!("../../../tests/conformance/measure/1c/measure-number-conversion-source.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-number-conversion-source.platform.txt"
        ),
    );
}

#[test]
fn numeric_conversion_preserves_native_values_types_errors_and_reference_assignment() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-number-conversion.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-number-conversion.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-number-conversion.platform.txt"
        ),
    );
}

#[test]
fn numeric_conversion_rejects_reserved_operands_and_missing_input() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-number-conversion.image"
    );
    for index in [1, 8] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[0] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn renamed_iterator_storage_preserves_native_collection_traversal() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-foreach-renamed-iterator.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-renamed-iterator.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-renamed-iterator.platform.txt"
        ),
    );
}

#[test]
fn renamed_iterator_rejects_nonlocal_flags_and_inconsistent_slots() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-renamed-iterator.image"
    );
    for flags in [0, 5, 13] {
        let mut image = parse(bytes).unwrap();
        image.procedures[0].variables.as_mut().unwrap()[5].flags = flags;
        assert!(super::recover_module(&image).is_err(), "flags={flags}");
    }
    for index in [6, 9, 13, 15, 48, 51] {
        let mut image = parse(bytes).unwrap();
        assert_eq!(image.instructions[index].code, 3);
        image.instructions[index].operand = 4;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn external_declarations_do_not_replace_collection_loop_local_storage() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-renamed-iterator.image"
    ))
    .unwrap();
    image.procedures.push(super::Procedure {
        name: "ВнешняяПроба".into(),
        flags: 33,
        parameter_count: 0,
        entry: 0,
        variables: None,
        defaults: None,
        labels: Vec::new(),
    });
    assert!(super::recover_module(&image).is_ok());
}

#[test]
fn collection_loop_terminal_jump_to_the_preceding_back_edge_matches_native() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-foreach-backward-tail.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-backward-tail.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-backward-tail.platform.txt"
        ),
    );
}

#[test]
fn collection_loop_backward_tail_rejects_targets_outside_the_validated_check() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-backward-tail.image"
    );
    for (index, operand) in [(45, 5), (45, 7), (46, 5), (46, 7), (46, 44)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn collection_loop_without_a_check_line_and_with_a_trampoline_matches_native() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-foreach-no-line-trampoline.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-no-line-trampoline.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-no-line-trampoline.platform.txt"
        ),
    );
}

#[test]
fn collection_loop_trampoline_rejects_damaged_check_back_edge_and_cleanup() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-foreach-no-line-trampoline.image"
    );
    for (index, operand) in [(7, 1), (11, 46), (45, 5), (46, 7), (47, 0)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn constant_short_circuit_predicates_preserve_all_four_native_results() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-constant-short-circuit.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-constant-short-circuit.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-constant-short-circuit.platform.txt"
        ),
    );
}

#[test]
fn constant_short_circuit_normalization_preserves_unknown_inputs_and_other_entries() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-constant-short-circuit.image"
    );
    for index in [0, 2] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 3,
            operand: 0,
        };
        super::normalize_measured_jumps(&mut image);
        assert_eq!(image.instructions[4].code, 40, "Cmd[{index}]");
        assert_eq!(image.instructions[9].code, 99);
    }
    for entry in [2, 9] {
        let mut image = parse(bytes).unwrap();
        image.procedures[1].entry = entry;
        super::normalize_measured_jumps(&mut image);
        assert_eq!(image.instructions[4].code, 40, "entry={entry}");
        assert_eq!(image.instructions[9].code, 99);
    }
    for target in [2, 9] {
        let mut image = parse(bytes).unwrap();
        let entry = image.procedures[1].entry as usize;
        image.instructions[entry] = Instruction {
            code: 39,
            operand: target,
        };
        super::normalize_measured_jumps(&mut image);
        assert_eq!(image.instructions[4].code, 40, "target={target}");
        assert_eq!(image.instructions[9].code, 99);
    }
}

#[test]
fn boolean_conversion_short_circuit_boundaries_preserve_native_reference_effects() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-short-circuit32.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-short-circuit32.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-short-circuit32.platform.txt"
        ),
    );
}

#[test]
fn boolean_conversion_short_circuit_rejects_malformed_boundaries() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-short-circuit32.image");
    for index in [6, 17] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    for target in [0, 2, 12, 99] {
        let mut image = parse(bytes).unwrap();
        image.instructions[1].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
}

#[test]
fn next_command_jumps_preserve_native_values_conditions_and_reference_assignment() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-next-jumps.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-next-jumps.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-next-jumps.platform.txt"
        ),
    );
}

#[test]
fn explicit_boolean_conversion_preserves_native_types_errors_and_reference_assignment() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-opcode32-values.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-opcode32-values.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-opcode32-values.platform.txt"
        ),
    );
}

#[test]
fn measured_nonzero_boolean_conversion_operand_matches_native_results() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-opcode32-operand/image"),
        include_str!("../../../docs/research/fixtures/native-opcode32-operand/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-opcode32-operand/platform.txt"),
    );
}

#[test]
fn explicit_boolean_conversion_rejects_reserved_operands_and_missing_input() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-opcode32-values.image");
    for index in [1, 8] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[0] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn mutually_referencing_unreachable_regions_preserve_the_native_receipt() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-unreachable-cycle.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-unreachable-cycle.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-unreachable-cycle.platform.txt"
        ),
    );
}

#[test]
fn a_local_label_back_edge_preserves_three_native_loop_iterations() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-goto-backward.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-goto-backward.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-goto-backward.platform.txt"
        ),
    );
}

#[test]
fn measured_boolean_prefixes_preserve_all_six_native_results() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-boolean-skips.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-boolean-skips.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-boolean-skips.platform.txt"
        ),
    );
}

#[test]
fn method_local_label_jumps_match_native_results_with_and_without_global_labels() {
    for (image, driver, receipt) in [
        (
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-goto-labels-local.image"
            )
            .as_slice(),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-goto-labels-local.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-goto-labels-local.platform.txt"
            ),
        ),
        (
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-goto-labels-both.image"
            )
            .as_slice(),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-goto-labels-both.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-goto-labels-both.platform.txt"
            ),
        ),
    ] {
        assert_native_driver(image, driver, receipt);
    }
}

#[test]
fn local_label_jumps_reject_missing_indices_and_targets_outside_the_image() {
    let original =
        include_str!("../../../tests/conformance/measure/1c/native-image-goto-labels-both.image");
    for (from, to) in [
        ("{42,0}", "{42,1}"),
        ("{42,0}", "{42,-1}"),
        ("\"МеткаПервойПробы\",3", "\"МеткаПервойПробы\",99"),
    ] {
        assert!(original.contains(from));
        assert!(super::decompile(original.replacen(from, to, 1).as_bytes()).is_err());
    }
    assert!(
        super::decompile(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-goto-labels-root.image"
        ))
        .is_err()
    );
}

#[test]
fn an_unknown_but_valid_parameter_condition_keeps_unsupported_commands_reachable() {
    let original =
        include_str!("../../../tests/conformance/measure/1c/native-image-unreachable-probe.image");
    let bytes = original
        .replacen("{6,0},{6,0},{33,0}", "{3,0},{6,0},{33,0}", 1)
        .replacen(
            "\"ИстиннаяПробы\",17,0,0",
            "\"ИстиннаяПробы\",17,1,0,{\"Var\",1,{\"АргументПробы\",13,-1}},{\"DefPrm\",1,{\"\"}}",
            1,
        );
    let error = super::decompile(bytes.as_bytes()).unwrap_err();
    assert!(error.contains("Cmd[4]"), "{error}");
}

#[test]
fn incoming_edges_keep_the_original_unsupported_command_diagnostic() {
    let original =
        include_str!("../../../tests/conformance/measure/1c/native-image-unreachable-probe.image");
    for target in [2, 5] {
        let bytes = original.replacen("{39,31}", &format!("{{39,{target}}}"), 1);
        let error = super::decompile(bytes.as_bytes()).unwrap_err();
        assert!(error.contains("Cmd[4]"), "{error}");
    }
}

#[test]
fn native_proven_unreachable_regions_preserve_all_four_execution_results() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-unreachable-probe.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-unreachable-probe.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-unreachable-probe.platform.txt"
        ),
    );
}

#[test]
fn unreachable_normalization_does_not_hide_reachable_corruption_or_other_entries() {
    let original =
        include_str!("../../../tests/conformance/measure/1c/native-image-unreachable-probe.image");
    for (from, to) in [
        ("{41,10}", "{41,4}"),
        ("{6,0},{6,0},{33,0}", "{3,0},{6,0},{33,0}"),
        ("{33,0},{41,10}", "{33,1},{41,10}"),
        ("{41,22}", "{41,99}"),
        ("{39,31}", "{39,99}"),
        ("{39,31}", "{39,5}"),
        ("{39,31}", "{39,2}"),
        ("\"СледующаяПробы\",17,0,22", "\"СледующаяПробы\",17,0,5"),
    ] {
        assert!(original.contains(from));
        assert!(
            super::decompile(original.replacen(from, to, 1).as_bytes()).is_err(),
            "{from} -> {to}"
        );
    }
}

#[test]
fn true_predicate_jumps_preserve_native_branch_values_updates_and_laziness() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-jump-true.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-jump-true.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-jump-true.platform.txt"
        ),
    );
}

#[test]
fn true_predicate_jumps_reject_invalid_alternative_and_continuation_targets() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-jump-true.image");
    let original = parse(bytes).unwrap();
    for index in [3, 19, 30, 40] {
        assert_eq!(original.instructions[index].code, 41);
        for target in [-1, index as i64, 99] {
            let mut image = parse(bytes).unwrap();
            image.instructions[index].operand = target;
            assert!(
                super::recover_module(&image).is_err(),
                "Cmd[{index}] -> {target}"
            );
        }
    }
    for index in [7, 21, 34, 42] {
        assert_eq!(original.instructions[index].code, 39);
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 99;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn supported_bodies_without_line_instructions_match_the_native_receipt() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-no-lines.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-no-lines.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-no-lines.platform.txt"
        ),
    );
}

#[test]
fn function_tails_without_lines_reject_invalid_exit_operands_and_targets() {
    let image = include_str!("../../../tests/conformance/measure/1c/native-image-no-lines.image");
    for (from, to) in [
        ("{45,0},{39,4}", "{45,1},{39,4}"),
        ("{39,4},{22,0}", "{39,3},{22,0}"),
        ("{39,20},{22,0}", "{39,19},{22,0}"),
        ("{39,20},{22,0}", "{39,20},{22,1}"),
    ] {
        assert!(image.contains(from));
        let error = super::decompile(image.replacen(from, to, 1).as_bytes()).unwrap_err();
        assert!(error.contains("image/Proc["), "{error}");
    }
}

#[test]
fn unused_native_label_tables_preserve_loop_results_and_reference_effects() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-label-tables.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-label-tables.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-label-tables.platform.txt"
        ),
    );
}

#[test]
fn label_tables_reject_duplicate_sections_and_malformed_records() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-label-tables.image");
    let text = crate::scsu::decode(bytes).unwrap();
    let root = "{\"Lbl\",1,{\"ГлобальнаяМеткаПробы\",0}}";
    let local = "{\"Lbl\",2,{\"ЛокальнаяМеткаПробы\",0},{\"МеткаКонцаПробы\",35}}";
    assert!(text.contains(root));
    assert_eq!(text.matches(local).count(), 2);
    for (from, to, at) in [
        (root, format!("{root},{root}"), "image/Lbl"),
        (local, format!("{local},{local}"), "image/Proc[0]/Lbl"),
        (
            "{\"ГлобальнаяМеткаПробы\",0}",
            "{\"ГлобальнаяМеткаПробы\",0,1}".into(),
            "image/Lbl[0]",
        ),
        (
            "{\"ЛокальнаяМеткаПробы\",0}",
            "{\"ЛокальнаяМеткаПробы\",-1}".into(),
            "image/Proc[0]/Lbl[0]",
        ),
        (
            "{\"ГлобальнаяМеткаПробы\",0}",
            "{1,0}".into(),
            "image/Lbl[0]",
        ),
    ] {
        let error = parse(text.replacen(from, &to, 1).as_bytes()).unwrap_err();
        assert!(error.contains(at), "{error}");
    }
}

#[test]
fn numeric_table_headers_do_not_replace_the_actual_records() {
    macro_rules! verify {
        ($name:literal) => {
            assert_native_driver(
                include_bytes!(concat!(
                    "../../../tests/conformance/measure/1c/native-image-table-count-",
                    $name,
                    ".image"
                )),
                include_str!(concat!(
                    "../../../tests/conformance/measure/1c/measure-native-image-table-count-",
                    $name,
                    ".bsl"
                )),
                include_str!(concat!(
                    "../../../tests/conformance/measure/1c/measure-native-image-table-count-",
                    $name,
                    ".platform.txt"
                )),
            );
        };
    }
    verify!("cmd");
    verify!("const");
    verify!("var");
    verify!("proc");
    verify!("cmd-small");
    verify!("const-small");
    verify!("proc-small");
    verify!("local-var");
    verify!("local-var-small");
    verify!("defaults");
    verify!("defaults-small");
}

#[test]
fn the_opt_in_sixth_buh_image_matches_independent_icu_transcoding() {
    let Some(path) = std::env::var_os("OPEN_BSL_BUH_SCSU_IMAGE") else {
        eprintln!("SCSU-образ buh не задан; частная проверка пропущена");
        return;
    };
    let bytes = std::fs::read(&path).unwrap();
    let expected = std::fs::read(format!(
        "{}.icu.utf8",
        std::path::Path::new(&path).display()
    ))
    .unwrap();
    assert_eq!(crate::scsu::decode(&bytes).unwrap().as_bytes(), expected);
    match super::decompile(&bytes) {
        Ok(source) => eprintln!("Восстановлено байт: {}", source.len()),
        Err(error) => eprintln!("Следующая граница восстановления: {error}"),
    }
}

#[test]
fn native_scsu_image_preserves_the_complete_compiler_receipt() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-scsu.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-scsu.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scsu.platform.txt"
        ),
    );
}

#[test]
fn native_counted_loop_exits_preserve_reference_counters_and_trace() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-counted-loop-exits.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-counted-loop-exits.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-counted-loop-exits.platform.txt"
        ),
    );
}

#[test]
fn counted_loop_exits_reject_invalid_targets_and_unknown_commands() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-counted-loop-exits.image"
    );
    let mut image = parse(bytes).unwrap();
    image.instructions[19].operand = 31;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[18] = Instruction {
        code: 9999,
        operand: 0,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_conditional_assignment_and_later_statement_branches_share_a_method() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-mixed-conditional.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-mixed-conditional.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-mixed-conditional.platform.txt"
        ),
    );
}

#[test]
fn mixed_conditional_recovery_does_not_hide_unknown_commands_in_either_construct() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-mixed-conditional.image"
    );
    for index in [6, 15, 20] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 9999,
            operand: 0,
        };
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn native_length_and_trim_preserve_results_before_operand_mutation() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-length-trim-snapshots.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-length-trim-snapshots.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-length-trim-snapshots.platform.txt"
        ),
    );
}

#[test]
fn length_and_trim_reject_invalid_operands_and_missing_arguments() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-length-trim-snapshots.image"
    );
    for code in [53, 56] {
        let mut image = parse(bytes).unwrap();
        let index = image
            .instructions
            .iter()
            .position(|instruction| instruction.code == code)
            .unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err());
        let mut image = parse(bytes).unwrap();
        image.instructions[index - 1] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn native_right_substring_and_blank_string_preserve_results_before_mutation() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-remaining-string-snapshots.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-remaining-string-snapshots.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-remaining-string-snapshots.platform.txt"
        ),
    );
}

#[test]
fn right_substring_and_blank_string_reject_invalid_operands_and_arguments() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-remaining-string-snapshots.image"
    );
    for code in [58, 65] {
        let mut image = parse(bytes).unwrap();
        let index = image
            .instructions
            .iter()
            .position(|instruction| instruction.code == code)
            .unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err());
        let mut image = parse(bytes).unwrap();
        image.instructions[index - 1] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn native_string_conversion_preserves_its_result_before_operand_mutation() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-string-snapshot.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-string-snapshot.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-string-snapshot.platform.txt"
        ),
    );
}

#[test]
fn string_conversion_rejects_invalid_operands_and_missing_arguments() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-string-snapshot.image");
    let mut image = parse(bytes).unwrap();
    let index = image
        .instructions
        .iter()
        .position(|instruction| instruction.code == 71)
        .unwrap();
    image.instructions[index].operand = 1;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[index - 1] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_uppercase_preserves_its_value_before_operand_mutation() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-upper-snapshot.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-upper-snapshot.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-upper-snapshot.platform.txt"
        ),
    );
}

#[test]
fn uppercase_rejects_nonzero_operands_and_missing_arguments() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-upper-snapshot.image");
    let mut image = parse(bytes).unwrap();
    let index = image
        .instructions
        .iter()
        .position(|instruction| instruction.code == 61)
        .unwrap();
    image.instructions[index].operand = 1;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[index - 1] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_builtin_values_are_preserved_before_a_mutating_call() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-builtin-snapshots.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-builtin-snapshots.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-builtin-snapshots.platform.txt"
        ),
    );
}

#[test]
fn native_scalar_operator_values_are_preserved_before_a_mutating_call() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-operator-snapshots.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-operator-snapshots.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-operator-snapshots.platform.txt"
        ),
    );
}

#[test]
fn native_expression_value_is_preserved_before_a_later_mutating_call() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-expression-snapshot.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-expression-snapshot.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-expression-snapshot.platform.txt"
        ),
    );
}

#[test]
fn independently_authored_extended_source_matches_its_native_execution_receipt() {
    assert_recovered_source(
        include_str!(
            "../../../tests/conformance/measure/1c/native-image-extended-control-module.bsl"
        )
        .to_owned(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-extended-source.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-extended-source.platform.txt"
        ),
    );
}

#[test]
fn final_module_terminator_preserves_the_complete_last_method() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-external-omitted.image");
    let mut image = parse(bytes).unwrap();
    let source = super::recover_module(&image).unwrap();
    image.instructions.push(Instruction {
        code: 22,
        operand: 0,
    });
    assert_eq!(super::recover_module(&image).unwrap(), source);
    image.instructions.push(Instruction {
        code: 22,
        operand: 0,
    });
    assert!(super::recover_module(&image).is_err());
    image.instructions.pop();
    image.instructions.last_mut().unwrap().operand = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn a_small_module_with_an_external_function_uses_general_recovery() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-external-function-nstr-hole.image"
    );
    let mut image = parse(bytes).unwrap();
    image.procedures[1].parameter_count = 1;
    image.instructions[2] = Instruction {
        code: 1,
        operand: 1,
    };
    image.instructions[3].operand = 1;
    let source = super::recover_module(&image).unwrap();
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-function-nstr-hole.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-function-nstr-hole.platform.txt"
        ),
    );
}

#[test]
fn native_external_function_omission_executes_through_the_facade() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-external-function-nstr-hole.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-function-nstr-hole.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-function-nstr-hole.platform.txt"
        ),
    );
}

#[test]
fn the_measured_temporary_storage_call_preserves_the_omitted_first_position() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-external-function-hole.image"
    );
    let source = super::decompile(bytes).unwrap();
    assert!(source.contains("ПоместитьВоВременноеХранилище(, ИдентификаторПробы)"));
    let engine = open_bsl::Engine::builder().build().unwrap();
    engine.compile(&source).unwrap();
    let mut image = parse(bytes).unwrap();
    image.instructions[3].operand = 3;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_external_procedure_omissions_preserve_optional_positions() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-external-omitted.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-omitted.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-omitted.platform.txt"
        ),
    );
}

#[test]
fn external_omissions_reject_overlong_calls_and_unknown_function_flags() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-external-omitted.image");
    let mut image = parse(bytes).unwrap();
    image.instructions[5].operand = 5;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.procedures[1].flags = 34;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_statement_try_preserves_normal_handler_and_suffix_effects() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-statement-try.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-statement-try.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-statement-try.platform.txt"
        ),
    );
}

#[test]
fn statement_try_rejects_invalid_boundaries_and_error_snapshot_operand() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-statement-try.image");
    for (index, operand) in [(4, 3), (4, 41), (17, 18), (17, 42), (20, 1), (32, 1)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(
            super::recover_module(&image).is_err(),
            "Cmd[{index}]={operand}"
        );
    }
    for index in [16, 32] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 99,
            operand: 0,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_counted_loop_evaluates_its_context_limit_before_body_mutations() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-counted-limit-call.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-counted-limit-call.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-counted-limit-call.platform.txt"
        ),
    );
}

#[test]
fn counted_loop_context_limit_rejects_bad_return_read_and_header() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-counted-limit-call.image"
    );
    for index in [8, 9, 15, 17, 33] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 99,
            operand: 0,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_while_continuation_rechecks_condition_and_skips_remaining_updates() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-while-continue.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-while-continue.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-while-continue.platform.txt"
        ),
    );
}

#[test]
fn while_continuation_rejects_wrong_condition_entry_and_unknown_tail() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-while-continue.image");
    for target in [1, 3, 19, 26, 39, 45, 47] {
        let mut image = parse(bytes).unwrap();
        image.instructions[23].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[24] = Instruction {
        code: 128,
        operand: 0,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_function_early_returns_preserve_selected_value_and_skip_other_branches() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-function-early-return.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-function-early-return.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-function-early-return.platform.txt"
        ),
    );
}

#[test]
fn function_early_returns_reject_wrong_exit_and_unknown_tail() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-function-early-return.image"
    );
    for index in [7, 18] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 29;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    for index in [6, 7, 17, 18] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 128,
            operand: 0,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_while_break_preserves_updates_and_forward_continuation() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-while-break.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-while-break.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-while-break.platform.txt"
        ),
    );
}

#[test]
fn while_break_rejects_unmeasured_targets_and_preserves_an_adjacent_jump_with_a_value() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-while-break.image");
    for target in [1, 19, 26, 28, 34, 35] {
        let mut image = parse(bytes).unwrap();
        image.instructions[17].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[27].operand = 29;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[29] = Instruction {
        code: 39,
        operand: 30,
    };
    image.instructions[28] = Instruction {
        code: 3,
        operand: 0,
    };
    // Нативная проба `measure-native-image-next-jumps` сняла прежнюю
    // неопределённость: переход к следующей команде сохраняет значение.
    assert!(super::recover_module(&image).is_ok());
}

#[test]
fn native_collection_break_preserves_prior_effects_and_leaves_the_loop() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-break.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-break.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-break.platform.txt"
        ),
    );
}

#[test]
fn collection_break_rejects_wrong_cleanup_and_hidden_unsupported_commands() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-break.image");
    for target in [6, 39, 45, 48, 59, 61] {
        let mut image = parse(bytes).unwrap();
        image.instructions[37].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
    for index in [38, 47] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 128,
            operand: 0,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_nested_conditional_return_preserves_selected_value_and_lazy_branches() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-conditional-return.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-conditional-return.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-conditional-return.platform.txt"
        ),
    );
}

#[test]
fn nested_conditional_return_rejects_invalid_joins_and_unknown_branch_commands() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-conditional-return.image"
    );
    for (index, operand) in [(2, 1), (2, 19), (4, 5), (4, 20), (8, 8), (10, 11), (10, 20)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(
            super::recover_module(&image).is_err(),
            "Cmd[{index}]={operand}"
        );
    }
    for index in [3, 9, 13] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 128,
            operand: 0,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_conditional_expression_preserves_assignment_and_lazy_evaluation() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-conditional-expression.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-conditional-expression.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-conditional-expression.platform.txt"
        ),
    );
}

#[test]
fn conditional_expression_rejects_invalid_boundaries_and_branch_stack() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-conditional-expression.image"
    );
    for (index, operand) in [(3, 2), (3, 16), (5, 6), (5, 17)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(
            super::recover_module(&image).is_err(),
            "Cmd[{index}]={operand}"
        );
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[8] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_collection_continuation_preserves_iteration_and_reference_updates() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-continue.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-continue.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-continue.platform.txt"
        ),
    );
}

#[test]
fn collection_continuation_rejects_wrong_targets_and_hidden_unsupported_code() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-continue.image");
    for target in [6, 39, 45, 48, 59, 61] {
        let mut image = parse(bytes).unwrap();
        image.instructions[37].operand = target;
        assert!(super::recover_module(&image).is_err(), "target={target}");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[38] = Instruction {
        code: 128,
        operand: 0,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_scalar_builtins_preserve_calendar_parts_rounding_and_substrings() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-scalar-builtins.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scalar-builtins.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scalar-builtins.platform.txt"
        ),
    );
}

#[test]
fn scalar_builtins_reject_nonzero_operands_and_missing_arguments() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-scalar-builtins.image");
    for index in [2, 10, 19, 29] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
        let mut image = parse(bytes).unwrap();
        image.instructions[index - 1] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_context_calls_preserve_seven_to_nine_arguments_and_omitted_positions() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-context-nine-arguments.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-nine-arguments.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-nine-arguments.platform.txt"
        ),
    );
}

#[test]
fn larger_context_calls_reject_invalid_counts_and_a_missing_receiver() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-context-nine-arguments.image"
    );
    for count in [-1, 10] {
        let mut image = parse(bytes).unwrap();
        image.instructions[9].operand = count;
        assert!(super::recover_module(&image).is_err());
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[1] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_discarded_function_results_preserve_calls_and_later_reference_effects() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-discarded-results.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-discarded-results.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-discarded-results.platform.txt"
        ),
    );
}

#[test]
fn discarded_results_do_not_allow_reading_an_unmeasured_result_after_a_procedure() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-discarded-results.image"
    );
    let image = parse(bytes).unwrap();
    let entry = image.procedures[4].entry as usize;
    let end = super::method_end(&image, image.procedures[4].entry);
    let returned = (entry..end)
        .find(|&index| image.instructions[index].code == 17)
        .unwrap();
    let mut image = parse(bytes).unwrap();
    image.instructions[returned - 1] = Instruction {
        code: 5,
        operand: 0,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_local_omissions_preserve_eight_argument_function_and_procedure_calls() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-local-omitted-arguments.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-omitted-arguments.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-omitted-arguments.platform.txt"
        ),
    );
}

#[test]
fn local_omissions_reject_invalid_counts_and_reserved_operands() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-local-omitted-arguments.image"
    );
    for count in [-1, 9] {
        let mut image = parse(bytes).unwrap();
        image.instructions[9].operand = count;
        assert!(super::recover_module(&image).is_err());
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[1].operand = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_boolean_defaults_preserve_value_type_and_explicit_undefined() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-boolean-defaults.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-boolean-defaults.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-boolean-defaults.platform.txt"
        ),
    );
}

#[test]
fn boolean_defaults_reject_invalid_values_and_do_not_enable_boolean_constants() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-boolean-defaults.image");
    for invalid in [
        Value::List(vec![Value::String("B".into()), Value::Atom("2".into())]),
        Value::List(vec![Value::String("B".into()), Value::String("1".into())]),
        Value::List(vec![Value::String("B".into())]),
    ] {
        let mut image = parse(bytes).unwrap();
        image.procedures[0].defaults.as_mut().unwrap()[0] = invalid;
        assert!(super::recover_module(&image).is_err());
    }
    let mut image = parse(bytes).unwrap();
    image.constants.push(Value::List(vec![
        Value::String("B".into()),
        Value::Atom("1".into()),
    ]));
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_division_and_remainder_preserve_signs_fractional_values_and_zero_errors() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-division.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-division.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-division.platform.txt"
        ),
    );
}

#[test]
fn division_and_remainder_reject_nonzero_operands_and_missing_arguments() {
    let bytes = include_bytes!("../../../tests/conformance/measure/1c/native-image-division.image");
    for index in [3, 12] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
        let mut image = parse(bytes).unwrap();
        image.instructions[index - 1] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_omitted_arguments_select_defaults_without_mutating_the_callers_slots() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-omitted-arguments.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-omitted-arguments.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-omitted-arguments.platform.txt"
        ),
    );
}

#[test]
fn omitted_arguments_reject_reserved_operands_and_use_as_a_value() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-omitted-arguments.image"
    );
    let mut image = parse(bytes).unwrap();
    image.instructions[2].operand = 1;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[7] = Instruction {
        code: 10,
        operand: 0,
    };
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[6].code = 20;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_min_max_preserve_argument_order_types_and_reference_values() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-min-max.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-min-max.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-min-max.platform.txt"
        ),
    );
}

#[test]
fn min_max_reject_invalid_counts_operands_and_missing_arguments() {
    let bytes = include_bytes!("../../../tests/conformance/measure/1c/native-image-min-max.image");
    for count in [-1, 0, 2, 7] {
        let mut image = parse(bytes).unwrap();
        image.instructions[2].operand = count;
        assert!(super::recover_module(&image).is_err(), "count={count}");
    }
    for index in [2, 3] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[3].operand = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_week_boundaries_require_a_measured_calendar_context() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-short-date-boundaries.image"
    );
    let error = super::decompile(bytes).unwrap_err();
    assert!(error.contains("Cmd[2]"), "{error}");
}

#[test]
fn native_day_time_boundaries_preserve_day_hour_and_minute_limits() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-day-time-boundaries.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-day-time-boundaries.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-day-time-boundaries.platform.txt"
        ),
    );
}

#[test]
fn day_time_boundaries_reject_nonzero_operands_and_missing_arguments() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-day-time-boundaries.image"
    );
    for index in [2, 10, 18, 26, 34, 42] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
        let mut image = parse(bytes).unwrap();
        image.instructions[index - 1] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_type_functions_preserve_primitive_types_aliases_and_unknown_name_errors() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-type-functions.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-type-functions.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-type-functions.platform.txt"
        ),
    );
}

#[test]
fn type_functions_reject_nonzero_operands_and_missing_arguments() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-type-functions.image");
    for index in [2, 10, 18, 20] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
        let mut image = parse(bytes).unwrap();
        image.instructions[index - 1] = Instruction {
            code: 1,
            operand: 1,
        };
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_procedure_early_return_preserves_reference_updates_and_skips_the_tail() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-procedure-early-return.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-early-return.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-early-return.platform.txt"
        ),
    );
}

#[test]
fn procedure_early_return_rejects_wrong_targets_scope_and_a_nonempty_stack() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-early-return.image"
    );
    for (index, operand) in [(9, 15), (8, 1), (16, 0)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[9].operand = 17;
    // `procedure-exit-no-line` измеряет переход прямо к завершающей
    // команде процедуры, а не только к предшествующей строке.
    assert!(super::recover_module(&image).is_ok());
    let mut image = parse(bytes).unwrap();
    image.instructions[7] = Instruction {
        code: 3,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.instructions[10] = Instruction {
        code: 128,
        operand: 0,
    };
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.procedures[0].flags = 17;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn procedure_early_return_cannot_jump_to_another_procedures_body() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-procedure-early-return.image"
    );
    let mut image = parse(bytes).unwrap();
    image.procedures.push(super::Procedure {
        name: "Вторая".into(),
        flags: 16,
        parameter_count: 0,
        entry: 18,
        variables: Some(vec![]),
        defaults: Some(vec![]),
        labels: vec![],
    });
    image.instructions.extend([
        Instruction {
            code: 1,
            operand: 1,
        },
        Instruction {
            code: 22,
            operand: 0,
        },
    ]);
    image.instructions[9].operand = 18;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_format_instruction_preserves_date_number_boolean_string_and_undefined_output() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-format.image"
    ))
    .unwrap();
    let driver =
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-format.bsl");
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-format.platform.txt"
    );
    let actual = run_recovered_source(source, driver);
    // Случай одного размера группировки расходится уже в существующем
    // форматтере; полная нативная проба сохранена, отдельная проверка ниже
    // сравнивает его поведение с обычным исходным BSL.
    let without_single_group = |text: &str| {
        text.lines()
            .filter(|line| !line.starts_with("image.native.format.grouped"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        without_single_group(&actual),
        without_single_group(expected)
    );
}

#[test]
fn recovered_format_preserves_the_existing_formatter_for_single_group_settings() {
    let driver =
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-format.bsl");
    let recovered = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-format.image"
    ))
    .unwrap();
    let ordinary = "Функция Проба(ЗначениеПробы, Знач ФорматПробы) Экспорт\n\
        Возврат Формат(ЗначениеПробы, ФорматПробы);\nКонецФункции\n";
    assert_eq!(
        run_recovered_source(recovered, driver),
        run_recovered_source(ordinary.into(), driver)
    );
}

#[test]
fn format_instruction_rejects_reserved_operands_and_missing_arguments() {
    let bytes = include_bytes!("../../../tests/conformance/measure/1c/native-image-format.image");
    let mut image = parse(bytes).unwrap();
    image.instructions[3].operand = 1;
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("Cmd[3]")
    );
    let mut image = parse(bytes).unwrap();
    image.instructions[2] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("Cmd[3]")
    );
}

#[test]
fn native_while_rechecks_the_context_call_and_preserves_inner_branch_updates() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-while-context-branch.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-while-context-branch.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-while-context-branch.platform.txt"
        ),
    );
}

#[test]
fn while_context_branch_rejects_invalid_exit_inner_branch_and_back_edge() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-while-context-branch.image"
    );
    for (index, target) in [(7, 38), (14, 40), (38, 2)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = target;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_context_functions_preserve_zero_to_six_arguments_and_shared_references() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-context-function-arguments.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-function-arguments.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-function-arguments.platform.txt"
        ),
    );
}

#[test]
fn context_functions_reject_invalid_counts_and_missing_arguments() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-context-function-arguments.image"
    );
    let image = parse(bytes).unwrap();
    for (index, instruction) in image.instructions.iter().enumerate() {
        if instruction.code == 18 {
            for count in [-1, 7, instruction.operand + 1] {
                let mut malformed = parse(bytes).unwrap();
                malformed.instructions[index].operand = count;
                assert!(super::recover_module(&malformed).is_err(), "Cmd[{index}]");
            }
        }
    }
}

#[test]
fn native_constructors_preserve_zero_to_six_arguments_and_independent_arrays() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-constructor-arguments.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-constructor-arguments.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-constructor-arguments.platform.txt"
        ),
    );
}

#[test]
fn constructors_reject_invalid_counts_and_missing_arguments() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-constructor-arguments.image"
    );
    let image = parse(bytes).unwrap();
    for (index, instruction) in image.instructions.iter().enumerate() {
        if instruction.code == 18 {
            for count in [-1, 7, instruction.operand + 1] {
                let mut malformed = parse(bytes).unwrap();
                malformed.instructions[index].operand = count;
                assert!(super::recover_module(&malformed).is_err(), "Cmd[{index}]");
            }
        }
    }
}

#[test]
fn native_local_calls_preserve_multiple_arguments_defaults_and_reference_updates() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-local-arguments.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-arguments.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-arguments.platform.txt"
        ),
    );
}

#[test]
fn local_calls_reject_missing_required_arguments_and_invalid_counts() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-local-arguments.image");
    let image = parse(bytes).unwrap();
    for (index, instruction) in image.instructions.iter().enumerate() {
        if instruction.code == 18 {
            for count in [-1, 7] {
                let mut malformed = parse(bytes).unwrap();
                malformed.instructions[index].operand = count;
                assert!(super::recover_module(&malformed).is_err(), "Cmd[{index}]");
            }
            if instruction.operand == 6 {
                let mut malformed = parse(bytes).unwrap();
                malformed.instructions[index].operand = 5;
                assert!(super::recover_module(&malformed).is_err(), "Cmd[{index}]");
            }
        }
    }
    let mut malformed = parse(bytes).unwrap();
    malformed.procedures[3].defaults.as_mut().unwrap()[5] =
        Value::List(vec![Value::String(String::new())]);
    assert!(super::recover_module(&malformed).is_err());
}

#[test]
fn native_context_procedures_preserve_three_five_and_six_reference_arguments() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-context-procedures.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-procedures.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-procedures.platform.txt"
        ),
    );
}

#[test]
fn context_procedures_reject_invalid_counts_and_missing_stack_arguments() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-context-procedures.image"
    );
    let image = parse(bytes).unwrap();
    for (index, instruction) in image.instructions.iter().enumerate() {
        if instruction.code == 18 {
            for count in [-1, 7, instruction.operand + 1] {
                let mut malformed = parse(bytes).unwrap();
                malformed.instructions[index].operand = count;
                assert!(super::recover_module(&malformed).is_err(), "Cmd[{index}]");
            }
        }
    }
}

#[test]
fn native_date_boundaries_match_leap_day_year_end_and_quarter_start() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-date-boundaries.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-date-boundaries.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-date-boundaries.platform.txt"
        ),
    );
}

#[test]
fn date_boundaries_reject_nonzero_reserved_operands() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-date-boundaries.image");
    for index in (2..48).step_by(8) {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_foreach_collection_call_preserves_iteration_and_the_original_table() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-call.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-foreach-call.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-call.platform.txt"
        ),
    );
}

#[test]
fn native_context_function_with_one_argument_preserves_lookup_results() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-context-one.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-context-one.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-one.platform.txt"
        ),
    );
}

#[test]
fn native_string_replacement_preserves_argument_order_and_the_source_parameter() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-string-replace.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-string-replace.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-string-replace.platform.txt"
        ),
    );
}

#[test]
fn string_replacement_rejects_reserved_operands_and_missing_arguments() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-string-replace.image");
    let mut image = parse(bytes).unwrap();
    image.instructions[4].operand = 1;
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("Cmd[4]")
    );
    let mut image = parse(bytes).unwrap();
    image.instructions[1] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("Cmd[4]")
    );
}

#[test]
fn native_foreach_branch_preserves_the_sum_and_the_callers_collection() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-branch.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-branch.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-foreach-branch.platform.txt"
        ),
    );
}

#[test]
fn invalid_foreach_branch_targets_do_not_produce_partial_source() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-foreach-branch.image");
    for (index, target) in [(23, 99), (29, 99), (12, 36)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = target;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_exported_procedure_preserves_loop_and_branch_parameter_updates() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-procedure-loop.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-loop.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-procedure-loop.platform.txt"
        ),
    );
}

#[test]
fn native_zero_argument_context_procedure_preserves_the_callers_mutated_object() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-context-zero.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-context-zero.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-zero.platform.txt"
        ),
    );
}

fn assert_native_driver(image: &[u8], driver: &str, expected: &str) {
    let source = super::decompile(image).unwrap();
    assert_recovered_source(source, driver, expected);
}

fn assert_recovered_source(source: String, driver: &str, expected: &str) {
    let actual = run_recovered_source(source, driver);
    assert_eq!(actual.trim(), expected.trim());
}

fn run_recovered_source(source: String, driver: &str) -> String {
    let engine = open_bsl::Engine::builder()
        .configuration(open_bsl::ModuleGraphRecipe {
            modules: vec![open_bsl::ModuleRecipe {
                name: "ЗамерыТранзитивногоРодителя".into(),
                source,
                imports: vec![],
                visibility: open_bsl::ModuleVisibility::Named,
            }],
            eager_init: false,
        })
        .build()
        .unwrap();
    let output = Output(std::rc::Rc::new(std::cell::RefCell::new(Vec::new())));
    let mut state = engine.state_builder().stdout(output.clone()).build();
    let module = engine.compile_entry(driver).unwrap();
    state.run(&module).unwrap();
    String::from_utf8(output.0.borrow().clone()).unwrap()
}

#[test]
fn recovered_property_reads_match_the_native_values_without_mutation() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-property.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-property.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-property.platform.txt"
        ),
    );
}

#[test]
fn recovered_property_assignment_updates_the_callers_object() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-property-write.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-property-write.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-property-write.platform.txt"
        ),
    );
}

#[test]
fn recovered_context_procedure_mutates_before_the_following_function_call() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-method-write.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-method-write.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-method-write.platform.txt"
        ),
    );
}

#[test]
fn recovered_calls_capture_each_result_before_later_mutations() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-call-order.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-call-order.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-call-order.platform.txt"
        ),
    );
}

#[test]
fn generated_call_temporaries_do_not_shadow_the_source_parameter() {
    let source =
        include_str!("../../../tests/conformance/measure/1c/native-image-call-order.image");
    let changed = source.replacen("\"Аргумент\",5", "\"__OPEN_BSL_IMAGE_RESULT_3\",5", 1);
    assert_ne!(source, changed);
    assert_native_driver(
        changed.as_bytes(),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-call-order.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-call-order.platform.txt"
        ),
    );
}

#[test]
fn context_procedure_recovery_rejects_bad_arity_and_method_names() {
    let source =
        include_str!("../../../tests/conformance/measure/1c/native-image-method-write.image");
    for (from, to) in [
        ("{18,1}", "{18,2}"),
        ("{20,0}", "{20,9}"),
        ("{\"S\",\"Добавить\"}", "{\"N\",0}"),
    ] {
        let changed = source.replacen(from, to, 1);
        assert_ne!(source, changed);
        assert!(super::decompile(changed.as_bytes()).is_err());
    }
}

#[test]
fn property_recovery_rejects_invalid_name_references_and_nonstring_names() {
    let source = include_str!("../../../tests/conformance/measure/1c/native-image-property.image");
    for (from, to) in [
        ("{12,0}", "{12,9}"),
        ("{\"S\",\"ЗначениеПробы\"}", "{\"N\",0}"),
        ("ЗначениеПробы", "ЗначениеПробы;"),
    ] {
        let changed = source.replacen(from, to, 1);
        assert_ne!(source, changed);
        assert!(super::decompile(changed.as_bytes()).is_err());
    }
}

#[test]
fn recovered_context_method_returns_the_native_result_without_mutation() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-method.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-method.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-method.platform.txt"
        ),
    );
}

#[test]
fn context_call_rejects_invalid_name_indices_and_unpaired_return_reads() {
    let source = include_str!("../../../tests/conformance/measure/1c/native-image-method.image");
    for (from, to) in [
        ("{21,0}", "{21,9}"),
        ("{5,0}", "{5,1}"),
        ("{\"S\",\"Количество\"}", "{\"N\",0}"),
        ("Количество", "Количество();"),
    ] {
        let changed = source.replacen(from, to, 1);
        assert_ne!(source, changed);
        assert!(super::decompile(changed.as_bytes()).is_err());
    }
}

#[test]
fn recovered_string_preserves_quotes_empty_lines_and_measured_line_endings() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-string.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-string.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-string.platform.txt"
        ),
    );
}

#[test]
fn recovered_local_call_preserves_references_and_private_visibility() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-local-call.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-local-call.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-call.platform.txt"
        ),
    );
}

#[test]
fn local_call_recovery_rejects_invalid_indices_arity_and_partial_callees() {
    let source =
        include_str!("../../../tests/conformance/measure/1c/native-image-local-call.image");
    for (from, to) in [
        ("{19,1}", "{19,9}"),
        ("{18,1}", "{18,2}"),
        ("{24,0}", "{128,0}"),
    ] {
        let changed = source.replacen(from, to, 1);
        assert_ne!(source, changed);
        assert!(super::recover_module(&parse(changed.as_bytes()).unwrap()).is_err());
    }
}

#[test]
fn an_unclosed_image_root_is_rejected() {
    let truncated = NATIVE_CONSTANT_IMAGE.strip_suffix('}').unwrap();
    assert!(parse(truncated.as_bytes()).is_err());
}

#[test]
fn recovered_while_matches_multiple_iterations_and_no_entry_cases() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-while.image"
    ))
    .unwrap();
    let mut source = super::recover_module(&image).unwrap();
    source.push_str("\nВыводПробы = \"\";\n");
    for (label, value) in [("positive", 3), ("zero", 0), ("negative", -2)] {
        source.push_str(&format!(
            "АргументПробы = {value}; РезультатПробы = Проба(АргументПробы);\n\
             ВыводПробы = ВыводПробы + \"image.native.while.{label}.result=\" + Строка(РезультатПробы)\n\
             + Символы.ПС + \"image.native.while.{label}.argument=\" + Строка(АргументПробы) + Символы.ПС;\n"
        ));
    }
    source.push_str("Возврат ВыводПробы;");
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let value = engine.new_state().run(&module).unwrap();
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-while.platform.txt"
    );
    assert_eq!(
        open_bsl::format_value(&value, None).unwrap().trim(),
        expected.trim()
    );
}

#[test]
fn while_recovery_rejects_a_back_edge_that_skips_the_condition() {
    let source = include_str!("../../../tests/conformance/measure/1c/native-image-while.image");
    let changed = source.replacen("{39,0}", "{39,5}", 1);
    assert_ne!(source, changed);
    assert!(super::recover_module(&parse(changed.as_bytes()).unwrap()).is_err());
}

#[test]
fn recovered_branches_match_both_native_paths_and_reference_updates() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-branch.image"
    ))
    .unwrap();
    let source = super::recover_module(&image).unwrap();
    let driver = r#"
АргументПробы = Истина;
РезультатПробы = Проба(АргументПробы);
ВыводПробы = "image.native.branch.true.result=" + Строка(РезультатПробы)
    + Символы.ПС + "image.native.branch.true.argument=" + Строка(АргументПробы);
АргументПробы = Ложь;
РезультатПробы = Проба(АргументПробы);
Возврат ВыводПробы + Символы.ПС
    + "image.native.branch.false.result=" + Строка(РезультатПробы)
    + Символы.ПС + "image.native.branch.false.argument=" + Строка(АргументПробы);
"#;
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&format!("{source}\n{driver}")).unwrap();
    let value = engine.new_state().run(&module).unwrap();
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-branch.platform.txt"
    );
    assert_eq!(
        open_bsl::format_value(&value, None).unwrap(),
        expected.trim()
    );
}

#[test]
fn structured_branches_rejoin_before_the_shared_return() {
    let bytes = include_bytes!("../../../tests/conformance/measure/1c/native-image-branch.image");
    let image = parse(bytes).unwrap();
    assert_recovered_source(
        super::recover_linear_module(&image).unwrap(),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-branch.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-branch.platform.txt"
        ),
    );
    for (index, operand) in [(2, 9), (7, 99), (7, 2)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(
            super::recover_linear_module(&image).is_err(),
            "Cmd[{index}]"
        );
    }
}

#[test]
fn a_measured_back_edge_is_not_misclassified_as_an_invalid_forward_join() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-branch-back-edge.image"
    ))
    .unwrap();
    assert_recovered_source(
        super::recover_linear_module(&image).unwrap(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-branch-back-edge.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-branch-back-edge.platform.txt"
        ),
    );
}

#[test]
fn native_logical_negation_reverses_the_selected_branch_and_reference_update() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-not.image"
    ))
    .unwrap();
    assert_recovered_source(
        super::recover_module(&image).unwrap(),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-not.bsl"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-not.platform.txt"),
    );
}

#[test]
fn native_local_call_preserves_an_omitted_undefined_default_and_an_explicit_argument() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-default-undefined.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-default-undefined.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-default-undefined.platform.txt"
        ),
    );
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-default-undefined.image"
    ))
    .unwrap();
    image.procedures[1].defaults = Some(vec![Value::List(vec![Value::String(String::new())])]);
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("Cmd[2]")
    );
}

#[test]
fn native_index_reference_preserves_read_write_and_out_of_bounds_failure() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-index.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-index.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-index.platform.txt"
        ),
    );
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-index.image"
    ))
    .unwrap();
    image.instructions[3].operand = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn native_comparison_opcodes_preserve_all_three_sides_of_the_boundary() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-comparisons.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-comparisons.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-comparisons.platform.txt"
        ),
    );
    for index in [3, 12, 21] {
        let mut image = parse(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-comparisons.image"
        ))
        .unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn native_context_calls_preserve_two_arguments_and_the_reference_output() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-context-arguments.image"
        ),
        &include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-arguments.bsl"
        ).split("    ТаблицаПробы =").next().unwrap().replace("Попытка\n", ""),
        &include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-context-arguments.platform.txt"
        ).lines().filter(|line| !line.starts_with("image.native.context.four.")).collect::<Vec<_>>().join("\n"),
    );
    for (index, count) in [(4, -1), (4, 4), (18, -1), (18, 4)] {
        let mut image = parse(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-context-arguments.image"
        ))
        .unwrap();
        image.instructions[index].operand = count;
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn reconstructed_four_argument_call_preserves_the_existing_runtime_limitation() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-context-arguments.image"
    ))
    .unwrap();
    assert!(
        source
            .contains("Аргумент.Добавить((\"КодПробы\"), Неопределено, (\"ЗаголовокПробы\"), 20);")
    );
    // Нативный результат сохранен целиком. Но `Колонки.Добавить` с четырьмя
    // аргументами пока отсутствует в базовом runtime: проверяем одинаковый
    // отказ обычного исходника и восстановленного, не успешную подмену.
    let baseline = "Функция ДобавитьКолонку(Аргумент) Экспорт\nАргумент.Добавить(\"КодПробы\", Неопределено, \"ЗаголовокПробы\", 20);\nВозврат Аргумент[0].Имя;\nКонецФункции";
    let engine = open_bsl::Engine::builder().build().unwrap();
    let mut failures = Vec::new();
    for module_source in [source.as_str(), baseline] {
        let module = engine.compile(&format!("{module_source}\nТаблицаПробы = Новый ТаблицаЗначений;\nДобавитьКолонку(ТаблицаПробы.Колонки);")).unwrap();
        failures.push(format!(
            "{:?}",
            engine.new_state().run(&module).unwrap_err()
        ));
    }
    assert_eq!(failures[0], failures[1]);
    assert!(failures[0].contains("MethodNotApplicable"));
    assert!(failures[0].contains("КоллекцияКолонокТаблицыЗначений"));
}

#[test]
fn native_external_method_entries_call_the_existing_globals_without_emitting_bodies() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-external-methods.image");
    assert_native_driver(
        bytes,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-methods.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-external-methods.platform.txt"
        ),
    );
    let source = super::decompile(bytes).unwrap();
    assert!(!source.contains("Процедура ЗаполнитьЗначенияСвойств"));
    assert!(!source.contains("Функция НСтр"));
    for index in [1, 2] {
        let mut image = parse(bytes).unwrap();
        image.procedures[index].entry = 1;
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn native_branch_with_a_loop_preserves_iteration_skip_and_reference_updates() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-branch-loop.image");
    assert_native_driver(
        bytes,
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-branch-loop.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-branch-loop.platform.txt"
        ),
    );
    for (index, target) in [(2, 18), (7, 16), (14, 7), (16, 99)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = target;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn native_date_construction_preserves_one_three_and_six_argument_forms() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-date-construction.image"
    );
    assert_native_driver(
        bytes,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-date-construction.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-date-construction.platform.txt"
        ),
    );
    for index in [2, 12, 25] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn native_counted_loop_with_an_inner_branch_preserves_sum_and_iteration_count() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-loop-branch.image");
    assert_native_driver(
        bytes,
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-loop-branch.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-loop-branch.platform.txt"
        ),
    );
    for (index, target) in [(20, 29), (27, 43), (43, 15)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = target;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn branch_recovery_rejects_targets_inside_an_expression_or_outside_the_body() {
    let source = include_str!("../../../tests/conformance/measure/1c/native-image-branch.image");
    for (from, to) in [("{40,8}", "{40,9}"), ("{39,11}", "{39,99}")] {
        let changed = source.replacen(from, to, 1);
        assert_ne!(source, changed);
        let image = parse(changed.as_bytes()).unwrap();
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn a_recovered_buh_date_function_matches_the_native_receipt() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-year.image"
    ))
    .unwrap();
    let source = super::recover_module(&image).unwrap();
    let driver = "Возврат \"image.native.year.january=\" + Формат(ГодГодовыхПремий('20260101'), \"ЧГ=0\")\n\
        + Символы.ПС + \"image.native.year.leap=\" + Формат(ГодГодовыхПремий('20240229'), \"ЧГ=0\");";
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&format!("{source}\n{driver}")).unwrap();
    let value = engine.new_state().run(&module).unwrap();
    let receipt = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-year.platform.txt"
    );
    assert_eq!(
        open_bsl::format_value(&value, None).unwrap(),
        receipt.trim()
    );
}

#[test]
fn recovered_parameter_functions_match_both_native_receipts() {
    for (image, receipt) in [
        (
            include_str!("../../../tests/conformance/measure/1c/native-image-add.image"),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-add.platform.txt"
            ),
        ),
        (
            include_str!("../../../tests/conformance/measure/1c/native-image-reference.image"),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-reference.platform.txt"
            ),
        ),
        (
            include_str!("../../../tests/conformance/measure/1c/native-image-value.image"),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-value.platform.txt"
            ),
        ),
    ] {
        let image = parse(image.as_bytes()).unwrap();
        let mut source = super::recover_module(&image).unwrap();
        source.push_str(
            "АргументПробы = 5; РезультатПробы = Проба(АргументПробы);\n\
             Возврат \"image.native.reference.enter=1\" + Символы.ПС\n\
             + \"image.native.reference.result=\" + Строка(РезультатПробы) + Символы.ПС\n\
             + \"image.native.reference.argument=\" + Строка(АргументПробы);",
        );
        let engine = open_bsl::Engine::builder().build().unwrap();
        let module = engine.compile(&source).unwrap();
        let value = engine.new_state().run(&module).unwrap();
        assert_eq!(
            open_bsl::format_value(&value, None).unwrap(),
            receipt.trim()
        );
    }
}

#[test]
fn native_reference_image_keeps_the_measured_parameter_header() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-reference.image");
    let image = parse(bytes).unwrap();
    let procedure = &image.procedures[0];
    assert_eq!(procedure.parameter_count, 1);
    assert_eq!(procedure.variables.as_ref().unwrap()[0].flags, 5);
    assert_eq!(
        procedure.defaults.as_ref().unwrap(),
        &[Value::List(vec![Value::String(String::new())])]
    );
}

#[test]
fn native_value_image_differs_only_in_the_parameter_flag() {
    let reference =
        include_str!("../../../tests/conformance/measure/1c/native-image-reference.image");
    let value = include_str!("../../../tests/conformance/measure/1c/native-image-value.image");
    assert_eq!(reference.replacen(",5,-1}", ",13,-1}", 1), value);
    let image = parse(value.as_bytes()).unwrap();
    assert_eq!(image.procedures[0].variables.as_ref().unwrap()[0].flags, 13);
}

#[test]
fn parameter_recovery_rejects_unmeasured_flags_and_changed_bodies() {
    let source = include_str!("../../../tests/conformance/measure/1c/native-image-reference.image");
    for (from, to) in [
        (",5,-1}", ",9,-1}"),
        ("{3,0}", "{3,1}"),
        ("{16,0}", "{17,0}"),
        ("{\"DefPrm\",1,{\"\"}}", "{\"DefPrm\",1,{\"U\"}}"),
    ] {
        let changed = source.replacen(from, to, 1);
        assert_ne!(source, changed);
        let image = parse(changed.as_bytes()).unwrap();
        assert!(super::recover_module(&image).is_err());
    }
}

#[test]
fn date_constant_recovery_rejects_invalid_calendar_values_and_literals() {
    let image = include_str!("../../../tests/conformance/measure/1c/native-image-date.image");
    for date in ["20140230000000", "2014040200000", "20140402'00000"] {
        let changed = image.replacen("20140402000000", date, 1);
        assert_ne!(changed, image);
        let error = super::decompile(changed.as_bytes()).unwrap_err();
        assert!(error.contains("image/Const[0]"), "{error}");
    }
}

#[test]
fn linear_recovery_rejects_invalid_parameter_slots_and_reserved_operands() {
    let image = include_str!(
        "../../../tests/conformance/measure/1c/native-image-compiled-arithmetic.image"
    );
    for (from, to, at) in [
        ("{3,1}", "{3,99}", "image/Cmd[1]"),
        ("{26,0}", "{26,1}", "image/Cmd[9]"),
        (
            "\"Арифметика\",17,2,0",
            "\"Арифметика\",17,3,0",
            "image/Proc[0]",
        ),
    ] {
        let changed = image.replacen(from, to, 1);
        assert_ne!(changed, image);
        let error = super::decompile(changed.as_bytes()).unwrap_err();
        assert!(error.contains(at), "{error}");
    }
}

#[test]
fn linear_recovery_does_not_discard_commands_after_an_early_return() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled-arithmetic.image"
    ))
    .unwrap();
    image.instructions.insert(
        13,
        Instruction {
            code: 99,
            operand: 0,
        },
    );
    image.instructions[15].operand = 16;
    let error = super::recover_module(&image).unwrap_err();
    assert!(error.contains("image/Proc[0]"), "{error}");
}

#[test]
fn the_compiler_arithmetic_body_preserves_both_parameters_and_the_default() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-compiled-arithmetic.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled-arithmetic.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled-arithmetic.platform.txt"
        ),
    );
}

#[test]
fn the_native_compiler_control_image_preserves_declarations_and_omitted_globals() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    assert_eq!(image.entry, u32::MAX);
    assert_eq!(image.instructions.len(), 232);
    assert_eq!(image.constants.len(), 12);
    assert!(image.variables.is_empty());
    assert_eq!(image.procedures.len(), 7);
    assert_eq!(image.procedures[0].name, "Арифметика");
    assert_eq!(image.procedures[0].flags, 17);
    assert_eq!(image.procedures[0].parameter_count, 2);
    let parameters = image.procedures[0].variables.as_ref().unwrap();
    assert_eq!(parameters[0].flags, 13);
    assert_eq!(parameters[1].flags, 5);
    assert_eq!(
        image.procedures[0].defaults.as_ref().unwrap()[1],
        Value::List(vec![Value::String("N".into()), Value::Atom("3".into())])
    );
    assert_eq!(image.procedures[1].flags, 16);
    assert_eq!(image.procedures[5].name, "СчитатьВызов");
    assert_eq!(image.procedures[5].flags, 1);
    assert_eq!(
        image.procedures[3].variables.as_ref().unwrap()[4].name,
        "0ЭлементПробы"
    );
    assert_eq!(image.procedures[6].variables, None);
}

#[test]
fn compiler_branches_preserve_all_three_native_return_paths() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    // Первые три метода сохранены без изменения команд и объявлений.
    // Ожидания взяты из полного нативного исполнения этого же образа.
    image.procedures.truncate(3);
    image.instructions.truncate(58);
    let source = super::recover_module(&image).unwrap();
    assert_recovered_source(
        source,
        "Сообщить(\"image.branch.negative=\" + Строка(ЗамерыТранзитивногоРодителя.Ветвление(-2)));\n\
         Сообщить(\"image.branch.zero=\" + Строка(ЗамерыТранзитивногоРодителя.Ветвление(0)));\n\
         Сообщить(\"image.branch.positive=\" + Строка(ЗамерыТранзитивногоРодителя.Ветвление(2)));",
        &include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
        )
        .lines()
        .filter(|line| line.starts_with("image.branch."))
        .collect::<Vec<_>>()
        .join("\n"),
    );
    image.instructions[29].operand = 38;
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("Cmd[29]")
    );
}

#[test]
fn compiler_loops_preserve_the_native_sum() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    // Все команды первых четырех методов остаются в исходных позициях.
    image.procedures.truncate(4);
    image.instructions.truncate(158);
    let source = super::recover_module(&image).unwrap();
    assert_recovered_source(
        source,
        "Сообщить(\"image.loops=\" + Строка(ЗамерыТранзитивногоРодителя.Циклы()));",
        &include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
        )
        .lines()
        .filter(|line| line.starts_with("image.loops="))
        .collect::<Vec<_>>()
        .join("\n"),
    );
}

#[test]
fn compiler_loops_reject_invalid_targets_and_observable_iterator_slots() {
    let original = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    for (index, operand) in [(74, 84), (99, 88), (143, 124), (147, 1), (139, 4)] {
        let mut image = parse(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-compiled.image"
        ))
        .unwrap();
        image.procedures.truncate(4);
        image.instructions.truncate(158);
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    let mut image = original;
    image.procedures.truncate(4);
    image.instructions.truncate(158);
    image.procedures[3].variables.as_mut().unwrap()[4].name = "ЧужаяПеременная".into();
    // Нативный `foreach-renamed-iterator` измеряет допустимость другого имени
    // скрытого слота при сохранении его флага и всех ссылок по индексу.
    assert!(super::recover_module(&image).is_ok());
}

#[test]
fn compiler_short_circuit_calls_only_the_selected_operand() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    image.procedures.truncate(6);
    image.instructions.truncate(215);
    let source = super::recover_module(&image).unwrap();
    assert_recovered_source(
        source,
        "Сообщить(\"image.short_circuit=\" + Строка(ЗамерыТранзитивногоРодителя.КороткоеВычисление()));",
        &include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
        )
        .lines()
        .filter(|line| line.starts_with("image.short_circuit="))
        .collect::<Vec<_>>()
        .join("\n"),
    );
    image.instructions[165].operand = 171;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn the_complete_compiler_control_image_matches_all_native_receipt_lines() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-compiled.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-compiled.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
        ),
    );
}

#[test]
fn method_table_order_does_not_define_body_boundaries() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    // Первые два метода вызываются по имени из драйвера; внутренний
    // индекс закрытой функции остается равен пяти, как в нативном образе.
    image.procedures.swap(0, 1);
    assert_recovered_source(
        super::recover_module(&image).unwrap(),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-compiled.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
        ),
    );
    image.procedures[0].entry = 0;
    assert!(
        super::recover_module(&image)
            .unwrap_err()
            .contains("повторный адрес")
    );
}

#[test]
fn complete_compiler_recovery_rejects_a_corrupt_final_handler_instead_of_returning_earlier_methods()
{
    for (index, operand) in [(215, 223), (220, 0), (221, 228), (226, 230), (228, 1)] {
        let mut image = parse(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-compiled.image"
        ))
        .unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn recovered_buh_date_constant_matches_the_native_value_and_type() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-date.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-date.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-date.platform.txt"
        ),
    );
}

#[test]
fn recovered_exported_procedure_is_available_for_local_and_external_calls() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-export-procedure.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-export-procedure.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-export-procedure.platform.txt"
        ),
    );
}

#[test]
fn local_function_recovery_rejects_invalid_targets_and_parameter_layout() {
    let image =
        include_str!("../../../tests/conformance/measure/1c/native-image-local-function.image");
    for (from, to, at) in [
        ("{19,1}", "{19,9}", "image/Cmd[3]"),
        (
            "\"Изменить\",1,1,10",
            "\"Изменить\",1,1,11",
            "image/Proc[1]",
        ),
    ] {
        let changed = image.replacen(from, to, 1);
        assert_ne!(changed, image);
        let error = super::decompile(changed.as_bytes()).unwrap_err();
        assert!(error.contains(at), "{error}");
    }
}

#[test]
fn recovered_local_function_preserves_its_result_reference_and_visibility() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-local-function.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-function.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-local-function.platform.txt"
        ),
    );
}

#[test]
fn global_recovery_rejects_unmeasured_flags_and_external_slots() {
    let image = include_str!("../../../tests/conformance/measure/1c/native-image-global.image");
    for (from, to) in [
        ("\"Символы\",2,-1", "\"Символы\",0,-1"),
        ("\"Символы\",2,-1", "\"Символы\",2,0"),
        ("{2,0}", "{2,9}"),
        ("\"Символы\",2,-1", "\"Символы();\",2,-1"),
    ] {
        let changed = image.replacen(from, to, 1);
        assert_ne!(changed, image);
        let error = super::decompile(changed.as_bytes()).unwrap_err();
        assert!(error.contains("image/Cmd[1]"), "{error}");
    }
}

#[test]
fn recovered_global_reference_uses_the_existing_bsl_context() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-global.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-global.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-global.platform.txt"
        ),
    );
}

#[test]
fn structured_module_recovery_preserves_the_measured_external_global() {
    let bytes = include_bytes!("../../../tests/conformance/measure/1c/native-image-global.image");
    let image = parse(bytes).unwrap();
    let source = super::recover_linear_module(&image).unwrap();
    assert!(!source.contains("Перем Символы"));
    assert_recovered_source(
        source,
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-global.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-global.platform.txt"
        ),
    );
    for (flags, external_slot) in [(1, -1), (2, 0)] {
        let mut image = parse(bytes).unwrap();
        image.variables[0].flags = flags;
        image.variables[0].external_slot = external_slot;
        assert!(
            super::recover_linear_module(&image)
                .unwrap_err()
                .contains("Var[0]")
        );
    }
    let mut image = parse(bytes).unwrap();
    image.variables.push(Variable {
        name: "СИМВОЛЫ".into(),
        flags: 2,
        external_slot: -1,
    });
    assert!(
        super::recover_linear_module(&image)
            .unwrap_err()
            .contains("Var[1]")
    );
}

#[test]
fn constructor_recovery_rejects_invalid_type_names_and_operands() {
    let image = include_str!("../../../tests/conformance/measure/1c/native-image-new.image");
    for (from, to) in [
        ("{51,0}", "{51,9}"),
        ("{18,0}", "{18,1}"),
        ("\"Массив\"", "\"Массив();\""),
    ] {
        let changed = image.replacen(from, to, 1);
        assert_ne!(changed, image);
        assert!(super::decompile(changed.as_bytes()).is_err());
    }
}

#[test]
fn recovered_constructor_replaces_the_argument_with_a_fresh_object() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-new.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-new.bsl"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-new.platform.txt"),
    );
}

#[test]
fn a_recovered_undefined_function_matches_the_native_receipt() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-undefined.image"),
        include_str!("../../../tests/conformance/measure/1c/measure-native-image-undefined.bsl"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-undefined.platform.txt"
        ),
    );
}

#[test]
fn a_recovered_constant_function_executes_with_the_native_result() {
    let image = parse(NATIVE_CONSTANT_IMAGE.as_bytes()).unwrap();
    let source = super::recover_module(&image).unwrap();
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let (value, parameters) = instance.call(&mut state, "Проба", vec![]).unwrap();
    assert!(parameters.is_empty());
    let actual = format!(
        "image.native.constant={}",
        open_bsl::format_value(&value, None).unwrap()
    );
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-constant.platform.txt"
    );
    assert_eq!(actual, expected.trim());
}

#[test]
fn constant_recovery_rejects_unsupported_code_and_invalid_references() {
    for (from, to, at) in [
        ("{4,0}", "{4,2}", "image/Cmd[1]"),
        ("{17,0}", "{128,0}", "image/Cmd[2]"),
        ("{39,5}", "{39,99}", "image/Cmd[4]"),
        ("\"Проба\",17", "\"Проба\",1", "image/Proc[0]"),
        ("{\"N\",60}", "{\"N\",нетчисла}", "image/Const[0]"),
        ("\"Проба\",17", "\"Проба();\",17", "image/Proc[0]"),
    ] {
        let changed = NATIVE_CONSTANT_IMAGE.replacen(from, to, 1);
        assert_ne!(changed, NATIVE_CONSTANT_IMAGE);
        let image = parse(changed.as_bytes()).unwrap();
        let error = super::recover_module(&image).unwrap_err();
        assert!(error.contains(at), "{error}");
    }
}

const TABLES: &str = r#"{1,
{"Cmd",2,4294967295,{1,7},{4,0}},
{"Const",1,{"S","строка ""в кавычках"""}},
{"Var",1,{"Справочники",2,-1}},
{"Proc",2,
 {"Проба",17,1,0,{"Var",1,{"Аргумент",5,-1}},{"DefPrm",1,{"B",1}}},
 {"ВнешнийВызов",33,2,0}}
}"#;

#[test]
fn raw_tables_keep_flags_slots_operands_and_omitted_local_sections() {
    let image = parse(TABLES.as_bytes()).unwrap();
    assert_eq!(image.entry, u32::MAX);
    assert_eq!(
        image.instructions,
        [
            Instruction {
                code: 1,
                operand: 7
            },
            Instruction {
                code: 4,
                operand: 0
            }
        ]
    );
    assert_eq!(
        image.constants,
        [Value::List(vec![
            Value::String("S".into()),
            Value::String("строка \"в кавычках\"".into())
        ])]
    );
    assert_eq!(
        image.variables,
        [Variable {
            name: "Справочники".into(),
            flags: 2,
            external_slot: -1
        }]
    );
    let procedure = &image.procedures[0];
    assert_eq!(procedure.name, "Проба");
    assert_eq!(procedure.flags, 17);
    assert_eq!(procedure.parameter_count, 1);
    assert_eq!(procedure.entry, 0);
    assert_eq!(procedure.variables.as_ref().unwrap()[0].name, "Аргумент");
    assert_eq!(
        procedure.defaults,
        Some(vec![Value::List(vec![
            Value::String("B".into()),
            Value::Atom("1".into())
        ])])
    );
    let external = &image.procedures[1];
    assert_eq!(external.flags, 33);
    assert_eq!(external.parameter_count, 2);
    assert!(external.variables.is_none());
    assert!(external.defaults.is_none());
}

#[test]
fn table_headers_reject_malformed_tokens_without_hiding_records() {
    for (from, to, at) in [
        ("\"Cmd\",2", "\"Cmd\",-1", "image/Cmd"),
        ("\"Const\",1", "\"Const\",\"1\"", "image/Const"),
        ("\"Var\",1", "\"Var\",4294967296", "image/Var"),
        ("\"Proc\",2", "\"Proc\",", "image/Proc"),
        ("\"DefPrm\",1", "\"DefPrm\",-1", "image/Proc[0]/DefPrm"),
    ] {
        let error = parse(TABLES.replacen(from, to, 1).as_bytes()).unwrap_err();
        assert!(error.contains(at), "{error}");
        assert!(error.contains("целое"), "{error}");
    }
    let smaller = parse(TABLES.replacen("\"Proc\",2", "\"Proc\",1", 1).as_bytes()).unwrap();
    assert_eq!(smaller.procedures.len(), 2);
}

#[test]
fn sections_and_record_shapes_fail_at_their_actual_location() {
    for (from, to, at) in [
        ("{1,\n", "{2,\n", "image"),
        ("\"Const\"", "\"Cmd\"", "image/Cmd"),
        ("\"Const\"", "\"Неизвестная\"", "image/Неизвестная"),
        ("{1,7}", "{1,7,8}", "image/Cmd[0]"),
        ("{1,7}", "{1,\"7\"}", "image/Cmd[0]"),
        (
            "{\"Справочники\",2,-1}",
            "{\"Справочники\",2}",
            "image/Var[0]",
        ),
        ("\"Проба\",17,1,0", "\"Проба\",17,1,\"0\"", "image/Proc[0]"),
    ] {
        let error = parse(TABLES.replacen(from, to, 1).as_bytes()).unwrap_err();
        assert!(error.contains(at), "{error}");
    }
    let error = parse(b"{1, {\"Cmd\",0,4294967295}}").unwrap_err();
    assert!(error.contains("image/Const"), "{error}");
}

#[test]
fn the_opt_in_fourth_buh_image_recovers_a_syntactically_valid_module() {
    let Some(path) = std::env::var_os("OPEN_BSL_BUH_FOURTH_NATIVE_IMAGE") else {
        eprintln!("синтаксис четвертого образа buh не проверен: образ не задан");
        return;
    };
    let image = parse(&std::fs::read(path).unwrap()).unwrap();
    let source = super::recover_module(&image).unwrap();
    eprintln!(
        "четвертый образ buh восстановлен: {} байт BSL",
        source.len()
    );
    let engine = open_bsl::Engine::builder().build().unwrap();
    match engine.compile(&source) {
        Err(open_bsl::Error::Parse(error)) => panic!("синтаксис четвертого образа buh: {error}"),
        Err(error) => eprintln!("синтаксис проверен; связывание без конфигурации: {error}"),
        Ok(_) => eprintln!("синтаксис и связывание проверены; исполнение не запускалось"),
    }
}

#[test]
fn the_opt_in_second_buh_image_recovers_a_syntactically_valid_module() {
    let Some(path) = std::env::var_os("OPEN_BSL_BUH_SECOND_NATIVE_IMAGE") else {
        eprintln!("синтаксис второго образа buh не проверен: образ не задан");
        return;
    };
    let image = parse(&std::fs::read(path).unwrap()).unwrap();
    let source = super::recover_module(&image).unwrap();
    eprintln!("второй образ buh восстановлен: {} байт BSL", source.len());
    let engine = open_bsl::Engine::builder().build().unwrap();
    match engine.compile(&source) {
        Err(open_bsl::Error::Parse(error)) => panic!("синтаксис второго образа buh: {error}"),
        Err(error) => eprintln!("синтаксис проверен; связывание без конфигурации: {error}"),
        Ok(_) => eprintln!("синтаксис и связывание проверены; исполнение не запускалось"),
    }
}

#[test]
fn the_opt_in_buh_image_recovers_a_complete_syntactically_valid_module() {
    let Some(path) = std::env::var_os("OPEN_BSL_BUH_NATIVE_IMAGE") else {
        eprintln!("синтаксис восстановленного buh не проверен: образ не задан");
        return;
    };
    let image = parse(&std::fs::read(path).unwrap()).unwrap();
    let source = super::recover_module(&image).unwrap();
    let engine = open_bsl::Engine::builder().build().unwrap();
    match engine.compile(&source) {
        Err(open_bsl::Error::Parse(error)) => panic!("синтаксис восстановленного buh: {error}"),
        Err(error) => eprintln!("синтаксис buh проверен; связывание без конфигурации: {error}"),
        Ok(_) => eprintln!("синтаксис и связывание buh проверены; исполнение не запускалось"),
    }
}

#[test]
fn the_opt_in_buh_image_preserves_its_native_declaration_tables() {
    let Some(path) = std::env::var_os("OPEN_BSL_BUH_NATIVE_IMAGE") else {
        eprintln!("нативный образ buh не проверен: OPEN_BSL_BUH_NATIVE_IMAGE не задан");
        return;
    };
    let image = parse(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(image.entry, u32::MAX);
    assert_eq!(image.instructions.len(), 11_913);
    assert_eq!(image.constants.len(), 648);
    assert_eq!(image.variables.len(), 38);
    assert_eq!(image.procedures.len(), 119);
    assert!(
        image
            .variables
            .iter()
            .all(|variable| variable.flags == 2 && variable.external_slot == -1)
    );
    match super::recover_module(&image) {
        Ok(source) => {
            eprintln!(
                "нативный образ buh восстановлен: {} байт BSL; исполнение этим тестом не проверено",
                source.len()
            );
        }
        Err(error) => {
            eprintln!("нативный образ buh еще не восстановлен: {error}");
        }
    }
    let procedure = image
        .procedures
        .iter()
        .find(|procedure| procedure.name == "ДатаСобытияСУчетомНерабочихДней")
        .unwrap();
    assert_eq!(procedure.flags, 17);
    assert_eq!(procedure.parameter_count, 3);
    assert_eq!(procedure.entry, 5641);
    assert_eq!(procedure.variables.as_ref().unwrap().len(), 8);
    let defaults = procedure.defaults.as_ref().unwrap();
    assert_eq!(defaults.len(), 3);
    assert_eq!(
        defaults[2],
        Value::List(vec![Value::String("B".into()), Value::Atom("1".into())])
    );
}

#[test]
fn measured_value_parameter_slot_zero_preserves_supplied_and_required_calls() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-parameter-slot-zero/image"),
        include_str!("../../../docs/research/fixtures/native-parameter-slot-zero/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-parameter-slot-zero/platform.txt"),
    );
}

#[test]
fn parameter_slot_zero_keeps_unmeasured_flags_slots_and_missing_defaults_unsupported() {
    let bytes = include_bytes!("../../../docs/research/fixtures/native-parameter-slot-zero/image");
    for (flags, slot) in [(13, 1), (5, 0), (1, 0)] {
        let mut image = parse(bytes).unwrap();
        let variable = &mut image.procedures[0].variables.as_mut().unwrap()[0];
        variable.flags = flags;
        variable.external_slot = slot;
        assert!(super::recover_module(&image).is_err());
    }
    let mut image = parse(bytes).unwrap();
    image.procedures[0].defaults = None;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn overlapping_constant_tail_preserves_ordinary_and_private_async_results() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-overlapping-constant-tail/image"),
        include_str!("../../../docs/research/fixtures/native-overlapping-constant-tail/driver.bsl"),
        include_str!(
            "../../../docs/research/fixtures/native-overlapping-constant-tail/platform.txt"
        ),
    );
}

#[test]
fn overlapping_constant_tail_keeps_private_entries_and_rejects_corrupt_boundaries() {
    let bytes =
        include_bytes!("../../../docs/research/fixtures/native-overlapping-constant-tail/image");
    let mut image = parse(bytes).unwrap();
    assert_eq!(super::method_end(&image, 0), 11);
    let private_body: Vec<_> = image.instructions[6..11]
        .iter()
        .map(|command| (command.code, command.operand))
        .collect();
    super::normalize_measured_jumps(&mut image);
    assert_eq!(super::method_end(&image, 0), 11);
    assert_eq!(
        image.instructions[6..11]
            .iter()
            .map(|command| (command.code, command.operand))
            .collect::<Vec<_>>(),
        private_body
    );
    for (index, operand) in [(0, 99), (7, 1), (8, 1), (9, 9), (10, 1)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        super::normalize_measured_jumps(&mut image);
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
    let mut image = parse(bytes).unwrap();
    let mut other = parse(bytes).unwrap().procedures.remove(0);
    other.name = "OtherEntry".into();
    other.entry = 3;
    image.procedures.push(other);
    super::normalize_measured_jumps(&mut image);
    assert_eq!(image.instructions[3].code, 95);
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn integer_part_preserves_native_values_errors_and_reference_assignment() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-int/image"),
        include_str!("../../../docs/research/fixtures/native-int/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-int/platform.txt"),
    );
}

#[test]
fn integer_part_result_is_fixed_before_a_reference_mutating_call() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-int-snapshot/image"),
        include_str!("../../../docs/research/fixtures/native-int-snapshot/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-int-snapshot/platform.txt"),
    );
}

#[test]
fn integer_part_rejects_unmeasured_operands_and_missing_input() {
    let bytes = include_bytes!("../../../docs/research/fixtures/native-int/image");
    for index in [1, 8] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = 1;
        assert!(super::recover_module(&image).is_err());
    }
    let mut image = parse(bytes).unwrap();
    image.instructions[0] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn rounding_result_is_fixed_before_a_reference_mutating_call() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-round-snapshot/image"),
        include_str!("../../../docs/research/fixtures/native-round-snapshot/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-round-snapshot/platform.txt"),
    );
}

#[test]
fn month_start_result_is_fixed_before_a_reference_mutating_call() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-month-snapshot/image"),
        include_str!("../../../docs/research/fixtures/native-month-snapshot/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-month-snapshot/platform.txt"),
    );
}

#[test]
fn calendar_boundary_results_are_fixed_before_reference_mutating_calls() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-calendar-snapshots/image"),
        include_str!("../../../docs/research/fixtures/native-calendar-snapshots/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-calendar-snapshots/platform.txt"),
    );
}

#[test]
fn add_month_result_is_fixed_before_a_reference_mutating_call() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-add-month-snapshot/image"),
        include_str!("../../../docs/research/fixtures/native-add-month-snapshot/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-add-month-snapshot/platform.txt"),
    );
}

#[test]
fn date_construction_results_are_fixed_before_reference_mutating_calls() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-date-construction-snapshots/image"),
        include_str!(
            "../../../docs/research/fixtures/native-date-construction-snapshots/driver.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/native-date-construction-snapshots/platform.txt"
        ),
    );
}

#[test]
fn extra_builtin_results_are_fixed_before_reference_mutating_calls() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-extra-builtin-snapshots/image"),
        include_str!("../../../docs/research/fixtures/native-extra-builtin-snapshots/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-extra-builtin-snapshots/platform.txt"),
    );
}

#[test]
fn remaining_string_and_eval_results_are_fixed_before_reference_mutating_calls() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-remaining-result-snapshots/image"),
        include_str!(
            "../../../docs/research/fixtures/native-remaining-result-snapshots/driver.bsl"
        ),
        include_str!(
            "../../../docs/research/fixtures/native-remaining-result-snapshots/platform.txt"
        ),
    );
}

#[test]
fn numeric_conversion_result_is_fixed_before_a_reference_mutating_call() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-number-snapshot/image"),
        include_str!("../../../docs/research/fixtures/native-number-snapshot/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-number-snapshot/platform.txt"),
    );
}

#[test]
fn left_substring_result_is_fixed_before_a_reference_mutating_call() {
    assert_native_driver(
        include_bytes!("../../../docs/research/fixtures/native-left-snapshot/image"),
        include_str!("../../../docs/research/fixtures/native-left-snapshot/driver.bsl"),
        include_str!("../../../docs/research/fixtures/native-left-snapshot/platform.txt"),
    );
}

#[test]
fn try_return_source_control_matches_native_before_image_recovery() {
    let engine = open_bsl::Engine::builder().build().unwrap();
    let output = Output(std::rc::Rc::new(std::cell::RefCell::new(Vec::new())));
    let mut state = engine.state_builder().stdout(output.clone()).build();
    let module = engine
        .compile_entry(include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-try-return-source.bsl"
        ))
        .unwrap();
    state.run(&module).unwrap();
    let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
    assert_eq!(
        actual.trim(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-try-return-source.platform.txt"
        )
        .trim()
    );
}

#[test]
fn try_return_common_module_control_matches_native_before_image_recovery() {
    assert_recovered_source(
        include_str!(
            "../../../tests/conformance/measure/1c/native-image-try-return-control-module.bsl"
        )
        .into(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-try-return-common.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-try-return-common.platform.txt"
        ),
    );
}

#[test]
fn compiled_try_return_control_matches_native() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-try-return-compiled.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-try-return-compiled.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-try-return-compiled.platform.txt"
        ),
    );
}

#[test]
fn compiled_try_returns_reject_corrupt_handlers_and_cleanup_tails() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-try-return-compiled.image"
    );
    for (index, operand) in [
        (0, 22),
        (18, 0),
        (19, 46),
        (21, 0),
        (22, 35),
        (32, 1),
        (33, 46),
        (35, 1),
        (45, 1),
        (46, 48),
        (49, 66),
        (65, 0),
        (66, 74),
        (74, 1),
        (84, 1),
    ] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "Cmd[{index}]");
    }
}

#[test]
fn static_private_flag_two_procedures_match_native_reference_effects() {
    for (image, driver, expected) in [
        (
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-private-procedure-flag2.image"
            )
            .as_slice(),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-flag2.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-flag2.platform.txt"
            ),
        ),
        (
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-private-procedure-three.image"
            )
            .as_slice(),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-three.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-three.platform.txt"
            ),
        ),
    ] {
        assert_native_driver(image, driver, expected);
    }
}

#[test]
fn static_private_flag_two_rejects_unmeasured_external_calls_and_invalid_boundaries() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-procedure-three.image"
    );
    let mut dynamic = parse(bytes).unwrap();
    dynamic.procedures[0].flags = 32;
    dynamic.procedures[0].name = "Execute".into();
    assert!(
        super::recover_module(&dynamic)
            .unwrap_err()
            .contains("динамический вызов")
    );
    let mut wrong_arity = parse(bytes).unwrap();
    wrong_arity.procedures[1].parameter_count = 2;
    assert!(super::recover_module(&wrong_arity).is_err());
    let mut overlapping = parse(bytes).unwrap();
    overlapping.procedures[1].entry = 4;
    assert!(super::recover_module(&overlapping).is_err());
    let source = super::decompile(bytes).unwrap();
    let engine = open_bsl::Engine::builder()
        .configuration(open_bsl::ModuleGraphRecipe {
            modules: vec![open_bsl::ModuleRecipe {
                name: "ЗамерыТранзитивногоРодителя".into(),
                source,
                imports: vec![],
                visibility: open_bsl::ModuleVisibility::Named,
            }],
            eager_init: false,
        })
        .build()
        .unwrap();
    assert!(
        engine
            .compile_entry("ЗамерыТранзитивногоРодителя.Изменить(1, 2, 3);")
            .is_err()
    );
}

#[test]
fn private_flag_two_eval_preserves_native_caught_failure_and_argument() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-private-procedure-eval.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-eval-outcome.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-eval-outcome.platform.txt"
        ),
    );
}

#[test]
fn execute_source_control_matches_native_before_image_recovery() {
    assert_recovered_source(
        include_str!(
            "../../../tests/conformance/measure/1c/native-image-execute-control-module.bsl"
        )
        .into(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-execute-source.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-execute-source.platform.txt"
        ),
    );
}

#[test]
fn compiled_execute_command_matches_native_reference_effects() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-execute-compiled.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-execute-compiled.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-execute-compiled.platform.txt"
        ),
    );
}

#[test]
fn compiled_execute_rejects_invalid_operands_and_expression_state() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-execute-compiled.image");
    let mut wrong_operand = parse(bytes).unwrap();
    wrong_operand.instructions[2].operand = 1;
    assert!(super::recover_module(&wrong_operand).is_err());
    let mut missing_input = parse(bytes).unwrap();
    missing_input.instructions[1] = Instruction {
        code: 1,
        operand: 1,
    };
    assert!(super::recover_module(&missing_input).is_err());
    let mut residual = parse(bytes).unwrap();
    residual.instructions[0] = Instruction {
        code: 3,
        operand: 0,
    };
    assert!(super::recover_module(&residual).is_err());
    let mut invalid_flag = parse(bytes).unwrap();
    invalid_flag.procedures[1].flags = 4;
    assert!(super::recover_module(&invalid_flag).is_err());
}

#[test]
fn private_flag_two_execute_preserves_native_reference_effects() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-private-procedure-execute.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-execute.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-procedure-execute.platform.txt"
        ),
    );
}

#[test]
fn shared_private_procedure_tail_matches_both_native_entries() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-shared-tail-both.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-tail-both.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-tail-both.platform.txt"
        ),
    );
}

#[test]
fn shared_private_tail_rejects_corrupt_boundaries() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-shared-tail-both.image");
    for (index, operand) in [(4, 99), (3, 1), (6, 1)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "command {index}");
    }
    let mut image = parse(bytes).unwrap();
    image.procedures[1].entry = 6;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn shared_reference_body_matches_both_native_entries() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-shared-reference-body.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-reference-body.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-reference-body.platform.txt"
        ),
    );
}

#[test]
fn shared_reference_body_rejects_unmeasured_frames_and_boundaries() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-shared-reference-body.image"
    );
    for (index, operand) in [(0, 4), (2, 4), (6, 1), (10, 1), (11, 13), (13, 1)] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "command {index}");
    }
    let mut image = parse(bytes).unwrap();
    image.procedures[1].variables.as_mut().unwrap()[0].flags = 1;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.procedures[2].entry = 8;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn private_parameter_trace_baseline_matches_native_array_observation() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-private-variable-trace-baseline.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-variable-trace-baseline.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-variable-trace-baseline.platform.txt"
        ),
    );
}

#[test]
fn private_reference_parameter_slot_zero_matches_native() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-variable-slot-only.image"
    );
    assert_native_driver(
        bytes,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-variable-slot-only.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-variable-slot-only.platform.txt"
        ),
    );
    let mut image = parse(bytes).unwrap();
    image.procedures[1].variables.as_mut().unwrap()[0].external_slot = 1;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn shared_try_body_matches_both_native_entries() {
    assert_native_driver(
        include_bytes!("../../../tests/conformance/measure/1c/native-image-shared-try-body.image"),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-try-body.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-try-body.platform.txt"
        ),
    );
}

#[test]
fn shared_try_body_rejects_changed_guards_and_frames() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-shared-try-body.image");
    for (index, operand) in [
        (0, 4),
        (1, 0),
        (2, 8),
        (3, 1),
        (4, 8),
        (5, 0),
        (6, 9),
        (7, 1),
    ] {
        let mut image = parse(bytes).unwrap();
        image.instructions[index].operand = operand;
        assert!(super::recover_module(&image).is_err(), "command {index}");
    }
    let mut image = parse(bytes).unwrap();
    image.procedures[1].variables.as_mut().unwrap()[0].flags = 13;
    assert!(super::recover_module(&image).is_err());
    let mut image = parse(bytes).unwrap();
    image.procedures[1].entry = 5;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn shared_try_body_rejects_an_additional_entry_in_a_guard() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-shared-try-body.image"
    ))
    .unwrap();
    image.procedures[2].entry = 2;
    assert!(super::recover_module(&image).is_err());
}

#[test]
fn overlapping_entry_with_unknown_frame_is_not_silently_discarded() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-overlap-frame.image");
    let image = parse(bytes).unwrap();
    let private = &image.procedures[1];
    assert_eq!(private.entry, 4);
    assert!(private.defaults.is_none());
    assert_eq!(
        private
            .variables
            .as_ref()
            .unwrap()
            .iter()
            .map(|v| v.flags)
            .collect::<Vec<_>>(),
        vec![1338, 1459, 3091, 3027]
    );
    // Публичный проход измерен, но отдельный частный вызов обрушил платформу.
    // Эти данные не разрешают удалять объявление ради частичного исходника.
    assert!(super::decompile(bytes).is_err());
}

#[test]
fn shared_body_with_unused_ordinary_locals_matches_both_native_entries() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-shared-unused-frame.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-unused-frame.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-unused-frame.platform.txt"
        ),
    );
}

#[test]
fn shared_unused_frame_rejects_unmeasured_descriptors_and_slot_access() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-shared-unused-frame.image"
    );
    for changed in 0..5 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[0].variables.as_mut().unwrap()[1].flags = 1338,
            1 => image.procedures[1].variables.as_mut().unwrap()[0].flags = 13,
            2 => image.procedures[1].variables.as_mut().unwrap()[1].external_slot = 0,
            3 => image.instructions[8].operand = 1,
            4 => {
                let mut source = parse(bytes).unwrap();
                image.procedures[1].variables = source.procedures[0].variables.take();
            }
            _ => unreachable!(),
        }
        assert!(super::recover_module(&image).is_err(), "mutation {changed}");
    }
}

#[test]
fn shared_label_instruction_preserves_each_active_frames_target() {
    assert_native_driver(
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-shared-label-frame.image"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-label-frame.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-shared-label-frame.platform.txt"
        ),
    );
}

#[test]
fn shared_label_frame_normalization_rejects_unmeasured_shapes() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-shared-label-frame.image"
    );
    for changed in 0..9 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].variables.as_mut().unwrap()[0].flags = 1338,
            1 => image.procedures[1].variables.as_mut().unwrap()[1].external_slot = 0,
            2 => image.instructions[16].operand = 0,
            3 => image.procedures[0].labels[4] = 33,
            4 => image.procedures[1].labels[4] = 41,
            5 => image.instructions[32].operand = 1,
            6 => image.procedures[2].entry = 25,
            7 => {
                image.instructions[44] = super::Instruction {
                    code: 39,
                    operand: 32,
                }
            }
            8 => {
                image.instructions[44] = super::Instruction {
                    code: 42,
                    operand: 4,
                }
            }
            _ => unreachable!(),
        }
        let original: Vec<_> = image
            .instructions
            .iter()
            .map(|command| (command.code, command.operand))
            .collect();
        super::normalize_frame_scoped_shared_labels(&mut image);
        assert_eq!(
            image
                .instructions
                .iter()
                .map(|command| (command.code, command.operand))
                .collect::<Vec<_>>(),
            original,
            "mutation {changed}"
        );
        assert_eq!(image.procedures[1].entry, 19, "mutation {changed}");
    }
}

#[test]
fn linear_private_return_recovery_preserves_the_native_argument_value() {
    // Имя типа в английской квитанции нормализуется отдельно от значений.
    let expected = include_str!("../../../tests/conformance/measure/1c/measure-native-image-private-return-baseline.platform.txt")
        .replace("type=Number", "type=Число");
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-return-baseline.image"
    ))
    .unwrap();
    let source = super::recover_linear_module(&image).unwrap();
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-return-baseline.bsl"
        ),
        &expected,
    );
}

#[test]
fn measured_pure_private_return_images_match_all_native_receipts() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/measure/1c");
    for name in [
        "private-return-baseline",
        "private-return-unusual",
        "private-return-constant",
        "private-return-slot-one",
        "private-frame-read",
    ] {
        let image = std::fs::read(root.join(format!("native-image-{name}.image"))).unwrap();
        let driver =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.bsl"))).unwrap();
        // Меняется только локализованное имя типа, значения сохраняются.
        let expected =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.platform.txt")))
                .unwrap()
                .replace("type=Number", "type=Число");
        let source = super::decompile(&image).unwrap();
        assert!(source.contains("Функция Изменить(") || source.contains("Функция Прочитать0("));
        assert_recovered_source(source, &driver, &expected);
    }
}

#[test]
fn pure_constant_backed_return_rejects_unmeasured_frames_and_bodies() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-return-unusual.image"
    );
    for changed in 0..8 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].variables.as_mut().unwrap()[0].flags = 1339,
            1 => image.procedures[1].variables.as_mut().unwrap()[0].external_slot = 2,
            2 => image.procedures[1].defaults = None,
            3 => {
                image.instructions[11] = super::Instruction {
                    code: 16,
                    operand: 0,
                }
            }
            4 => {
                image.instructions[11] = super::Instruction {
                    code: 52,
                    operand: 0,
                }
            }
            5 => image.instructions[17].operand = 1,
            6 => image.instructions[20].operand = 20,
            7 => image.procedures[1].flags = 2,
            _ => unreachable!(),
        }
        assert!(
            super::measured_pure_private_return(&image, &image.procedures[1]).is_none(),
            "mutation {changed}"
        );
        assert!(
            super::recover_linear_module(&image).is_err(),
            "mutation {changed}"
        );
    }
}

#[test]
fn measured_numeric_private_writes_match_complete_native_receipts() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/measure/1c");
    for name in [
        "private-alias-write",
        "private-alias-write-observe-constant",
        "private-alias-direct-observation",
    ] {
        let image = std::fs::read(root.join(format!("native-image-{name}.image"))).unwrap();
        let driver =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.bsl"))).unwrap();
        let expected =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.platform.txt")))
                .unwrap()
                .replace("type=Number", "type=Число");
        let source = super::decompile(&image).unwrap();
        assert!(source.contains("Функция Изменить("));
        assert_recovered_source(source, &driver, &expected);
    }
}

#[test]
fn numeric_private_write_rejects_unmeasured_descriptors_and_edges() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-alias-write.image"
    );
    for changed in 0..9 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].variables.as_mut().unwrap()[0].flags = 1339,
            1 => image.procedures[1].variables.as_mut().unwrap()[0].external_slot = 1,
            2 => image.procedures[1].defaults = None,
            3 => image.instructions[15].operand = 1,
            4 => image.instructions[20].operand = 20,
            5 => image.instructions[13].operand = 1,
            6 => image.procedures[1].flags = 2,
            7 => {
                image.constants[0] = open_sdbl::metadata::Value::List(vec![
                    open_sdbl::metadata::Value::String("S".to_owned()),
                    open_sdbl::metadata::Value::String("7".to_owned()),
                ]);
            }
            8 => image.instructions[7].operand = 11,
            _ => unreachable!(),
        }
        assert!(
            super::measured_numeric_private_write(&image, &image.procedures[1]).is_none(),
            "mutation {changed}"
        );
        assert!(
            super::recover_linear_module(&image).is_err(),
            "mutation {changed}"
        );
    }
}

#[test]
fn measured_private_reads_without_defaults_match_the_complete_native_receipt() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-frame-no-defaults.image"
    );
    let source = super::decompile(bytes).unwrap();
    for slot in 0..4 {
        assert!(source.contains(&format!("Функция Прочитать{slot}(")));
        assert!(source.contains(&format!("Функция Проба{slot}(")));
    }
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults.platform.txt"
    ).replace("type=Number", "type=Число");
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults.bsl"
        ),
        &expected,
    );
}

#[test]
fn absent_private_defaults_do_not_weaken_frames_or_call_arity() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-frame-no-defaults.image"
    );
    for changed in 0..7 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.instructions[2].operand = 0,
            1 => image.instructions[2].operand = 2,
            2 => image.procedures[1].variables.as_mut().unwrap()[2].flags = 3092,
            3 => image.procedures[1].variables.as_mut().unwrap()[1].external_slot = 1,
            4 => image.procedures[1].flags = 2,
            5 => image.instructions[0].code = 52,
            6 => image.instructions[11].code = 16,
            _ => unreachable!(),
        }
        assert!(
            super::recover_linear_module(&image).is_err(),
            "mutation {changed}"
        );
    }
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/measure/1c");
    for name in [
        "private-frame-no-defaults-arities",
        "private-frame-no-defaults-two-arguments",
        "private-frame-no-defaults-distinct-second-argument",
    ] {
        let bytes = std::fs::read(root.join(format!("native-image-{name}.image"))).unwrap();
        assert!(super::decompile(&bytes).is_err(), "image {name}");
    }
    // Отсутствие DefPrm у одиночного кадра не измерено этим контролем.
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private-return-unusual.image"
    ))
    .unwrap();
    image.procedures[1].defaults = None;
    assert!(super::recover_linear_module(&image).is_err());
}

#[test]
fn frame_scoped_zero_label_function_paths_match_complete_native_receipts() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/measure/1c");
    for name in ["zero-label-private-path", "nonzero-label-private-path"] {
        let bytes = std::fs::read(root.join(format!("native-image-{name}.image"))).unwrap();
        let driver =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.bsl"))).unwrap();
        let expected =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.platform.txt")))
                .unwrap()
                .replace("type=Number", "type=Число");
        let source = super::decompile(&bytes).unwrap();
        for name in ["Проба", "Прочитать", "ПробаЧастногоВхода"] {
            assert!(source.contains(&format!("Функция {name}(")));
        }
        assert_recovered_source(source, &driver, &expected);
    }
}

#[test]
fn scoped_function_graphs_reject_unknown_frames_writes_and_control() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-zero-label-private-path.image"
    );
    for changed in 0..11 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].variables.as_mut().unwrap()[2].flags = 3092,
            1 => image.procedures[1].variables.as_mut().unwrap()[1].external_slot = 1,
            2 => {
                image.procedures[1].defaults =
                    Some(vec![Value::List(vec![Value::String(String::new())])])
            }
            3 => image.procedures[1].labels[4] = 25,
            4 => image.instructions[32].code = 16,
            5 => image.instructions[21].operand = 0,
            6 => image.instructions[58].operand = 55,
            7 => image.instructions[45].operand = 2,
            8 => image.instructions[43].code = 52,
            9 => image.procedures[0].variables.as_mut().unwrap()[0].flags = 13,
            10 => image.procedures[1].name = image.procedures[0].name.clone(),
            _ => unreachable!(),
        }
        assert!(
            super::scoped_functions::recover(&image).unwrap().is_err(),
            "mutation {changed}"
        );
    }
}

#[test]
fn frame_scoped_private_procedure_effects_match_both_native_receipts() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/measure/1c");
    for name in [
        "zero-label-procedure-queue",
        "nonzero-label-procedure-queue",
    ] {
        let bytes = std::fs::read(root.join(format!("native-image-{name}.image"))).unwrap();
        let driver =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.bsl"))).unwrap();
        let expected =
            std::fs::read_to_string(root.join(format!("measure-native-image-{name}.platform.txt")))
                .unwrap();
        let source = super::decompile(&bytes).unwrap();
        assert!(source.contains("Процедура Прочитать("));
        assert_recovered_source(source, &driver, &expected);
    }
}

#[test]
fn scoped_procedure_routes_reject_writes_and_unmeasured_callbacks() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-zero-label-procedure-queue.image"
    );
    for changed in 0..10 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].variables.as_mut().unwrap()[2].flags = 3092,
            1 => image.procedures[1].variables.as_mut().unwrap()[1].external_slot = 1,
            2 => {
                image.procedures[1].defaults =
                    Some(vec![Value::List(vec![Value::String(String::new())])])
            }
            3 => {
                image.constants[0] =
                    Value::List(vec![Value::String("S".into()), Value::String("7".into())])
            }
            4 => image.procedures[3].name = "ЗаполнитьЗначенияСвойств".into(),
            5 => image.instructions[55].code = 3,
            6 => image.instructions[55].code = 16,
            7 => image.procedures[1].labels[4] = 25,
            8 => image.instructions[56].operand = 2,
            9 => image.instructions[55].code = 52,
            _ => unreachable!(),
        }
        assert!(
            super::scoped_functions::recover(&image).unwrap().is_err(),
            "mutation {changed}"
        );
    }
}

#[test]
fn scoped_foreach_preserves_the_public_frame_and_private_native_result() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-foreach-frame.image"
    ))
    .unwrap();
    for name in ["Проба", "ЗакрытаяПроба", "ПроверитьЧастную"] {
        assert!(source.contains(&format!("Функция {name}(")));
    }
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-frame.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-frame.platform.txt"
        ),
    );
}

#[test]
fn scoped_foreach_rejects_unmeasured_iterator_and_loop_boundaries() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-foreach-frame.image"
    );
    for changed in 0..10 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[0].variables.as_mut().unwrap()[3].flags = 2,
            1 => image.procedures[0].variables.as_mut().unwrap()[3].external_slot = 0,
            2 => image.instructions[38].operand = 2,
            3 => image.instructions[37].operand = 19,
            4 => image.instructions[41].code = 32,
            5 => image.instructions[24].operand = 37,
            6 => image.instructions[29].code = 6,
            7 => image.instructions[37].code = 42,
            8 => image.instructions[34].code = 52,
            9 => image.procedures[0].variables.as_mut().unwrap()[0].flags = 13,
            _ => unreachable!(),
        }
        assert!(
            super::scoped_functions::recover(&image).unwrap().is_err(),
            "mutation {changed}"
        );
    }
}

#[test]
fn scoped_unreachable_target_does_not_split_an_expression_block() {
    for (bytes, receipt) in [
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-foreach-expression-target.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-expression-target.platform.txt"),
        ),
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-foreach-dead-target.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-dead-target.platform.txt"),
        ),
    ] {
        let source = super::decompile(bytes).unwrap();
        for name in ["Проба", "ЗакрытаяПроба", "ПроверитьЧастную"] {
            assert!(source.contains(&format!("Функция {name}(")));
        }
        assert_recovered_source(
            source,
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-frame.bsl"),
            receipt,
        );
    }
}

#[test]
fn scoped_reachable_mid_expression_jump_remains_an_error() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-foreach-expression-target.image"
    );
    for changed in 0..2 {
        let mut image = parse(bytes).unwrap();
        if changed == 0 {
            image.procedures[0].entry = 62;
        } else {
            image.instructions[0] = super::Instruction {
                code: 39,
                operand: 62,
            };
        }
        assert!(super::scoped_functions::recover(&image).unwrap().is_err());
    }
}

#[test]
fn scoped_conditional_returns_preserve_both_native_alternatives() {
    for (bytes, driver, expected) in [
        (
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-scoped-return-join-40.image"
            )
            .as_slice(),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-scoped-return-join-40.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-scoped-return-join-40.platform.txt"
            ),
        ),
        (
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-scoped-return-join-41.image"
            )
            .as_slice(),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-scoped-return-join-41.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-native-image-scoped-return-join-41.platform.txt"
            ),
        ),
    ] {
        let source = super::decompile(bytes).unwrap();
        for name in ["Проба", "ЗакрытаяПроба", "ПроверитьЧастную"]
        {
            assert!(source.contains(&format!("Функция {name}(")));
        }
        assert_recovered_source(source, driver, expected);
    }
}

#[test]
fn scoped_conditional_returns_reject_unsupported_merges_and_cleanup() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-return-join-40.image"
    );
    for changed in 0..6 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.instructions[54].code = 52,
            1 => image.instructions[55].operand = 51,
            2 => {
                image.instructions[51] = super::Instruction {
                    code: 42,
                    operand: 0,
                }
            }
            3 => image.instructions[52].code = 16,
            4 => image.instructions[53].operand = 1,
            5 => image.instructions[49].operand = 99,
            _ => unreachable!(),
        }
        assert!(
            super::scoped_functions::recover(&image).unwrap().is_err(),
            "mutation {changed}"
        );
    }
}

#[test]
fn scoped_infeasible_call_edge_preserves_argument_preparation() {
    for (bytes, expected) in [
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-infeasible-call-edge-40.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-infeasible-call-edge-40.platform.txt"),
        ),
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-infeasible-call-edge-41.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-infeasible-call-edge-41.platform.txt"),
        ),
    ] {
        let source = super::decompile(bytes).unwrap();
        for name in ["Проба", "ЗакрытаяПроба", "ПроверитьЧастную"] {
            assert!(source.contains(&format!("Функция {name}(")));
        }
        assert_recovered_source(
            source,
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-frame.bsl"),
            expected,
        );
    }
}

#[test]
fn scoped_incomplete_call_edges_remain_errors_when_feasible_or_unknown() {
    for (bytes, opposite) in [
        (include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-infeasible-call-edge-40.image").as_slice(), 6),
        (include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-infeasible-call-edge-41.image").as_slice(), 7),
    ] {
        for changed in 0..2 {
            let mut image = parse(bytes).unwrap();
            image.instructions[52] = super::Instruction {
                code: if changed == 0 { opposite } else { 3 },
                operand: 0,
            };
            assert!(super::scoped_functions::recover(&image).unwrap().is_err(), "mutation {changed}");
        }
    }
}

#[test]
fn scoped_foreach_outside_trampolines_replay_complete_native_receipts() {
    for (bytes, driver, receipt) in [
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-foreach-outside-trampoline.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-trampoline.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-trampoline.platform.txt"),
        ),
        (
            include_bytes!("../../../tests/conformance/measure/1c/native-image-scoped-foreach-outside-conditional.image").as_slice(),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-conditional.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-conditional.platform.txt"),
        ),
    ] {
        let source = super::decompile(bytes).unwrap();
        for name in ["Проба", "ЗакрытаяПроба", "ПроверитьЧастную"] {
            assert!(source.contains(&format!("Функция {name}(")));
        }
        assert_recovered_source(source, driver, receipt);
    }
}

#[test]
fn scoped_foreach_outside_trampolines_reject_changed_routes() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-foreach-outside-trampoline.image"
    );
    for changed in 0..5 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.instructions[9].code = 52,
            1 => image.instructions[9].operand = 9,
            2 => image.instructions[9].operand = 10,
            3 => image.instructions[38].operand = 10,
            _ => image.instructions[9].code = 42,
        }
        assert!(
            super::scoped_functions::recover(&image).unwrap().is_err(),
            "mutation={changed}"
        );
    }
}

#[test]
fn scoped_backward_boolean_assignment_replays_every_native_observation() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-backward-bool.image"
    ))
    .unwrap();
    assert!(source.contains("Процедура Проба("));
    assert!(source.contains("Функция ЗакрытаяПроба("));
    assert!(source.contains("Функция ПроверитьЧастную("));
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scoped-backward-bool.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scoped-backward-bool.platform.txt"
        ),
    );
}

#[test]
fn scoped_backward_boolean_assignment_rejects_unmeasured_shared_entries() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-backward-bool.image"
    );
    for changed in 0..7 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => {
                image.instructions[13] = Instruction {
                    code: 39,
                    operand: 11,
                }
            }
            1 => image.instructions[10].code = 7,
            2 => image.instructions[11].code = 52,
            3 => image.instructions[20].code = 6,
            4 => image.instructions[21].operand = 10,
            5 => image.instructions[17].code = 4,
            _ => image.procedures[0].flags = 3,
        }
        assert!(
            super::scoped_functions::recover(&image).unwrap().is_err(),
            "mutation={changed}"
        );
    }
}

#[test]
fn private_procedure_with_27_zero_labels_preserves_the_native_queue_receipt() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-zero-label-procedure-27.image"
    ))
    .unwrap();
    assert!(source.contains("Процедура Прочитать("));
    assert!(source.contains("Функция Проба("));
    assert!(source.contains("Функция ПробаЧастногоВхода("));
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27.platform.txt"
        ),
    );
}

#[test]
fn private_procedure_27_label_table_rejects_unmeasured_frames_and_routes() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-zero-label-procedure-27.image"
    );
    for changed in 0..5 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].labels[4] = 32,
            1 => {
                image.procedures[1].labels.pop();
            }
            2 => image.procedures[1].flags = 1,
            3 => image.procedures[1].variables.as_mut().unwrap()[0].flags = 5,
            _ => image.procedures[1].variables.as_mut().unwrap()[3].external_slot = -1,
        }
        assert!(
            super::scoped_functions::recover(&image).is_none_or(|result| result.is_err()),
            "mutation={changed}"
        );
    }
}

#[test]
fn private_27_label_string_frames_replay_every_native_observation() {
    for bytes in [
        include_bytes!("../../../tests/conformance/measure/1c/native-image-zero-label-procedure-27-string.image").as_slice(),
        include_bytes!("../../../tests/conformance/measure/1c/native-image-zero-label-procedure-27-string-frame18.image").as_slice(),
    ] {
        let source = super::decompile(bytes).unwrap();
        for name in ["Проба", "ПробаЧастногоВхода"] {
            assert!(source.contains(&format!("Функция {name}(")));
        }
        assert!(source.contains("Процедура Прочитать("));
        assert_recovered_source(
            source,
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27-string.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27-string.platform.txt"),
        );
    }
}

#[test]
fn private_27_label_string_frames_reject_unmeasured_combinations() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-zero-label-procedure-27-string.image"
    );
    for changed in 0..5 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].labels.truncate(5),
            1 => image.procedures[1].flags = 1,
            2 => image.procedures[1].labels[4] = 32,
            3 => image.procedures[1].variables.as_mut().unwrap()[2].flags = 5,
            _ => image.constants[0] = Value::List(vec![Value::String("U".into())]),
        }
        assert!(
            super::scoped_functions::recover(&image).is_none_or(|result| result.is_err()),
            "mutation={changed}"
        );
    }
}

#[test]
fn ordinary_shared_context_replays_the_complete_native_module_receipt() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-ordinary-shared-context.image"
    ))
    .unwrap();
    assert!(source.contains("Функция Проба("));
    assert!(source.contains("Функция Изменить("));
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-ordinary-shared-context.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-ordinary-shared-context.platform.txt"
        ),
    );
}

#[test]
fn ordinary_scoped_assembly_rejects_incomplete_or_unsupported_declarations() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-ordinary-shared-context.image"
    );
    for changed in 0..7 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => {
                image.procedures[0].defaults.as_mut().unwrap().pop();
            }
            1 => image.procedures[0].variables.as_mut().unwrap()[0].flags = 1338,
            2 => image.procedures[0].labels[4] = 999,
            3 => image.procedures[0].flags = 3,
            4 => image.variables[0].flags = 17,
            5 => image.procedures[2].entry = 8,
            _ => image.procedures.push(super::Procedure {
                name: "НеподдержаннаяПроба".into(),
                flags: 3,
                parameter_count: 0,
                entry: 999,
                variables: None,
                defaults: None,
                labels: Vec::new(),
            }),
        }
        assert!(
            super::recover_linear_module_with_scoped_bodies(&image, true).is_err(),
            "mutation={changed}"
        );
    }
}

#[test]
fn contextual_private_frame_replays_the_complete_native_receipt() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private27-context.image"
    ))
    .unwrap();
    assert!(source.contains("Функция Проба("));
    assert!(source.contains("Функция ПробаЧастногоВхода("));
    assert!(source.contains("Процедура Прочитать("));
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private27-context.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-private27-context.platform.txt"
        ),
    );
}

#[test]
fn contextual_private_frame_rejects_unmeasured_declarations() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-private27-context.image"
    );
    for changed in 0..9 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.procedures[1].parameter_count = 2,
            1 => {
                image.procedures[1].defaults =
                    Some(vec![Value::List(vec![Value::String(String::new())])])
            }
            2 => image.procedures[1].variables.as_mut().unwrap()[2].flags = 5,
            3 => image.procedures[1].variables.as_mut().unwrap()[3].external_slot = -1,
            4 => image.procedures[1].labels[4] = 32,
            5 => image.constants[0] = Value::List(vec![Value::String("U".into())]),
            6 => image.variables[0].flags = 17,
            7 => image.procedures[4].entry = 8,
            _ => image.procedures.push(super::Procedure {
                name: "НеподдержанныйКадр".into(),
                flags: 3,
                parameter_count: 0,
                entry: 999,
                variables: None,
                defaults: None,
                labels: Vec::new(),
            }),
        }
        assert!(
            super::recover_linear_module_with_scoped_bodies(&image, true).is_err(),
            "mutation={changed}"
        );
    }
}

#[test]
fn ordinary_backward_shared_target_replays_the_native_receipt() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-ordinary-backward-shared.image"
    ))
    .unwrap();
    assert!(source.contains("Функция ПерваяПробы("));
    assert!(source.contains("Функция ВтораяПробы("));
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-ordinary-backward-shared.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-ordinary-backward-shared.platform.txt"
        ),
    );
}

#[test]
fn scoped_initializer_replays_the_complete_native_module_receipt() {
    let source = super::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-initializer.image"
    ))
    .unwrap();
    assert!(source.contains("Функция Проба("));
    assert!(source.contains("Функция Изменить("));
    assert_recovered_source(
        source,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scoped-initializer.bsl"
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-scoped-initializer.platform.txt"
        ),
    );
}

#[test]
fn scoped_initializer_rejects_invalid_or_unmeasured_control() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-initializer.image"
    );
    for changed in 0..6 {
        let mut image = parse(bytes).unwrap();
        match changed {
            0 => image.entry = image.instructions.len() as u32,
            1 => image.instructions[23].operand = 24,
            2 => image.instructions[41].operand = 999,
            3 => {
                image.instructions[40] = Instruction {
                    code: 3,
                    operand: 0,
                }
            }
            4 => {
                image.instructions[40] = Instruction {
                    code: 7,
                    operand: 0,
                }
            }
            _ => {
                image.instructions[44] = Instruction {
                    code: 1,
                    operand: 1,
                }
            }
        }
        let error = super::recover_linear_module_with_scoped_bodies(&image, true).unwrap_err();
        assert!(error.contains("инициализ"), "mutation={changed}: {error}");
    }
}

#[test]
fn native_source_syntax_string_defaults_are_bare_constants() {
    let mut image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-boolean-defaults.image"
    ))
    .unwrap();
    image.procedures[0].defaults.as_mut().unwrap()[0] = Value::List(vec![
        Value::String("S".into()),
        Value::String(String::new()),
    ]);
    let source = super::recover_linear_module(&image).unwrap();
    assert!(source.contains(" = \"\""));
    assert!(!source.contains(" = (\"\")"));
    open_bsl::Engine::builder()
        .build()
        .unwrap()
        .compile_entry(&source)
        .unwrap();
}

#[test]
fn native_source_syntax_initializer_ends_without_a_top_level_return() {
    let image = parse(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-scoped-initializer.image"
    ))
    .unwrap();
    let constants = super::constant_literals(&image.constants).unwrap();
    let source = super::scoped_functions::recover_initializer(&image, &constants).unwrap();
    assert!(!source.contains("Возврат"));
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile_entry(&source).unwrap();
    assert_eq!(
        engine.state_builder().build().run(&module).unwrap(),
        open_bsl::Value::Undefined
    );
}
