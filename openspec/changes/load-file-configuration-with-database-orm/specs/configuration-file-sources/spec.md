## ADDED Requirements

### Requirement: Native Configurator XML source reader
The bsl-config-xml adapter SHALL load a native Configurator directory export as
one indexed source bundle containing logical module declarations, original BSL,
source-owned metadata descriptions, forms and templates. It SHALL obtain
identities, ownership and execution attributes from native descriptions rather
than infer a configuration from unrelated BSL filenames. It SHALL preserve
source spelling and supported declared encodings. Supported native format
versions SHALL be documented and covered by retained native fixtures.

#### Scenario: Configurator export contains several module kinds
- **WHEN** a supported export declares a session module, common module, object module, manager module and form module
- **THEN** each is indexed with its native owner, module kind and applicable execution attributes
- **AND** a requested module returns its original BSL without source rewriting

#### Scenario: Missing declared module file
- **WHEN** a native description declares a module whose required source file is absent
- **THEN** acquisition fails with its format, path and logical module identity
- **AND** the loader does not read a database copy or synthesize a successful empty module

#### Scenario: Unknown native representation
- **WHEN** a required description or reached module uses an unsupported native representation, including unrecognized binary-only code
- **THEN** loading fails explicitly with the affected resource
- **AND** it does not claim complete source recovery

### Requirement: Native EDT source reader
The bsl-config-edt adapter SHALL load a native EDT configuration project as the
same neutral bundle contract as the XML reader. Native project descriptions
SHALL determine object identities, ownership, module execution flags and artifact
references. The adapter SHALL NOT require a Configurator conversion step or an
EDT process at runtime. Supported project versions SHALL be documented and
covered by retained artifacts produced by real EDT tooling.

#### Scenario: Equivalent EDT and Configurator fixtures
- **GIVEN** native EDT and Configurator artifacts describe the same logical configuration
- **WHEN** either reader indexes and acquires the same supported module or artifact
- **THEN** it exposes equivalent identities, ownership, types and effective content through the neutral contract
- **AND** format-specific paths are not used as logical object identities

#### Scenario: EDT project used without an IDE process
- **WHEN** the CLI loads a supported EDT project directory with no running EDT installation
- **THEN** the reader acquires its declared code and artifacts directly from that directory

#### Scenario: Invalid EDT reference
- **WHEN** a reached native project reference cannot be resolved within the explicitly selected bundle
- **THEN** acquisition fails with the referring object and unresolved resource
- **AND** another format or the DB code store is not consulted

### Requirement: Source-owned descriptions forms and templates
In mixed mode the selected file bundle SHALL supply source-level configuration
and object descriptions, form attribute identities/names/types, declared main
attributes, nested attributes, relevant data bindings, command/handler descriptions,
form module links, common templates and object-owned templates with their declared
kind and complete content. The system SHALL preserve missing, explicitly empty
and unsupported states distinctly. Source-owned nonpersistent descriptions SHALL
NOT require a SQL column solely because they are metadata. Existing runtime
metadata/artifact consumers SHALL obtain these facets from the selected bundle;
this SHALL NOT introduce another global metadata tree or a GUI/event-loop engine.

For template representations already supported by runtime decoders, existing
applicable template lookup entry points SHALL materialize the actual source
payload through those decoders. Unsupported requested representations SHALL
fail explicitly rather than return fabricated successful empty values.

#### Scenario: Changed local form attribute
- **GIVEN** a file form adds a nonpersistent attribute or changes its declared type relative to the DB copy
- **WHEN** that form description is requested in mixed mode
- **THEN** the selected file name, type and ownership are exposed
- **AND** the DB form payload is not used and no SQL schema modification is attempted

#### Scenario: Changed local template content
- **GIVEN** a supported file template contains content different from its DB counterpart
- **WHEN** its existing template consumer obtains the template in mixed mode
- **THEN** it receives the actual file content and declared kind through the existing value decoder
- **AND** no database payload is substituted

#### Scenario: Required unsupported template kind
- **WHEN** a requested template kind has no supported runtime materializer
- **THEN** the request fails explicitly with owner, template and kind
- **AND** it does not return an empty spreadsheet, text document or binary value as success

#### Scenario: Other source metadata property
- **GIVEN** a supported source-owned property of a configuration object differs in the file description
- **WHEN** the existing metadata consumer requests that property
- **THEN** it exposes the file value
- **AND** database storage mappings and effective principal rights remain unchanged

### Requirement: Native identity and database storage compatibility
Mixed-source composition SHALL verify the native root configuration object
identity and the identity/kind of persistent owners linked from the file bundle.
It SHALL distinguish the root metadata UUID from an internal configuration
storage namespace. A DB namespace SHALL NOT be assigned to unverified unrelated
sources merely to satisfy the existing factory identity check. Compatible local
code and nonpersistent artifact edits SHALL be accepted without comparing all
source bytes with the database. A reached database-bound field/type incompatible
with the acquired ORM model SHALL fail explicitly without changing the schema
or silently replacing ORM facts with file descriptions.

#### Scenario: Different configuration root
- **WHEN** the selected directory belongs to another native root configuration UUID
- **THEN** construction fails before a usable session is returned
- **AND** matching display names do not bypass the check

#### Scenario: Distinct internal namespace and native root UUID
- **GIVEN** the DB acquisition exposes different storage namespace and native root object identities
- **WHEN** a matching file bundle is bound
- **THEN** native identity verification succeeds without incorrectly comparing the file root UUID with the storage namespace
- **AND** the existing factory receives a compatible verified logical namespace

#### Scenario: Local method edited
- **WHEN** a module body is edited in a matching file configuration without changing its persistent owner
- **THEN** composition accepts that body without a whole-source fingerprint equality requirement

#### Scenario: Missing persistent field
- **WHEN** reached file metadata requires a persistent field absent from the acquired DB object
- **THEN** the request fails with owner and field identity before using an invented SQL mapping
- **AND** the missing field is not treated as an ordinary nonpersistent form attribute

### Requirement: Single bundle and explicit extension sources
All module bodies and source-owned descriptions/artifacts SHALL come from one
selected format bundle. A bundle MAY explicitly include same-format main and
extension directories. Database extension applicability and platform order SHALL
select the extensions to apply, while their source content SHALL come only from
those directories. The existing effective-module contract SHALL produce one body
per logical module. No per-module or per-artifact fallback to DB, another format
or an unselected directory is permitted.

#### Scenario: Applicable extension supplied by files
- **GIVEN** each applicable extension has an explicit matching-identity directory in the selected format
- **WHEN** its supported operations are applied in database-established order
- **THEN** the effective local graph and source artifacts include the selected extension changes
- **AND** no DB BSL body enters the graph

#### Scenario: Applicable extension lacks file source
- **WHEN** an applicable extension has no supplied matching file source
- **THEN** startup fails identifying that extension
- **AND** neither its DB code nor a base-only graph is presented as complete

#### Scenario: Inactive extension has source files
- **WHEN** files are supplied for an extension that is not applicable under the existing DB/session policy
- **THEN** its code and artifacts do not modify the effective bundle

#### Scenario: Unsupported extension operation
- **WHEN** a reached extension operation cannot be reduced by the existing effective-module mechanism
- **THEN** loading fails explicitly rather than inventing precedence or interception behavior

### Requirement: On-demand immutable acquisition
File source indexing SHALL discover native identity, ownership and module/artifact
membership without eagerly decoding every body, form or template. Acquisition
SHALL retain the existing declaration-discovery requirements and first-access
compilation behavior. Dynamic string lookup SHALL use the same selected bundle.
Acquired immutable resources SHALL be cached for reuse by the factory and its
sessions; mutable session state SHALL NOT enter those caches. Reopening a session
SHALL NOT hash the full directory or Config again. A source bundle is required
to remain stable for a factory lifetime; a detected change during acquisition
SHALL fail explicitly without silently replacing cached resources. New source
generations require a new factory; automatic hot reload is not provided.

#### Scenario: Unused large form or template
- **WHEN** startup never requests an indexed form/template payload
- **THEN** its content is not decoded or retained merely because its owner exists
- **AND** its declared membership remains discoverable

#### Scenario: Repeated dynamic access
- **WHEN** runtime string-based access reaches the same file module or artifact twice
- **THEN** the existing selected-source machinery acquires and caches that resource
- **AND** the repeated access reuses the immutable result without DB code fallback

#### Scenario: Second session of one factory
- **WHEN** another session opens on an unchanged factory
- **THEN** it reuses the indexed bundle and acquired source cache
- **AND** opening it does not rescan/hash every file or reuse another session's mutable resources

#### Scenario: Detected edit during resource acquisition
- **WHEN** a file differs from its indexed generation under the adapter's documented change checks
- **THEN** acquisition reports a changed-source error requiring factory reconstruction
- **AND** already cached immutable resources are not silently updated

### Requirement: CLI file selection with a database
Configured script mode SHALL accept --config-xml <directory> or --config-edt
<directory> in separated and equals forms, with exactly one of these selectors
and an existing --config-db invocation. A repeatable --config-extension
<extension-uuid>=<directory> option SHALL bind distinct extension identities to
directories in that same file format and SHALL require a file selector.
Empty/repeated file selectors, both formats together, empty paths, duplicate
extension UUID bindings and use without DB mode SHALL fail before connecting.
Unknown/mismatched extension identities SHALL fail before session startup.
Neither selector SHALL change existing DB credential/principal/session-kind
options. Option text after the script path SHALL remain a script argument.

#### Scenario: XML with database ORM
- **WHEN** the command selects --config-db and --config-xml with valid existing connection arguments
- **THEN** the XML bundle supplies the code and source artifacts while DB mode supplies ORM and session resources

#### Scenario: EDT with database ORM
- **WHEN** the command selects --config-db and --config-edt with valid existing connection arguments
- **THEN** the EDT bundle supplies the code and source artifacts while DB mode supplies ORM and session resources

#### Scenario: Invalid selectors
- **WHEN** both selectors occur, one is repeated/empty, or file mode appears without --config-db
- **THEN** CLI parsing fails before opening a DB connection

#### Scenario: Extension directory bindings
- **WHEN** distinct extension UUID/directory mappings are supplied in file mode
- **THEN** they are passed explicitly to that format's reader
- **AND** a duplicate binding or a binding without file mode is rejected

#### Scenario: No source override
- **WHEN** neither new selector is supplied
- **THEN** existing database-only startup or standalone script behavior is unchanged

#### Scenario: Option-like script argument
- **WHEN** --config-xml or --config-edt occurs after the script path
- **THEN** it is passed to the script rather than switching configuration sources

### Requirement: Facade-only file source consumption
Public file-source reader, descriptor, artifact and composition types required
by a host SHALL be accessible through the open-bsl facade. File adapters SHALL
NOT introduce SQL or DB adapter dependencies, and the runtime/VM SHALL NOT gain
filesystem/XML/EDT dependencies. The implementation SHALL reuse the existing
interpreter and native payload/XML decoders and SHALL add no new external crate.

#### Scenario: Consumer builds both reader modes
- **WHEN** open-bsl-consumer constructs supported XML and EDT source inputs and accesses their neutral metadata/artifact surface
- **THEN** it compiles using facade-exposed names without naming implementation crates

#### Scenario: Dependency boundaries
- **WHEN** normal dependency trees are checked after implementation
- **THEN** bsl-rt and bsl-vm retain their previous normal dependencies
- **AND** neither file adapter depends on open-sdbl, bsl-config-db, bsl-orm or bsl-session

### Requirement: Native scalar and reference-family type parity
File-declared UUID scalar types and generic reference families SHALL resolve to
the same existing runtime type descriptors as their database ORM declarations.
A nonempty chart-of-calculation-types reference family SHALL include the
registered reference descriptor of every corresponding database owner. The file
adapter SHALL NOT fabricate a second set of descriptors or silently omit members.

#### Scenario: Native UUID session parameter
- **GIVEN** paired Config, Configurator XML and EDT descriptions declare a UUID session parameter
- **WHEN** the file description is validated or its type is requested
- **THEN** the database declaration and file declaration both resolve to the existing UUID runtime type

#### Scenario: Nonempty calculation-type reference family
- **GIVEN** the database catalog contains calculation-type charts and a file declaration uses their generic reference family
- **WHEN** the declaration is expanded
- **THEN** every corresponding registered ORM reference descriptor is included once
- **AND** Russian and English concrete reference type names resolve to that same descriptor

### Requirement: Existing protected-module policy for native file images
For a recognized native binary-only module, mixed file mode SHALL apply the
same explicit protected-module policy as database mode to bytes from the
selected directory. The default SHALL expose export signatures with the
previously agreed stub bodies; explicit body recovery SHALL reuse the existing
strict recovery mechanism. Neither mode SHALL claim recovered original bodies
when only export declarations are available. Malformed or unsupported images
SHALL fail explicitly without database source fallback.

#### Scenario: Native Module.bin reached during initialization
- **GIVEN** a selected XML or EDT directory supplies a recognized native module image
- **WHEN** session initialization reaches that module with the default protected-module policy
- **THEN** the existing export-interface mechanism handles the selected file bytes
- **AND** no corresponding module code is read from the database

#### Scenario: Unknown binary image
- **WHEN** a selected module image cannot be decoded by the existing native image mechanism
- **THEN** loading fails explicitly for that module instead of publishing an empty successful module
