# Design

The existing get_index comment documents a deliberate historical workaround: For Each used numeric indexing, so non-string brackets had to mean a positional pair. The compiler now emits the separate CollectionItem operation. Override its runtime map branch with the old positional pair retrieval, then make get_index use the existing map storage lookup for every key. set_index already uses map keys and is unchanged. Other collection branches remain unchanged.

Native 1C 8.3.27.2342 evidence: docs/research/fixtures/map-key-index-native-1015. Type, numeric zero, UUID and enumeration reference keys return 11/22/33/44, missing type key returns Undefined, bracket assignment updates the type key, and For Each still visits four KeyAndValue pairs summing to 111. This deliberately replaces the historical numeric bracket workaround with measured platform behavior; it does not retain numeric brackets as positional traversal.
