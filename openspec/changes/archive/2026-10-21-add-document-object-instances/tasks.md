# Tasks

- [x] 1. Inspect original UNF calls and measure string-number document body and initial properties on native 1C.
- [x] 2. Validate the OpenSpec change strictly before runtime/API implementation.
- [x] 3. Specify and obtain document configuration descriptors from open-sdbl, checking native XML against decoded acquisition.
- [x] 4. Measure numeric/disabled numbering, assignment, Fill and object export state on native 1C as required by reached code.
- [x] 5. Implement CreateDocument, exact document object types and module instances with measured standard properties.
- [x] 6. Verify aliases, arity, original exports, initialization failures, per-object state and cross-State isolation through the facade.
- [x] 7. Regenerate the API reference and execute original UNF startup; retain exact next failures without claiming full completion.
- [x] 8. Run formatting, Clippy, workspace tests, rustdoc, consumer and dependency checks; archive after review and green checks.

- [x] 9. Expose instance-bound non-exported platform handler invocation through the facade and verify receiver/parameter/session isolation with consumer tests and workspace gates.

- [x] 10. Measure the three-parameter Fill event and expose read-only compiled function parameter counts through the facade; verify non-execution and missing functions with consumer tests and workspace gates.

- [x] 11. Add immutable owned attribute descriptors using existing ColumnType and SerializedValueGraph; verify unknown/empty states, filling default materialization and Send+Sync, and run workspace gates.

- [x] 12. Measure handler replacement, mutation and removal of FillingData on native 1C; reproduce the mismatch through the facade and use the final parameter returned by the existing VM callback.

- [x] 13. Acquire document attribute types and filling settings from the live configuration, support types reached by original startup, and verify them against native configuration XML and runtime output.

- [x] 14. Measure document DataExchange and AdditionalProperties during and after initialization; reuse existing exchange components, verify aliases and isolation with native replay, regenerate the reference, and pass workspace gates.

- [x] 15. Measure single-type Boolean and datetime attributes on native 1C, reuse existing conversions, verify creation/assignment/filling with native oracles, and pass all workspace gates without changing existing expectations.

- [x] 16. Measure single catalog-reference and reference/string attributes, including explicitly empty composite values, reuse existing reference types and conversions, and verify the native replay and all workspace gates.

- [x] 17. Reproduce the unsupported enumeration attribute failure through the facade, support measured single enumeration and enumeration/string declarations using existing type adjustment, verify 34 native creation/assignment/filling/storage observations and pass workspace gates without changing existing expectations.

- [x] 18. Verify the 14 native configured enumeration FillValue observations through immutable value graphs and acquire the corresponding DesignTimeRef from the configuration descriptor.

- [x] 19. Replay all 14 configured enumeration FillValue observations using the existing graph in two independent Sessions; preserve existing runtime behavior and pass workspace gates. SDK acquisition in task 18 remains outstanding.

- [x] 20. Replay the 26 measured document-reference attribute observations, support the acquired concrete types and validate standard filling before integrating the original defined-type declarations.

- [x] 21. Support single document-reference and document-reference/string attributes, measure GetRef and preservation/storage cases, replay all 41 observations through the facade, reuse the existing UUID generator/reference factory and pass workspace gates. Original defined-type integration in task 20 remains outstanding.

- [x] 22. Replay both 36-observation mixed-reference declarations through the facade and pass workspace gates.
- [x] 23. Measure the original 56-member UNF event-basis DefinedType in an isolated native configuration, retaining the declaration and all 224 type/UUID observations.
- [x] 24. Reproduce the cardinality restriction through the facade, support every measured member using existing type adjustment, replay the full oracle, and pass workspace gates before integration with SDK acquisition.

- [x] 25. Measure creation, assignment and standard filling for the ValueStorage attribute reached in original UNF; retain all 16 native observations.
- [x] 26. Reproduce and implement the measured ValueStorage document attribute behavior using the existing ORM storage and graph, replay the oracle and pass workspace gates.

- [x] 27. Measure fixed-length String document numbers on native 1C and preserve the 15-observation oracle before projecting the SDK allowed-length flag.
- [x] 28. Represent fixed-length String numbers in ORM settings, reuse existing string adjustment, replay the native oracle and integrate the acquired SDK flag without changing existing regression expectations.

- [x] 29. Reproduce original UNF object-module failure on Запасы, confirm ownership using native XML and measure section creation, typed rows, filtered unloading, deletion and object isolation on native 1C.
- [x] 30. Acquire full document section and row descriptors, preserve exact configuration-owned types and register section names in the existing object-module context.
- [x] 31. Replay the 13 native observations through the facade with real section/row objects and the existing value-table output; verify the original UNF module and all workspace gates without changing regression expectations.

- [x] 32. Measure English aliases and the seven additional initialization, read-only numbering and identity observations; replay both native scripts through concrete section/row objects in independent Sessions and regenerate the API reference. Full acquisition and original UNF verification remain tasks 30 and 31.

- [x] 33. Reproduce the original Движения context failure and measure the empty collection and an actual UNF-named subordinate information register in isolated native configurations, preserving all 21 observations.
- [x] 34. Implement the acquired movement association descriptors, standard object context property and native record-set/collection identities; replay all 29 extended observations through the facade and pass workspace gates. SDK acquisition remains task 35.
- [x] 35. Obtain complete document register associations from SDK acquisition, verify the original Event's one register and ЗаказПокупателя's 51 against native XML, and repeat original session startup without substituting empty collections.

- [x] 36. Reproduce the original УстановитьСсылкуНового failure and measure all 21 document future-reference observations, including initialization, aliases, wrong kinds and preservation after rejection.
- [x] 37. Reuse existing typed references and compatibility checks for document future-reference methods; replay the native oracle, regenerate the API reference, repeat original UNF startup and pass workspace gates. Original startup now stops at SetNewNumber; full initialization remains incomplete.

- [x] 38. Measure explicit new-number assignment without document writes on native 1C and retain all six observations; read the installed syntax-help method and event contracts before designing allocation.
- [x] 39. Record the user-approved deferral of actual live-infobase numbering; retain the original work in `docs/plans/document-live-number-allocation.md` without claiming implementation.
- [x] 40. Preserve four additional native event observations: cancellation retains a manually assigned number, prefix mutation affects standard allocation, and both calls receive the original event parameters. Periodic and concurrent allocation remains task 39.
- [x] 41. Measure the original Year periodicity with length 11, preserve six date/prefix observations, and repeat the script in the same native infobase after process termination to verify the observed reservation lifetime. Concurrent sessions and stored documents remain unverified.
- [x] 42. Measure SetNewNumber during document-module initialization and retain rejection followed by the first successful selected-object allocation.
- [x] 43. Add the instance-bound numbering event and session data-adapter dispatch, verify cancellation, final prefix/date delivery, missing capability, both aliases and native body behavior, then regenerate the API reference and pass workspace gates. Actual infobase allocation remains task 39.
- [x] 44. Measure numeric numbering separately on native 1C, retaining initial zero, repeated allocation and acceptance of a String prefix that yields the next numeric number; do not infer argument rejection from the help note.
- [x] 45. Measure the numeric single-parameter numbering event and reproduce the interpreter's incorrect two-argument delivery with a new native-oracle test before fixing it.
- [x] 46. Deliver the measured numeric handler's single argument, replay all numbering event tests in isolated Sessions and pass workspace gates without changing existing expectations.
- [x] 47. Measure the String-number single-parameter event and reproduce the erroneous type restriction with a new native-oracle test before removing it.
- [x] 48. Verify both measured single-parameter event shapes and the two-parameter prefix event, and repeat workspace gates after removing the unsupported type restriction.
- [x] 49. Measure the effect of a natively stored document on numbering in a disposable infobase; preserve separate first-run and restarted-process oracles without writing original demo/buh/unf data.
- [x] 50. Retain the SDK's acquired periodicity and numerator identities in immutable ORM descriptors without confusing unknown scope and known absence; verify all five native periodicities and shared numerator IDs across the adapter boundary.
- [x] 51. Record the user-approved deferral of actual live-infobase numbering; retain the original work in `docs/plans/document-live-number-allocation.md` without claiming implementation.
- [x] 52. Measure concurrent unpersisted reservations in two real native file-infobase sessions with the first object still held, preserve both numbers and process results, and record that independent session counters and stored-row maxima do not reproduce this observation.

Reconciliation checkpoint (2026-10-21): task 18 combines the unchanged fourteen-line configured-enumeration oracle replay in `crates/bsl-session/tests/document_objects.rs::configured_enumeration_filling_value_matches_native_through_the_graph` (two independent Sessions) with actual acquired DesignTimeRef projection in `crates/bsl-config-db/src/document_filling_acquisition_tests.rs::native_enumeration_filling_value_survives_acquisition_and_projection`. Both pass in the retained final-checks-adapted workspace log. Task 35 is proved by `document_registers_tests.rs` comparing the original acquired Event and CustomerOrder descriptors with native XML identities (one and all fifty-one registers in order), together with the successful original UNF startup in all three session kinds in the retained SDK 0.9.35 matrix. Current SDK 0.9.36 global gates and refreshed startup remain separate outstanding integration work; these historical receipts are not relabelled as new runs. Real infobase number allocation in tasks 39/51 is not claimed implemented.

Reconciliation checkpoint: tasks 4 and 6 are verified by the existing 26-test `document_objects.rs` native replay suite in the retained final-checks-adapted log. `measured_document_body_and_assignment_match_native_oracles` covers String, Number and disabled-number variants; filling events, handler-returned data and existing Fill exports have dedicated native replay tests. Alias/type/arity, repeated initialization failures, separate module counters on two objects in each of two Sessions, and owned attribute isolation are asserted directly. This closes those measured object tests only; it does not assert real infobase allocation or post-SDK36 global completion.

Tabular-section reconciliation: task 30 is verified by `document_filling_acquisition_tests::acquired_event_section_fields_match_native_ownership_and_order`, comparing all 37 native XML fields in five original UNF Event sections with acquired Config ownership/order and ORM projection. SessionFactory registers configuration-owned section types and includes acquired section names in the existing object-module property context. Task 31 is verified by `crates/bsl-session/tests/document_tabular_sections.rs`: the original thirteen-observation section/row/unload script replays in two Sessions, supplementary initialization/identity controls and direct object-module section access pass. These tests and the unchanged existing suite pass in the retained final-checks-adapted log; original UNF startup passes in all three session kinds of the SDK 0.9.35 matrix. Task 8 remains the gate for the running post-SDK36 checks; the historical results are not claimed to be new runs.

Verification update (2026-10-21): the SDK 0.9.36 workspace gate at `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk36-final-checks/` passed all eight checks, including API-reference freshness, facade consumer and core dependency comparison (4006 tests, zero failures, one ignored dump). Original UNF thin-client and web-client initialization passed in `docs/research/fixtures/on-demand-session-matrix-sdk36-1021/`, with approved default protected export stubs. These receipts do not prove live document number allocation or execution of protected bodies; unresolved tasks remain open.

Scope decision (2026-10-21): the user approved keeping actual live-infobase document-number allocation as separate future work. Tasks 39 and 51 above record that decision, not a completed allocator. The measured handler/dispatch and explicit unsupported-adapter boundary remain in scope. Existing requirements are unchanged: they expressly allow an unsupported adapter to fail and forbid fabricated numbers or claiming a test adapter as live numbering.

Acquired reference closure (2026-10-21): tasks 3/13/20 now include native SDK owner-resource checks and the actual public DatabaseConfigurationSource projection, not only a manually built reference map. The retained SDK36 control read all56 correct owner/type GUIDs but failed projection (0 types); the new counting-source regression reproduced that failure, and SDK37 header identity plus owner-index wiring restored all56 types in native order while leaving unrelated catalog details unread. The live SDK37 UNF control passed in7.28 seconds: `docs/research/fixtures/document-reference-owner-acquisition-sdk37-1021/result.json`. Native primitive/default, enumeration filling and 26-observation basis reference replay tests passed in the complete SDK37 workspace gate (4007 passed,0 failed,1 ignored): `docs/research/fixtures/deferred-prepared-module-caches-1021/sdk37-final-checks/test-summary.json`. Live numbering remains explicitly deferred. Archive remains pending the final current-build original startup review.

Final current-build review (2026-10-21): original UNF thin-client initialization passed on SDK37 (`docs/research/fixtures/on-demand-session-matrix-sdk37-1021/unf-thin-client/cli-unf-on-demand-preparation-1020-result.json`), after the56-type acquisition/projection fix. Complete workspace gate4007/0/1 and all eight checks are green, API reference is current, facade consumer builds and core normal dependencies are unchanged. Native document creation/filling/reference/handler tests remain separate semantic evidence. Actual infobase number allocation is user-deferred future work (`docs/plans/document-live-number-allocation.md`); handler and test-adapter checks do not prove it, and the production adapter retains explicit unsupported failure. Protected export bodies remain default stubs, and the remaining SDK37 matrix stays required for the overarching goal.
