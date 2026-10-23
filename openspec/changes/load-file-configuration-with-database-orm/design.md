# Context

ConfigurationSource currently exposes ConfigurationIndex, module_source and
module_declarations. ConfigurationCodeSnapshot represents one effective module
graph. SessionFactory::new_with_source already consumes that contract and a
separate MetadataCatalog. Both file crates currently expose no loading API.
DatabaseConfigurationSource combines code and ORM acquisition today.
Several existing GetTemplate entry points currently fail as unsupported; therefore
reading template bytes alone must not be advertised as successful BSL integration.

This is one coherent change: both file adapters supply the same source facet to
the existing database-backed execution path. They do not implement independent
BSL engines or independent ORM catalogs.

# Source ownership

| Facet | Authority in mixed mode |
|---|---|
| SQL connections, storage/table/column mappings, persistent object facts | Database through open-sdbl and bsl-config-db |
| Principals, assigned roles, effective rights/RLS, separators, live values and transactions | Database and existing session services |
| Module graph, BSL text, module execution flags and source imports | Selected XML or EDT bundle |
| Source-level configuration/object descriptions and nonpersistent members | Selected XML or EDT bundle |
| Form definitions, attributes, declared types, data paths, commands and handlers | Selected XML or EDT bundle |
| Common/object templates, declared template kind and resource content | Selected XML or EDT bundle |
| Extension applicability, platform order and database runtime facts | Existing database/session acquisition |
| Code and source artifacts of applied extensions | Explicit directories in the same selected file format |

Keep one existing global Метаданные and existing managers. Supply source-owned
facets to their current metadata/artifact consumers instead of registering a
second competing global. File descriptors may enrich forms, templates and
nonpersistent configuration objects. They must not overwrite physical mappings,
create database columns or grant permissions. Local role descriptions are not
an authentication/authorization input. A database-bound field/type disagreement
is an explicit compatibility failure, not a silent choice of one description.

# Neutral API and crate boundaries

Retain ConfigurationSource for declarations and module bodies. Add the minimum
companion artifact/descriptor contract in bsl-config, with stable owner/member
identities, artifact kind, declared representation and origin diagnostics. It
must distinguish an absent optional artifact, a known empty artifact, unsupported
representation and an I/O/parse failure. Do not encode native paths as logical
module or object identities. Exact Rust signatures are finalized with focused
API tests during implementation; they must not expose EDT/XML parser internals.

A source bundle groups one module source with its descriptions and artifacts;
consumers cannot independently substitute a different provider for one module,
form or template. All new consumer-facing types/readers are nameable through
open-bsl, with optional facade features if necessary. Keep the empty-dependency
bsl-config contract neutral; parser and filesystem work belongs to the adapters.

Reuse bsl-xml::core or lift its existing reusable parser surface; do not write a
second XML parser. Both formats use the same existing XML machinery and shared
neutral projection helpers where they actually coincide. Reuse existing text,
spreadsheet/MXL, binary and other supported payload decoders through their
existing component/facade boundary. The file adapters must not depend on
bsl-session, bsl-orm, bsl-config-db, open-sdbl or a SQL provider. Avoid a facade
cycle: adapters depend on neutral contracts and existing lower-level parser
crates; the facade reexports them, never the other way around.

bsl-session composes the selected bundle with its DB-backed catalog/resources;
bsl-orm consumes the neutral read-only source facet through the facade. Per-session
connections and mutable objects are never stored in the shared file-source cache.
bsl-cli selects readers and composes inputs; it parses neither native file format
nor Config database storage. bsl-rt and bsl-vm normal dependencies are unchanged.

# Native formats and evidence

Discover native object/module ownership and execution flags from actual format
metadata rather than a recursive search for arbitrary .bsl files. Preserve Unicode
names, source text and declared encodings. A valid optional module not declared
by the native descriptor is different from a declared resource missing its file.
Malformed/unknown required structures fail with format, relative path and logical
owner/member identity. Unknown optional properties may remain opaque only when
that preserves their declared value and no runtime consumer needs interpretation.

Before implementing each mapping, retain paired native artifacts for the same
configuration: Configurator directory export and EDT project, plus database UUIDs
and platform observations for any observable BSL behavior. Existing Configurator
fixtures can be reused; EDT fixtures must come from a real EDT export/project,
not from guessing .mdo shapes. Document the supported format versions from this
evidence. Unmeasured runtime semantics follow the repository marker/registry/
measurement discipline; this proposal invents no platform defaults.

Form loading includes attribute identity/name/type, main attribute where declared,
child collections, relevant data bindings, command/handler descriptions and form
module linkage. It does not implement a visual client or event loop. Templates
include owner, identity/name, kind and complete content, not just filenames.
For kinds already supported by existing runtime decoders, wire template lookup
and materialization to real values. Unsupported kinds remain explicit errors;
no empty SpreadsheetDocument or empty form is substituted for unreadable input.

# Binding source to database

Check the native root configuration object UUID and the UUID/kind of linked
persistent owners. The database storage namespace/configuration identity and the
root metadata object's UUID are not assumed identical: the existing specs
explicitly distinguish them. Verify native identity before assigning the shared
factory compatibility namespace. Do not simply stamp a DB identity onto an
unrelated directory to make SessionFactory accept it.

Discover identity/member indexes once. Validate detailed storage-relevant links
when they are reached, preserving existing on-demand acquisition. A file-only
nonpersistent member such as a form attribute may be valid without a SQL column;
a file-defined persistent field absent from the acquired database model is not.
Allow edited BSL, form descriptions and templates of the same configuration;
do not compare whole-tree hashes or require source equality with Config.
A newly added code-only common module does not create a persistent database object.

# CLI

Examples (existing connection, principal and platform arguments remain required
as today):

    bsl-cli --config-db postgres <existing DB arguments> --config-xml ./src script.bsl
    bsl-cli --config-db postgres <existing DB arguments> --config-edt ./project script.bsl

--config-xml and --config-edt accept separated and equals forms and are mutually
exclusive. They are invalid without --config-db, when repeated, or with an empty
path. Options after the script path remain script arguments. With neither option,
all existing standalone and database-only behavior is preserved.

For native exports that keep extensions separately, --config-extension
<extension-uuid>=<directory> is repeatable with distinct UUIDs, requires one file
mode, and uses that mode's format. Main and extension roots must never be guessed
from display names. Existing database extension applicability/order chooses what
to apply. An applicable extension lacking its selected file source fails before
startup; no DB BSL is spliced in. Inactive extensions do not contribute code.
Unsupported interception/merge operations fail under the existing effective-module
contract; this change does not invent extension semantics. The bundle still
presents exactly one effective body per logical module.

Reject malformed/incompatible selector combinations before opening SQL connections.
Perform native identity binding after the necessary DB discovery but before
returning a usable session. Existing DB credential handling is unchanged.
The protected-decompilation flag must not cause file mode to read DB bodies.
Recognized native images use the existing export-interface or explicit strict
recovery policy from the selected bundle, as detailed below. Unknown binary
representations fail; a new decompiler is out of scope.

# On-demand reads and lifetime

Index the selected native directory graph and the minimal metadata needed for
selection, then use existing first-access compilation. Global declarations may
still require the existing selected declaration scan; do not promise that no
unreached text is ever inspected. Do not eagerly decode all form/template payloads
or acquire DB code resources. Database descriptor reads needed for ORM/identity/
rights are allowed even when the same descriptor also carries code-location fields.

Use immutable shared cached results for each acquired body/description/payload,
including dynamic string lookup through the same selected bundle. Repeated
Sessions of one factory reuse those results and do not hash the whole tree or
Config again. Host edits require creating a new factory; automatic hot reload is
not added. Check indexed file identity/size/timestamps around first acquisition
and fail explicitly on a detected change. This detects ordinary edits, not an
atomic filesystem snapshot or adversarial timestamp-preserving mutation. The host
must keep its source bundle stable for the factory lifetime. Cached values stay
unchanged even after an edit is detected elsewhere.

# Verification and scope limits

Start with paired native fixtures and failing selection/routing tests, including
an edited local method whose result differs from the DB copy, a local form
attribute and a local template payload. Counting providers must show no DB code
fallback and no unused payload reads. Verify repeated sessions, dynamic calls,
owner mismatches, extension order, explicit errors and existing rights/separation.
Exercise the same semantics through XML and EDT and facade-only consumer tests.
Live acceptance on demo, buh and unf is read-only; heavy bsl-cli processes are
strictly serial under the existing exclusive lock and memory guard.

No new external dependency, new interpreter, schema write, automatic directory
conversion, source watcher, general UI engine or full protected decompiler is
required. If an existing parser cannot cover a measured native representation,
record the concrete gap before proposing a separate dependency or SDK change.

## Native EDT import evidence

EDT 2026.1.3+25 imports the retained Configurator fixture and the full BUH dump. The importer omits `templateType` for spreadsheet templates, uses short child-subsystem references and normalizes BSL line endings to LF. Paired native fixtures and reader tests cover these mappings. Native catalogs can declare templates or common modules without payload files. Indexing retains those declarations; acquiring missing content produces an explicit source error, never an empty module or template. This preserves on-demand preparation for unrelated session code. Recognized protected images follow the existing policy described below; unrecognized images fail explicitly.

Native acquisition checks also retain paired Config/XML examples of omitted primitive qualifiers: unqualified `S` maps to an unlimited variable string, and unqualified `D` maps to date-and-time. Compatibility normalizes these measured defaults only; different lengths, fixed strings and date/time-only constraints remain errors.

Nested extension forms/templates are verified through their own existing SDK resource headers. Explicit links are checked against the selected base owner/kind; an explicitly adopted header with a nil link is resolved by name strictly within the already verified parent and kind, following the existing adopted-by-name rule. Unknown ownership, missing targets and ambiguity remain errors. Form payloads are not needed for this binding.

## Native type parity fixes after full-directory acceptance

The native UUID parameter is an existing scalar projection bug: Config uses
`# / fc01b5df-97fe-449b-83d4-218a090e681e`, while XML/EDT use `v8:UUID`.
The retained paired native artifact is the oracle, without a new language type.
Calculation-type charts are already catalogued ORM owners. Register their
reference value descriptors through the existing shared descriptor construction,
using native GeneratedType names, instead of rejecting a nonempty family or
creating file-only descriptors. Generic-family completeness remains strict.

## Native protected modules in file bundles

Native Configurator exports can contain Module.bin even for a reached common
module. Keep decoding in bsl-config-db, where the existing image recovery and
export-interface policy already live. The neutral source exposes optional raw
module image bytes; XML/EDT only acquire those bytes through their existing
bounded immutable resource cache. Binding delegates known images to the same
decoder and LoadOptions as database mode. This is file acquisition, never DB
code fallback. Unbound readers still report that no BSL text is present.

## Exchange-plan metadata validation

The SDK supplies a measured complete list of owned exchange-plan attributes.
The ORM catalog retains that immutable list for source-compatibility checks,
using the existing DocumentAttribute representation. It does not introduce
exchange-plan database mutation or object lifecycle behavior. Unknown membership
remains None; a missing field is never accepted by skipping validation.

## Legacy defined-type identity

The matched BUH native pair retains the older class-0 defined-type own header.
The SDK shared recognizer must preserve the same type identity in full
descriptors and lightweight headers so selective acquisition follows the
definition and its concrete reference dependency. File validation stays strict;
the file declaration does not invent a missing database type.
