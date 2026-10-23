## Implementation
- [x] Validate this change strictly before implementation.
- [x] Add a failing external unit test for the approved opaque shape and negative variants (opaque-export-red-1011.log reproduces the refusal).
- [x] Implement only the interface-mode exception and document the host policy.
- [x] Verify argument preservation and strict full-mode refusal; repeat original loading (462 loader tests pass; generalized one/two/three-parameter controls pass; original UNF loading passes the protected headers and reaches UndefinedFunction("ПолучитьСсылку") in a characteristic manager module).
- [x] Run formatting, workspace lint/tests, rustdoc and facade checks, then archive (condition-adapter logs: all commands exit 0; workspace process 69155 exit 0).
