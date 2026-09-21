# Tasks

- [x] 1. Specify and implement the facade-only component-to-module call bridge using existing State module instances.
- [x] 2. Test nested calls, `ЭтотОбъект`, by-reference parameters, errors, and separation between States.
- [x] 3. Link ORM manager objects to their manager modules and implement platform-first, export-second dispatch.
- [x] 4. Implement and test the measured unset, nested initialization, cycle, retry, clear, and non-rollback session-parameter behavior.
- [ ] 5. Connect query, manager, register, and constant reads to the current principal, session parameters, rights, and RLS.
- [x] 5a. Implement the measured linear `Запрос`, result, selection, and unload surface through the Session-owned `DataAccess` boundary.
- [x] 5b. Expose the host-supplied information-base connection string through the current Session and fail explicitly when it is absent.
- [ ] 5c. Implement the measured `ХранилищеЗначения` and `СжатиеДанных` constructors and detached `Получить()` semantics through the existing serialized-value graph.
- [ ] 6. Reject missing rights and unsupported runtime members explicitly instead of returning permissive placeholders.
- [x] 7. Create background-job sessions with an explicit principal, copied parent parameters, a separate connection, and isolated module state.
- [ ] 8. Run the `demo`, `buh`, and `unf` selected-parameter scenarios and the complete workspace verification gates without changing existing tests.
