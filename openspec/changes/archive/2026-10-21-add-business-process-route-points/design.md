# Design

Use open-sdbl 0.9.8 acquisition as the sole decoder. Project nested points
into the immutable ORM catalog. Preserve unknown composition independently
from a known empty collection and retain the generated route-reference type.

Measured demo points can share a GUID across different business processes.
Identity therefore includes configuration, owner, and point GUID. Never put
points into the global metadata-object GUID index. Reuse existing value-type
and component protocols for equality and facade registration.

Expose `ТочкиМаршрута` / `RoutePoints` as a read-only business-process manager
property. Register the same property in its own manager-module context through
the existing facade mechanism. Local and module-variable precedence remains
the responsibility of the existing resolver and is unchanged.

Before implementing runtime collection/reference behavior, obtain native
receipts from the independent nonempty control. The SDK control was saved
with incomplete diagram connections; it proves metadata composition, not
business-process execution. Do not execute the process or original startup
code to obtain collection evidence.

Background sessions use the existing acquired catalog. This change does not
introduce mutable shared state or database connections in route-point values.
