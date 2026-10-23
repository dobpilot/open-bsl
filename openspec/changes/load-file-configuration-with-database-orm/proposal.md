# Why

Configuration startup currently obtains both executable sources and ORM facts
from the information base. Developers must be able to execute local Configurator
XML or EDT sources against an existing database, including local form attributes,
templates and other configuration descriptions, without loading the configuration
into 1C first or replacing the database ORM with a second implementation.

The neutral ConfigurationSource and source-backed SessionFactory already exist.
The XML and EDT crates are placeholders, so source selection cannot yet be used.

# What Changes

- Implement bsl-config-xml and bsl-config-edt as readers of native directory
  exports/projects, using the existing neutral module source contract.
- Extend the neutral source surface to address source-owned metadata descriptions,
  form definitions/attributes and template resources as well as BSL modules.
- Compose exactly one selected file-source bundle with database-acquired ORM,
  schema mappings, data access, users, authorization and separation facts.
- Bind source and database objects by native identity and kind; allow changed
  local code and nonpersistent descriptions without requiring equal source hashes.
- Add --config-xml <directory> and --config-edt <directory> as mutually exclusive
  modifiers of --config-db. Add explicit same-format extension-directory mappings
  through repeatable --config-extension <uuid>=<directory> when needed.
- Preserve on-demand reads, reusable factory caches, existing resolver/VM behavior,
  session isolation and the existing database-only and standalone CLI paths.
- Replace the placeholder-only requirement and scope the existing single-database
  acquisition requirement to database-only code mode.

# Scope

This change includes actual readers and their integration, not more placeholder
crates. Source ownership covers module execution flags, configuration/object
source descriptions, form attributes/types/bindings, commands and handlers,
common and object-owned templates and their content. Database storage facts and
live session state remain database-owned. Runtime-facing source descriptions
must be connected to existing consumers; a byte reader unused by the runtime is
not sufficient. Unsupported payload kinds fail explicitly.

A new interpreter, configuration upload, database schema migration, GUI rendering,
a new query engine, file-only ORM, automatic source mixing, hot reload and new
protected-module decompilation are outside scope. Existing XML/MXL/template
parsers and runtime types are reused. No new external dependency is authorized.

# Impact

Affected: bsl-config, bsl-config-xml, bsl-config-edt, open-bsl facade, bsl-session,
bsl-orm integration, bsl-config-db acquisition composition, bsl-cli and
open-bsl-consumer. The runtime and VM acquire no filesystem or SQL dependency.
Public Rust APIs and CLI behavior require this change before implementation.
The deferred decompile-native-module-images change remains untouched.
