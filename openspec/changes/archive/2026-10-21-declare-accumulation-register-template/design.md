# Design

Use the existing information-register GetTemplate declaration pattern. Native vendor help 8.3.27.2342 confirms ПолучитьМакет / GetTemplate with one required argument. Evidence: target/diagnostics/accumulation-register-template-native-1016/vendor-page.json, exact document platform-syntax-help:syntax-context:objects/catalog125/catalog204/object210/methods/GetTemplate538.html. Retain native controls before implementation.

Add one method descriptor and a narrow explicit unsupported-retrieval callback; extend existing export-precedence admission to this manager kind. Existing receiver wiring supplies bare calls; no global function or permissive unresolved-name fallback is added. Preserve module/local variables and same-named exports. The reached call occurs in a dormant export; compilation success is not evidence that retrieval runs during startup. If startup reaches actual retrieval, implement it from native evidence before claiming success.

A prior information-register test forbids this method for accumulation registers. Prepare a one-line expectation correction and obtain user approval before applying it; do not rewrite the old test to hide a regression. No external dependency or RT/VM boundary change.

Native controls are retained in docs/research/fixtures/accumulation-register-template-1016. Both static dormant calls compile; dynamic bare and qualified aliases resolve, with exact arity one. The platform returns Undefined for an Undefined argument and rejects absent String names, Null and Number. The current explicit unsupported callback deliberately does not emulate retrieval, including that Undefined case; no successful retrieval behavior is claimed.
