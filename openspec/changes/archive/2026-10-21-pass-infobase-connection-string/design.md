# Decisions

The registration string is opaque host input. The SQL host/database parameters do not define a 1C cluster registration, so CLI never derives this value from them or reads ibases.v8i automatically. The diagnostic original demo repeat obtains its separately supplied exact registration from the user's existing local file; this is not CLI behavior.

Both separated and equals argument forms use the existing flag parser. Empty, duplicate, or standalone use fails before connecting and preserves the invocation on parse failure. Omission leaves the existing missing-host-value behavior intact. SessionFactory::with_connection_string already stores the string without normalization. No additional language semantics are introduced.
