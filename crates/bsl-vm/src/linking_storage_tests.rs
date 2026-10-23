use super::*;

#[test]
fn linking_straight_line_code_keeps_empty_method_cells_within_pointer_budget() {
    let program = crate::tests::compile_module(&"А = 1;\n".repeat(10_000));
    let env = bsl_rt::HostEnv::process();
    let linked = link_components(
        &program,
        None,
        env.zone(),
        env.files(),
        env.random(),
        env.network(),
        env.host_data(),
        env.background_jobs(),
        env.temp_storage(),
        env.message_sink(),
        0,
    )
    .unwrap();
    let instructions: usize = program.chunks.iter().map(|chunk| chunk.instrs.len()).sum();
    let cells: usize = linked.object_methods.iter().map(Vec::len).sum();
    let bytes: usize = linked
        .object_methods
        .iter()
        .map(|chunk| chunk.capacity() * std::mem::size_of_val(&chunk[0]))
        .sum();
    assert_eq!(cells, instructions);
    assert!(linked.object_methods.iter().flatten().all(Option::is_none));
    assert!(
        bytes <= instructions * std::mem::size_of::<usize>(),
        "пустые ячейки методов: {bytes} байт на {instructions} инструкций"
    );
}
