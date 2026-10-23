# Design

Native controls measure-infobase-user-data-separation, measure-infobase-user-data-separation-values and the authenticated named-user receipt in docs/research/fixtures/user-data-separation-named establish a Structure property and mutable returned structure shared by reads on the same user object. A subsequent CurrentUser call receives the original session snapshot.

Use the existing SerializedValueGraph in Principal as an immutable optional snapshot. Cache its materialization only within each InfoBaseUserValue, never within Principal or across states. Validate one Structure root when reading the property. Unknown data causes an explicit runtime error; a known empty graph structure returns a real empty Structure. The host builder is not a database decoder and cannot prove that a graph was acquired correctly. SDK projection remains pending and original UNF success is not claimed by synthetic host tests.

Existing read-only user property setters are a deliberate scope restriction of this host; this change only supports the reached read and returned-structure mutation. It does not implement new-user editing or database writes.
