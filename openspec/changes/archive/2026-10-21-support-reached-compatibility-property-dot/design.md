# Design

Retain MetadataPropertyEnumeration and the single runtime enum member table. Remove only the CompatibilityMode-specific rejection in get_property. Unknown members still fail through lookup_member. This does not make the compatibility enumeration available as a bare global or as a newly registered type name.

The retained native control in docs/research/fixtures/compatibility-property-dot establishes DontUse and Version8_3_24 dot values, equality with indexed access and English aliases, and the Compatibility mode type display. The old comment preserved an earlier indexed-only contract; the new measured dot behavior deliberately extends that contract.

Add a public ORM integration test executing the entire native control through the existing facade, with no changes to existing tests. Repeat original read-only demo to identify the next terminal result.
