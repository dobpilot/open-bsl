# Design

Extend the existing configuration_method_context mechanism with an independent
finite property table attached to a module. This is receiver context, not a new
library global or a second interpreter. Explicit parameters, local declarations
and module variables retain their existing precedence. A matching receiver
property is considered before introducing an implicit local on assignment.
Modules without an attached table retain their previous behavior.

Read and write use the existing ThisObject expression and property instructions.
Dynamic snippets and worker reconstruction retain the same table and therefore
cannot resolve a different receiver from the original compiled module.

The native probe measure-catalog-object-properties and its copied object module
establish property initialization, bare read/write, parameter shadowing and the
supported exchange settings. Implementation follows measured results; unknown
operations fail explicitly. Mutable settings belong to the concrete object,
not the engine or a thread-local variable. No SQL write or write callback is
introduced by changing these settings.

Native 1C 8.3.27.2342 allows Code, Description and Ref reads during the object
module body, but rejects Code/Description assignment and IsNew before object
selection completes. Creation therefore executes the body before marking the
component object as selected. This is not a ban on every receiver operation:
ExchangeData.Loading reads as false during initialization. After creation it is
writable, and Recipients.AutoFill starts true and can be changed to false.
AdditionalProperties is initially an empty Structure. The probe also confirms
that the Code parameter shadows the receiver property without changing it.

Object callbacks must borrow the caller's RuntimeShapes instead of seeding a
second table from the catalog program. Entry compilation already preserves the
catalog prefix, but adds entry-specific names; dynamic fields created with a
fresh catalog-only table otherwise receive different NameIds. Swap the existing
RuntimeShapes into the nested execution and return it on completion or failure.
This shares names and shape transitions within that call without copying heap
values or changing bytecode instructions.
Configured dynamic fragments are seeded with the current caller name prefix,
including entry-specific names. Cache reuse checks that the cached unit and
current prefix agree; an incompatible prefix causes recompilation rather than
reusing field identities from another entry. Standalone cache behavior is kept.

The follow-up native probe invokes a pure exported function returning 17 through
ThisObject during initialization. It is rejected even without attribute access.
This closes OBJECT.MODULE.INIT_REENTRY using the saved native oracle: retain the
existing rejection and remove its unmeasured marker, registry entry and placeholder.
