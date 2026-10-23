# Change: Implement handler-scoped error descriptions

## Why
Configuration callbacks and ordinary BSL handlers currently receive an unconditional component error when they call `ОписаниеОшибки`. This replaces the diagnostic they were handling and prevents meaningful diagnosis of original configuration code. A native control now proves that descriptions belong to the active handler of a particular call, rather than to the last exception of the task.

## What Changes
- Implement the existing zero-argument `ОписаниеОшибки` / `ErrorDescription` function using the active handler of the current call.
- Restore the outer handler's description after a nested handler finishes and return an empty string outside a handler.
- Keep handler descriptions isolated across ordinary calls and sessions.
- Preserve existing structured error values and exception propagation; do not introduce a second exception representation.

## Impact
- Affected specification: new `error-description` capability.
- Affected implementation: existing runtime function registration, compiler exception metadata and VM handler state as required by the verified design.
- The unconditional unsupported error is replaced by measured behavior. No database dependency or new external dependency is introduced.
- Bytecode representation changes, if required, must be explicit and versioned before implementation; existing handler ranges do not currently encode the end of a handler body.
