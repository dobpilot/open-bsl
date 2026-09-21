# Design

`SessionFactory` owns a compiled open-bsl engine, one `ConfigurationCodeSnapshot`, and the immutable ORM metadata catalog produced at acquisition time. It performs compatibility checks once: the code and metadata identify the same configuration, and every linked manager module names an object present in the metadata catalog. Extra database metadata is allowed. The factory does not re-read or hash `Config` for every session; after a configuration update the host builds a new factory.

Each `Session` owns one open-bsl `State`, session module instances, session parameters, execution modes, a frozen principal description, and one live database connection. Database statements use the existing bounded read-only `READ COMMITTED` operations of `open-sdbl-db`; no transaction spans the lifetime of the BSL session. A failed connection, compatibility check, or startup handler returns an error and no usable partial session.

`ТекущийРежимЗапуска()` is derived from the explicit session kind. Web-client and thin-client sessions execute managed application code and return `РежимЗапускаКлиентскогоПриложения.УправляемоеПриложение`. A background-job session returns `Неопределено`; this result was measured inside a real background job on platform 8.3.27.2342 by `measure-session-current-run-mode.bsl`.

The host supplies one non-empty platform version when it constructs `SessionFactory`. The factory stores the string once and every session exposes it through a newly constructed `СистемнаяИнформация` value. The runtime does not substitute the open-bsl crate version or query the database on every session open.

The principal is selected from `v8users` by case-insensitive name or GUID. Only a user with at least one enabled authentication method is selectable. With one selectable user the CLI may choose it automatically; with several it requires `--principal`; with no `v8users` rows it constructs the platform empty user and applies the configuration's main roles. The description is frozen at session creation. Temporary authentication locks are outside this testing host.

The required upstream `open-sdbl` user model must decode every authentication method supported by the selected platform family and expose whether at least one method is enabled. `AdmRole` remains separate from object roles and does not grant unrestricted application data access.

`bsl-orm` owns configuration metadata and data-facing platform objects; it does not load BSL source. The initial surface is the one recorded in `docs/research/session-module-runtime-surface-2026-09-21.md`. It includes the global `РольДоступна`/`IsInRole` function, accepting a role name or metadata role object and answering from the frozen principal's roles. Unknown roles and invalid argument types are errors. Unsupported platform members fail explicitly with their BSL name.

`--config-db` is an additive CLI modifier. Without it, script execution, commands, debugging, optimization, bytecode operation, and the REPL behave exactly as before. With it, the CLI loads the database configuration, selects the principal, starts a configuration session, and executes the supplied test script in that context. `--principal` is invalid without `--config-db`.
