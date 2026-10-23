# Tasks

- [x] 1. Add the required `open-sdbl` authentication-method model and test filtering of selectable `v8users` principals.
- [x] 2. Add the initial `bsl-orm` configuration metadata and role objects, including `РольДоступна`.
- [x] 3. Add `bsl-session::SessionFactory` and validate code/metadata compatibility once at factory construction.
- [x] 3a. Build the startup catalog from server-available modules only, retaining duplicate-global rejection inside that context.
- [x] 3b. Compile selected configuration modules with the standard component set obtained only through the `open-bsl` facade.
- [x] 3c. Add facade-owned rooted catalog compilation through the existing semantic links, and use the session module as the sole startup root.
- [x] 3d. Expose a host-only call to the session-owned catalog handler through the facade, retaining export-only BSL resolution and ordinary component calls.
- [x] 4. Add isolated `Session` creation with one connection, one open-bsl `State`, a frozen principal, and failure-atomic startup.
- [x] 5. Execute `УстановкаПараметровСеанса(Неопределено)` for web-client, thin-client, and background-job session kinds, including selective reentry of a non-exported platform handler. Verified by `private_startup_handler_reenters_for_each_supported_session_kind` on controlled code; original configuration startup remains task 10.
- [x] 6. Expose and test the measured `ТекущийРежимЗапуска()` result for every supported session kind.
- [x] 7. Expose the host-supplied platform version through `СистемнаяИнформация` during startup.
- [x] 8. Add the complete configuration-extension snapshot, source and scope enums, manager methods, extension objects, and session application issues to `bsl-orm`.
- [x] 8a. Add the measured configuration compatibility mode to the immutable ORM catalog and metadata properties without declaring a new bare global.
- [x] 8b. Build the ORM catalog and principal candidates from one `open-sdbl-db` acquisition in `bsl-config-db`, preserving the shared configuration identity and provider boundaries.
- [x] 8c. Provide one factory-input adapter path that parses the configuration code once and reuses that snapshot while projecting the ORM catalog.
- [x] 8d. Select factory Config resources before retention through the upstream selected-acquisition API, using the same required-name rule as the existing factory adapter.
- [x] 9. Add the optional `bsl-cli --config-db` and `--principal` path while keeping existing CLI tests byte-for-byte unchanged where applicable.
- [x] 10. Exercise full startup for web-client, thin-client, and background-job sessions against the `demo`, `buh`, and `unf` configurations and preserve non-confidential evidence. All nine cases and selected initialization in all three databases pass on the frozen SDK0.9.20 review stack; after authorized integration, the actual ROOT CLI also passes DEMO. Receipts are indexed in `docs/plans/configured-session-startup-matrix.json`.
- [x] 11. Document every public Rust API in Russian, regenerate the BSL API reference, and build `open-bsl-consumer` using facade names only.
- [x] 12. Run the complete workspace checks, verify the `bsl-rt` and `bsl-vm` normal dependency trees, validate with `openspec validate start-configuration-session --strict`, and archive the change after every gate passes.
- [x] 12a. Complete the workspace regression gate on 2026-10-08 after explicitly approved test updates: 3313 passed, 0 failed, 1 ignored; fmt, all-target Clippy, warning-free rustdoc and consumer build passed. Core/VM normal dependency trees match the previous baseline. Strict validation passes. Original configuration startup and this change's archive remain outstanding; the regression gate does not prove them. Logs: `/tmp/open-bsl-workspace-tests-approved-1008.log`, `/tmp/open-bsl-workspace-clippy-approved-1008.log`, `/tmp/open-bsl-workspace-doc-approved-1008.log`, `/tmp/open-bsl-consumer-approved-1008.log`.

Final acceptance on 2026-10-16: ROOT3794passed,0failed,1ignored; SDK0.9.20 1065passed,0failed,13ignored. Formatting, all-target Clippy, warning-free rustdoc, strict validation, facade consumer and unchanged RT/VM normal dependencies pass. All nine original startup contexts and all three selected-initialization cases pass on the immutable SDK0.9.20 review stack; actual ROOT CLI also passes DEMO after integration. Requirement evidence: `docs/research/fixtures/approved-sdk-production-integration-1016/start-change-acceptance-audit.json`. Broader runtime follow-ups remain in the separate `complete-configuration-session-runtime` change and are not certified by this archive.
