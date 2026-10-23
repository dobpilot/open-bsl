## Implementation
- [x] Validate this change strictly before code changes.
- [x] Add defaulted loader options and propagate them through base and extensions.
- [x] Reuse measured signature formatting for exported-only placeholders and preserve strict complete recovery.
- [x] Add and document the CLI flag with argument-validation coverage (three new CLI tests pass).
- [x] Test default procedures/functions, signatures, skipped private bodies and initializer, malformed exports, unchanged text and explicit complete recovery (ten new loader tests, including extension replacement and async exports;458 loader tests pass without filters).
- [x] Run formatting, workspace Clippy/tests, rustdoc and consumer/dependency checks (fmt/Clippy/rustdoc/strict and consumer build pass; runtime/VM trees unchanged; workspace3486pass/0fail/1ignored/0filtered, loader458pass; user-approved existing-test correction applied).
- [x] Recover measured async exported interfaces as explicit synchronous placeholders, verify flags18/19 and repeat the original read-only startup (native method-flag2 receipt and dedicated test pass; UNF exposes an unsupported opaque parameter signature, demo reaches its existing conditional-separation SDK error in448.41s; successful original startup is not claimed).
