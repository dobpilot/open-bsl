# Dependency-directed configuration preparation

## Current implementation and evidence

`SessionFactory::new_with_entry_sources` adds all named and global server common
modules to roots. `rooted_recipe` parses every source to discover exports, then
resolves the entire selected closure. `finish_catalog` verifies every resulting
program. The database `FactoryConfigSelector` retains the source resources of
all module-bearing objects. `load_factory_inputs_with_metadata_and_options`
decodes the complete source snapshot and ORM catalog.

The successful alternating one/two-worker comparison is preserved in
`docs/research/fixtures/parallel-rooted-preparation-1020`. Its phase timings
identify aggregate costs, not the CPU share of individual resolver operations.
This change must be compared against that implementation with counters as well
as timings; reducing work is the primary acceptance criterion.

## Preparation boundary

Separate configuration discovery from materialization. Discovery retains stable
module/object identities, names, execution attributes, ownership, global export
signatures and resource locations needed to resolve names. It must not construct
all procedure bodies, resolved trees, bytecode or detailed ORM objects.

Discovery is not free: a global bare function requires the global export
declarations, and metadata names may reside in object description resources.
Reading such resources for their index is permitted; resolving their bodies or
materializing every detailed object is not. If the storage has no separate
declaration table, use a declaration-only scan of the necessary sources rather
than promising that no source bytes outside the closure can ever be read.

Initial roots are the session module and dependencies of supplied entry sources.
The existing semantic resolver discovers static module dependencies; explicit
imports remain dependencies. Manager/object contexts and metadata references
add the corresponding dependencies without making every manager a root.

The implementation must not turn arbitrary string literals into dependency
guesses. A name computed from data is resolved when its actual value is known.
The existing dynamic-common-module test is a mandatory regression case.

## First access

A reached but unprepared module loads its effective source through the selected
source provider. The ordinary open-bsl parser, resolver, compiler and verifier
prepare it and its dependencies. A recursive preparation state prevents
re-entering the same preparation while permitting existing supported module
cycles. Published module identities and existing links must remain valid when
the prepared set grows; an active frame cannot be renumbered.

Immutable prepared code is cached at the factory boundary. Each Session keeps
its own initialization state, variables, parameters, principal and connection.
Preparing a dependency neither executes its BSL nor copies another Session's
mutable values. Background sessions follow the same ownership rule.

Do not replace missing code with empty bodies or successful placeholder values.
For protected modules the explicitly selected existing decoding/stub policy
remains authoritative.

## Metadata

Retain enough discovery information for lookup and complete collection identity
enumeration. Materialize detailed object properties, attribute/type descriptions
and executable manager/object contexts only when reached. A metadata collection
must not pretend to contain only already materialized objects. Enumeration or
an actual request for all details can legitimately reach the complete set.

Dependencies include referenced types and the acquired configuration facts used
by rights, separation, principal selection and session parameter initialization;
these cannot be removed merely because their names are absent from BSL source.
Use the existing measured metadata decoders; do not infer new storage semantics.

Selected resources are immutable for the lifetime of the factory. The SDK caches
their parsed facts once and resolves the accumulated facts when the reached set
grows; it does not repeatedly inflate earlier resources. Physical table and
dependency lookup indexes preserve the resolver's existing first-match order.
The source retains selected resource bytes needed by the existing ORM projector,
not the discarded discovery bodies. A descriptor and its predefined-value data
are acquired together before publication. A failed acquisition must not turn a
retry into a successful incomplete snapshot.

The ORM projection is reused while the snapshot identity is unchanged. Its
temporary value-graph registry registers the reference types actually needed by
acquired filling values; column declarations already retain their type
descriptors. This avoids reconstructing an execution registry for every type in
the discovery index merely to serialize a reached object's defaults. Native
filling-value parity remains a required regression check.

Only the root ORM catalog exposes complete discovery. A temporary projection
used to return one object's details includes acquired owners and their acquired
type dependencies, not every discovery-only owner. It is never published as the
session's root catalog. Common-attribute composition still uses complete
discovery, and serialized defaults materialize through the session's registered
types. Tests must preserve complete root enumeration after detail acquisition.

The module dependency walk acquires receiver metadata for the currently reached
frontier as a group before resolving that frontier's bodies. Optional default
methods on ModuleContextSource and MetadataObjectSource carry this acquisition
through the session context adapter; providers without batching remain valid.
MetadataCatalog forwards explicit owner groups to its provider. The database
provider resolves the group's acquired facts once; ordinary per-object access
then reuses that immutable snapshot and projection. Only modules already queued
by semantic or explicit dependencies enter a group. No unknown future target or
unrelated owner is prefetched, and no BSL executes during acquisition.

Query preparation uses one immutable metadata snapshot. Expanded RLS may reveal
additional dependencies; acquiring those dependencies restarts preparation of
the original query against the new snapshot. Prepared-query fingerprint checks
remain intact, and previously issued snapshots remain unchanged.

## Database and crate boundaries

The current SDK `configuration_selected` is insufficient for truly selective
loading: `ConfigPlan::Selected` decodes all metadata, and `ConfigSelector::retain`
is called after resource assembly. Discarding bytes at this boundary saves
retention but does not avoid full reads and decoding.

The SDK needs a bootstrap/discovery path and bounded named resource reads,
including effective extension resources, without a hidden full metadata pass.
Reuse its provider implementations, part assembly, limits, diagnostics and
existing configuration-consistency policy. Do not add SQL in open-bsl crates.
Do not recalculate a fingerprint for every new Session.

`bsl-config` owns source-neutral discovery/loading contracts; `bsl-config-db`
implements database loading. The XML and EDT crates remain placeholders. ORM
owns metadata materialization; session owns parameters and session resources.
The facade exposes consumer contracts. VM and base runtime gain no provider,
database or frontend dependency. No external crate is required.

## Verification boundary

Validate declaration-index invariants before publishing the factory, including
global-name conflicts. Validate newly materialized bytecode and its cross-module
links before executing it. Do not verify untouched bodies or repeatedly verify
the unchanged shared declaration tables merely because another module is loaded.
Keep the full verifier for untrusted complete bytecode images.

## Implementation order

First add failing work-count tests and the neutral indexed-source contracts.
Then implement incremental catalog preparation in the facade/compiler/VM while
preserving active identities. Add lazy ORM details, connect bounded SDK reads,
and only then switch configured CLI defaults. Do not ship a roots-only shortcut
that loses dynamic targets, and do not claim selective acquisition while still
using a full SDK metadata pass.

## Compatibility and measurement

The timing of errors in untouched bodies changes intentionally: body errors
surface when that body/module is prepared, not at unrelated startup. Declaration
conflicts remain early errors. Existing measured runtime behavior remains the
oracle. Any newly chosen platform semantics requires the repository's marker,
registry and native measurement workflow before implementation.

Use deterministic read/decode/resolve/codegen/verify counters in test providers.
For performance claims run alternating baseline/candidate BUH invocations,
strictly one `bsl-cli` process at a time, with the existing memory guards.

## Incremental execution implementation

Prepared module IDs remain positions in the immutable declaration index. A newly
reached semantic closure is compiled by the existing configuration compiler as
one batch; its name and shape tables are not cumulative copies of every earlier
batch. The compiler's existing narrow-name layout pass remains authoritative.
At first entry the VM binds that batch's names to the current execution's
namespace, including names introduced earlier by dynamic structures. Bound code
and component links belong to that execution; parsing, resolution and original
code generation remain cached at the factory boundary. The namespace survives
successive calls into the same module instances.

Visible imports use the existing shared `ModuleImports` storage. Excluding the
current module does not copy all other module declarations. A verifier cache
reuses duplicate-name/alias validation only for the same immutable `Arc` table;
weak references neither retain old tables nor permit mutation of a cached table
through `Arc::get_mut`. Changed copies are verified again. Instruction, link,
arity and module-state checks remain in place; untrusted complete images keep
the full verifier.

A background worker receives a Send source recipe and constructs its own indexed
engine. Submission reads only the actual target's declarations; worker execution
prepares its target and dependencies. This does not transfer the parent's live
module instances or runtime name namespace to the worker.

### Declaration text from protected resources
The source contract distinguishes complete source text from declaration-only text. Existing sources return complete text by default, which the engine reuses for compilation. A database source can project protected native declarations without recovering bodies; the engine must never compile this projection as executable source. Private declarations retain method order so export chunk identities do not shift. Actual access still uses the selected protected-body policy and existing decompiler.

## Selected database metadata implementation

The database source keeps compact SDK headers and resource locations separately
from reached detailed resources. Physical schema, DBNames and extension storage
structures are acquired once per factory. Selected-resource resolution uses the
SDK's strict missing-descriptor diagnostics to fetch separator parameters, then
follows declared type/member GUIDs using the header index. The existing ORM
projection handles reached objects; collection enumeration uses the full index.
Route maps supply point identities without preparing all business-process fields.

Each query compilation owns an immutable `Arc` snapshot throughout preparation,
permission checks and compilation. SDK query AST discovery selects direct sources
and constants. Expanded restrictions add their actual query dependencies; a new
snapshot requires a new prepare step, preserving the SDK fingerprint contract.
No source/metadata mutex or `RefCell` borrow spans BSL parameter initialization.
The CLI retains a factory acquisition connection for late reads and creates a
separate connection for each executable Session. Users/rights use that already initialized factory connection before execution;
the host connection list counts only executable Sessions.

## Local member operands and runtime field identities

Late modules retain their original verified bytecode, local member-name table
and narrow member operands. Global runtime field identifiers must not be written
into `u16` method operands: a Session can already contain more than 65,536 field
names before a new member is reached. Execution caches bind structure-property
sites and shape descriptors to runtime identifiers instead. Component calls keep
the original program spelling and linking tables. The bytecode helper prepares
runtime shapes without rewriting the source Program; no serialized format or
input-verifier invariant changes. This also avoids cloning a complete runtime
name table into each entered module. Regression checks cover large runtime
namespaces, cross-module structures and existing unbundled/debug images.

Dynamic snippets retain their caller Program name prefix because copied static
function chunks keep local name operands. Dynamic frames and notification
callbacks share the execution runtime namespace; their field-cache bindings
preserve structures passed across static/dynamic calls. A compatible identity
mapping also requires exact spelling so literal structure keys keep their
original case during enumeration.

## Final verification and measured outcome

The approved existing-test adjustment replaces the artificial Session-wide
u16 name-limit expectation with the reached module-body error. Runtime sources
are unchanged from the measured final binary. The full workspace passes with
3,996 tests passed, zero failed and one intentionally ignored measurement dump.
Formatting, workspace/all-target Clippy, warnings-denied rustdoc, the facade
consumer and unchanged normal RT/VM dependency trees pass.

One sequential BUH A/B pair without concurrent builds/tests completed successfully:
542.18 seconds baseline versus 292.10 seconds candidate, sampled peak RSS
6,235,100 KiB versus 5,218,708 KiB. This is one pair, not a statistical estimate.
The candidate prepares 4,796 of 10,024 indexed modules and acquires 3,726 detailed
metadata resources. Stdout and BSL-emitted events match; the baseline-only warning
about three unread unused resources is retained as an explicit acquisition
diagnostic difference. Guarded runs, source/binary hashes, instruction-cost checks
and complete check logs are retained in
`docs/research/fixtures/on-demand-session-preparation-1020`.
