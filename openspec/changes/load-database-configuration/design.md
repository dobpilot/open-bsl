# Design

`ConfigurationCodeSnapshot` is an immutable catalog of the configuration identity, logical modules, their kinds and owners, import relationships, and one final source body per module. It contains no connection, credentials, database session, parser state, or provider-specific resource representation. A build selects exactly one code loader for the complete snapshot; modules from database, XML, and EDT sources are never mixed.

`bsl-config-db` consumes a combined acquisition from `open-sdbl-db`: reassembled base and active-extension resources, `MetadataSnapshot`, and the existing resolution report are obtained in one bounded read-only acquisition. The loader decodes the configuration graph, applies active extensions in platform order, and emits only the effective module graph. An extension form the loader cannot apply is an error naming that extension; silently running base code is forbidden.

The combined acquisition is a requested upstream change. It reuses the existing provider queries, part assembly, limits, metadata pipeline, and extension store support. `open-sdbl` remains unaware of BSL module formats. There is no configuration fingerprint check when a session is opened. Updating a base requires the host to load a new snapshot and build a new session factory.

`bsl-config-xml` and `bsl-config-edt` contain crate documentation describing their future responsibility. They expose no loader that always fails, add no CLI options, and initially have no dependencies. Their future implementations must produce the same `ConfigurationCodeSnapshot` contract.
