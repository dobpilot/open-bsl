# Tasks

- [ ] 1. Add the required `open-sdbl` authentication-method model and test filtering of selectable `v8users` principals.
- [x] 2. Add `bsl-orm` metadata and role objects for the measured full-start surface, including `РольДоступна`.
- [x] 3. Add `bsl-session::SessionFactory` and validate code/metadata compatibility once at factory construction.
- [x] 4. Add isolated `Session` creation with one connection, one open-bsl `State`, a frozen principal, and failure-atomic startup.
- [x] 5. Execute `УстановкаПараметровСеанса(Неопределено)` for web-client, thin-client, and background-job session kinds.
- [x] 6. Expose and test the measured `ТекущийРежимЗапуска()` result for every supported session kind.
- [ ] 7. Expose the host-supplied platform version through `СистемнаяИнформация` during startup.
- [ ] 8. Add the optional `bsl-cli --config-db` and `--principal` path while keeping existing CLI tests byte-for-byte unchanged where applicable.
- [ ] 9. Exercise full startup against the `demo`, `buh`, and `unf` configurations and preserve the evidence outside confidential fixtures.
- [ ] 10. Document every public Rust API in Russian, regenerate the BSL API reference, and build `open-bsl-consumer`.
