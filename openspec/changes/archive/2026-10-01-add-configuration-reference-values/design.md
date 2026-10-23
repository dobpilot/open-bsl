# Design

The native 8.3.27.2342 measurement is recorded in
tests/conformance/measure/1c/measure-manager-references.platform.txt.
Catalog EmptyRef has a nil UUID and is unfilled. GetRef preserves an explicit
UUID; repeated UUIDs yield equal references, while omitted UUIDs yield distinct
filled references. TypeOf prints the catalog name and recognizes its qualified
Russian and English type aliases. The printed name alone is not a lookup alias.

Owned immutable type descriptors are retained by Arc in the checked engine
catalog and in TypeRef. They are not leaked into process-global static storage
and are not re-created for every database session. Static TypeDescriptor stays
the dispatch descriptor, so existing method and property tables and VM caches
remain the existing mechanism. A default object hook reports that static type;
configuration references override it with their engine-registered value type.
The existing background engine recipe carries these immutable declarations,
just as it already carries registered libraries; this is not Session host data.
The change does not enlarge BslValue; the existing size test remains unchanged.

The host's receiver-context declaration applies only to the named configuration
module. It contains registered method descriptors, not an accept-all fallback.
Calls use ThisObject and the ordinary method path. Native priority against
global exports is measured by `measure-manager-context-priority`: the form
calls the global export and returns `global`, while the manager's unqualified
call returns its native catalog reference type. The receiver method therefore
precedes that global export.
Ordinary local function declarations keep the resolver's existing precedence;
the host declaration does not replace that established language decision.
An additional attempt to define a function with a platform method's name in
the native manager failed to produce output before timeout, so it is not
used as evidence for native declaration validity.
Dynamic value types have no constructor unless a later contract supplies one;
their existence must not manufacture object construction support.

Catalog references own the immutable catalog and their UUID. They create no
connection and read no row merely by being constructed. Reading properties or
using them as query parameters remains subject to the Session data adapter.

The receiver context is retained by the engine and reconstructed for dynamic
compilation from DynamicScope.module. It is not a serialized BSL import and
cannot expose additional exported functions or variables. Worker engine recipes
retain this immutable context. Unknown and duplicate context module declarations
fail construction. Native optional-argument measurement confirms that explicit
Undefined generates a new reference, while number, string and Null arguments
are rejected (measure-manager-reference-arguments.platform.txt).

The existing UUID value constructor becomes public through Value, retaining
its implementation and RandomHandle capability. Components reuse it instead
of duplicating UUID parsing or generation. ObjectProtocol gains a fallible
presentation hook with the old display behavior as its default; bsl-format
uses that hook. Empty catalog reference presentation is natively measured as
empty. Filled reference presentation is database-dependent; until the adapter
implements it, formatting fails explicitly rather than emitting diagnostic
Display. That adapter remains part of the unfinished data access task.

The ORM exposes construction and inspection of catalog references to source
adapters: catalog_reference_value receives the immutable catalog, object ID and
canonical UUID; catalog_reference_parts returns the object's metadata and UUID.
The database bridge reuses open-sdbl Guid.to_1c_bytes for parameter RRRef bytes
and the existing inverse permutation for results. Reference column constraints
retain the owner's metadata identity instead of being lowered to Binary/String.

The subsequent native `measure-catalog-create-item.platform.txt` probe found
that GetRef without a UUID yields UUID version 1. Reusing the version 4 UUID
constructor does not satisfy that observed behavior. Task 6 remains unfinished
until reference generation is corrected; changing only the version nibble of
random UUID bytes would not implement a time-based UUID and is not a solution.
The ordinary UUID constructor retains its existing measured behavior.

The same probe confirms CreateItem returns a new CatalogObject with an empty
reference, empty string code and empty description for the probe catalog.
These catalog-specific defaults are not evidence for numeric codes or arbitrary
attributes. CreateItem requires a separate configuration object contract and
acquired field metadata; reference creation alone must not fabricate that object.

Reference UUID generation uses the RFC 9562 version 1 layout, with actual
UTC time in 100 ns Gregorian-epoch ticks, monotonically increasing timestamps
within one SessionData, and a once-per-session random node/clock sequence.
The random node is marked multicast as required for non-MAC UUID nodes; it is
an explicit host implementation choice and does not claim to reproduce the
platform host's hardware address. Native consecutive values retain node and
clock sequence and increase timestamp by one. The implementation preserves
that property without reading the host network interfaces or database.
Layout source: https://www.rfc-editor.org/rfc/rfc9562.html#section-5.1 .
A shared ClockHandle wraps the existing HostEnv Clock, and CallContext can
receive it as a capability. This preserves with_clock injection and avoids
using a separate SystemTime clock inside the ORM. Existing InterpreterServices
construction stays valid; interpreter contexts attach the handle after creation.
Reduced contexts without clocks report CapabilityMissing(Clock).
