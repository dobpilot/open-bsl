# Why

The buh startup closure reaches `ОбщегоНазначенияБП` through neighboring exports of `ОбщегоНазначенияПереопределяемый`. The first module contains an undefined variable in an unrelated export. Eager semantic rejection prevents even entering the session handler. Native 1C 8.3.27.2342 returns 42 when a valid parent export is called and an unused neighboring export references a child module with an undefined variable. This differs from an undefined variable inside the accessed module itself.

# What Changes

- Add an explicit facade builder policy for deferring common-module undefined-variable errors until the affected module is initialized.
- Enable that policy for configuration Session factories; preserve strict defaults for existing facade consumers and preserve strict private object and manager module validation.
- Reuse the existing deferred module initialization error and existing resolver, compiler, and VM pipeline.
- Verify both the unused-child and accessed-child native cases before implementing the error propagation contract.

# Impact

Affected crates: `open-bsl`, `bsl-sema`, and `bsl-session`. No new dependencies, database access, source rewriting, or separate language implementation. This change does not promise successful buh initialization: an actual access to the faulty common module must still fail.
