# Evidence

`tests/conformance/measure/1c/measure-module-transitive-unused.platform.txt` records `module.transitive.good = 42` on native 1C 8.3.27.2342. Its isolated configuration has a valid parent export and another parent export referencing a child whose neighboring export reads an undefined variable. The accessed-child companion probe did not finish within 90 seconds despite wrapping the call in `Попытка/Исключение`; the native journal records an undefined-variable compilation error in the child at line 6. This is a failed probe with a compiler diagnostic, not a successful empty-output measurement.

The previous same-module probe in `docs/research/configuration-session-startup.md` still applies: an undefined variable inside the accessed common module prevents that module from being used. Deferral applies to the entire affected module, not to individual faulty methods.

The additional native `measure-module-transitive-arguments.bsl` probe records
both a parent-body marker and an argument-evaluation marker before the child's
compilation error. Therefore a deferred child failure must not be raised at
parent-method entry or before evaluating the imported call's arguments. Keep
the existing call evaluation order and fail on access to the child instance.

# Compatibility

Keep existing facade compilation strict by default. Add the explicit `EngineBuilder::defer_common_module_name_errors` opt-in. `SessionFactory` enables it for named and global common modules. Private object/manager modules and explicit session modules retain their existing semantic diagnostics. Ordinary standalone compilation remains strict. Existing tests must not be modified to accept the policy.

# Mechanism

Pass the policy through both rooted selection and final catalog compilation so they agree. Extend the configuration resolver through an additive entry point while preserving its existing entry points. An undefined-variable error in a selected common module becomes a module compilation failure on first access. Do not evaluate unknown names, substitute permissive values, or execute a valid export of a faulty module. Parse errors, signatures, arity behavior, and other semantic errors keep their existing contracts.

Do not add dependencies. Reuse module instances and the existing exception classification boundary. The current deferred bare-module-name mechanism inserts an ordinary catchable `Raise`; this cannot be reused unchanged for the measured compilation failure. Introduce `RtError::ModuleCompilation(String)` and classify it as not being a BSL exception. Introduce `RStmtKind::RaiseCompilationError(String)` and the cold instruction `Instr::RaiseCompilationError { src }`. Its operand reads a register containing the diagnostic string. The existing compiler emits the string constant and instruction at the beginning of the affected module initializer; the existing VM unwind boundary preserves the fatal classification. No second resolver, executor, or component library is introduced.

The instruction is serialized by the existing text instruction table and validated by the existing image checker. Increment `FORMAT_VERSION` from 41 to 42, add a round-trip test and malformed-operand test, and classify the instruction in the single effects table. This preserves failures in portable images without adding a field to every `Program` or `ModuleProgram` constructor. Preserve export surfaces for static linking, but prevent any export body from running after module compilation has failed.

The user approved the mechanical transition of four existing header assertions in `bsl-bytecode/tests/{text_format,imports}.rs` from version 41 to 42. The compiled-opcode corpus is extended with a real deferred-error program without weakening its exhaustive coverage assertion.

# Limits

Lexical matches in native buh provide candidate dependency paths, not proof that the session handler executes the faulty export. An actual startup run after the change must establish the next failure or success. The failed accessed-child probe proves that the current ordinary catchable-Raise shortcut is insufficient; it does not establish platform retry semantics after such a compilation failure.

# Unknown global functions in method bodies

The native controls in `docs/research/fixtures/deferred-common-function-errors` use the same isolated parent and child configuration with an unknown global function instead of an unknown variable. The unused child control returns 42 and completes. Accessing the child records the unknown-function compiler diagnostic in the native journal and does not complete within 45 seconds despite a caller exception handler. Its partial output includes only the begin and parent markers. Preserve this failed run as evidence, not as a successful oracle.

Extend only the method-body branch for an unknown global function. Keep the existing missing-imported-method branch first: its measured method-scoped behavior is different. Default parameter expressions and top-level unknown-function errors retain strict rejection because this control does not measure them. No setter stub for `SetInfoBaseTimeZone` is introduced; an actual access to a module containing that unsupported call still fails.

The user explicitly approved updating the existing unknown-bare-function expectation to the native measured behavior. Strict default and private/explicit module assertions remain unchanged; the new dedicated unknown-function tests cover unused and accessed common modules.
