# Tasks

- [x] 1. Specify and implement the facade-only component-to-module call bridge using existing State module instances.
- [x] 2. Test nested calls, `ЭтотОбъект`, by-reference parameters, errors, and separation between States.
- [x] 3. Link ORM manager objects to their manager modules and implement platform-first, export-second dispatch.
- [x] 4. Implement and test the measured unset, nested initialization, cycle, retry, clear, and non-rollback session-parameter behavior.
- [ ] 5. Connect query, manager, register, and constant reads to the current principal, session parameters, rights, and RLS.
- [x] 5a. Implement the measured linear `Запрос`, result, selection, and unload surface through the Session-owned `DataAccess` boundary.
- [x] 5b. Expose the host-supplied information-base connection string through the current Session and fail explicitly when it is absent.
- [x] 5c. Implement the measured `ХранилищеЗначения` and `СжатиеДанных` constructors and detached `Получить()` semantics through the existing serialized-value graph.
- [x] 5d. Implement the measured current information-base session and its foreground/background job association.
- [ ] 5e. Add only the remaining metadata, manager, register, constant, connection, security-mode, and diagnostic members reached by the recorded `demo`, `buh`, and `unf` startup scenarios, measuring platform semantics before implementation.
- [x] 5f. Expose the measured information-base connection number from the concrete foreground or background Session connection.
- [ ] 6. Reject missing rights and unsupported runtime members explicitly instead of returning permissive placeholders.
- [x] 7. Create background-job sessions with an explicit principal, copied parent parameters, a separate connection, and isolated module state.
- [ ] 8. Run the full-start and selected-parameter `demo`, `buh`, and `unf` scenarios without changing existing regression expectations.
- [ ] 9. Run formatting, Clippy, workspace tests, rustdoc, API-reference generation, facade-consumer compilation, and dependency-tree checks.
- [ ] 10. Validate with `openspec validate complete-configuration-session-runtime --strict` and archive the change only after every runtime and workspace gate passes.
