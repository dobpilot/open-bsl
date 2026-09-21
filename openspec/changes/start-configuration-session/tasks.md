# Tasks

- [ ] 1. Add the required `open-sdbl` authentication-method model and test filtering of selectable `v8users` principals.
- [x] 2. Add the initial `bsl-orm` configuration metadata and role objects, including `РольДоступна`.
- [x] 3. Add `bsl-session::SessionFactory` and validate code/metadata compatibility once at factory construction.
- [x] 4. Add isolated `Session` creation with one connection, one open-bsl `State`, a frozen principal, and failure-atomic startup.
- [x] 5. Execute `УстановкаПараметровСеанса(Неопределено)` for web-client, thin-client, and background-job session kinds.
- [x] 6. Expose and test the measured `ТекущийРежимЗапуска()` result for every supported session kind.
- [x] 7. Expose the host-supplied platform version through `СистемнаяИнформация` during startup.
- [x] 8. Add the complete configuration-extension snapshot, source and scope enums, manager methods, extension objects, and session application issues to `bsl-orm`.
- [ ] 8a. Add the measured configuration compatibility mode to the immutable ORM catalog and metadata properties without declaring a new bare global.
- [ ] 9. Add the optional `bsl-cli --config-db` and `--principal` path while keeping existing CLI tests byte-for-byte unchanged where applicable.
- [ ] 10. Exercise full startup for web-client, thin-client, and background-job sessions against the `demo`, `buh`, and `unf` configurations and preserve non-confidential evidence.
- [ ] 11. Document every public Rust API in Russian, regenerate the BSL API reference, and build `open-bsl-consumer` using facade names only.
- [ ] 12. Run the complete workspace checks, verify the `bsl-rt` and `bsl-vm` normal dependency trees, validate with `openspec validate start-configuration-session --strict`, and archive the change after every gate passes.
