# Design

Primary vendor source: platform-syntax-help:syntax-context:objects/catalog125/catalog126/object128/methods/GetTemplate377.html, version 8.3.27.2342. GetTemplate has one mandatory template argument (String or template metadata) and returns the actual template object.

UNF source reaches the missing declaration in Catalog.Counterparties.ManagerModule, external-data loading procedure, lines 865 and 871. This is a linking requirement, not evidence that startup executes template retrieval.

Use the existing finite catalog manager method table. Keep the current exported manager procedure when it has the same name, as for newly introduced attribute lookup. No arbitrary unresolved function fallback is allowed. Real template decoding is still required if execution reaches this boundary: unsupported retrieval must fail explicitly without a successful substitute.

The user approved the stale catalog-negative expectation change. Both document_templates.rs and data_processor_templates.rs contained this same negative assertion; only their Catalog entries were removed, leaving constant and information-register checks intact.

The reviewed patch has been applied. Four new catalog tests verify the explicit retrieval boundary, receiver arity, variable priority and preservation of existing exports with zero and five parameters. Real template decoding remains open if original startup reaches retrieval.
