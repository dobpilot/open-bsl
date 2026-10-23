# Design

## Evidence
See `docs/research/characteristic-manager-create.md` and its independent native XML, compressed resource and platform receipts. The syntax help version is 8.3.27.2342. Creating an element does not itself write it to the database.

## Ownership
The immutable ORM catalog owns characteristic settings and configured value types supplied by the host. The SDK owns Config parsing; open-bsl must not add another Config parser. Unknown settings are distinct from an empty known type list. SDK 0.9.7 supplies measured characteristic settings and own attributes. The live projection still requires all configured reference type declarations; missing declarations fail explicitly and do not become a partial type list.

## Instance lifecycle
Create an independent configuration instance and empty standard values. Run the existing object-module initializer before selecting the new object. During its body Code and Description are empty strings, ValueType contains zero types, Description writes and IsNew fail. After initialization, install the acquired configured types and select the object. The native control then observes four types and IsNew=true.

## Existing machinery
Use configuration_method_context, configuration_property_names, ConfigurationInstance and the existing value/type system. Keep distinct characteristic type names; do not present a catalog object as a characteristic object. Reuse measured scalar qualification rather than implement another conversion engine. Catalog behavior and global resolution priority remain unchanged.

## Bounds
Only measured object members are implemented. Persisted reads, writes, record sets and unmeasured lifecycle behavior remain explicit unsupported operations. Additional aliases, qualification behavior and metadata variants need direct evidence before their tests are accepted.

## Type enumeration
The native `characteristic-manager-types` receipt establishes that `TypeDescription.Types` returns a mutable array independent of its descriptor. Implement this operation in the existing builtin method table, not as an ORM-specific replacement type.

## Exchange settings
The independent characteristic exchange receipt confirms the same loading and recipient defaults as the existing ORM exchange parameters. Reuse `catalog_object::exchange_parameters_value` per instance; do not introduce a second exchange object implementation.

## Own attributes
Reuse the immutable `DocumentAttribute` descriptor already shared by catalog attributes. Keep a separate optional characteristic list; `None` is unknown and `Some([])` is complete empty. The first supported own-attribute shape is the independently measured variable-length string. Reuse the existing scalar adjustment helper for that shape and fail explicitly for other unmeasured shapes. Per-instance slots hold values; acquired names extend the existing receiver context rather than globals or module locals.

## Configured value type qualifiers
The independent native `characteristic-value-type-qualifiers` receipt establishes compound conversion, string truncation, numeric rounding and saturation, and Undefined defaults. Preserve the existing `ColumnType` representation rather than reduce it to `TypeRef`. Reuse runtime scalar adjustment; do not introduce another ORM conversion engine. Keep the existing narrow standalone constructor behavior additive. The runtime representation and facade API must be reviewed before implementation, including assignment and snapshot behavior.

The native internal-text round trip contains `S,100,1` and `N,15,2,0`; passing the same descriptor to a typed column applies the same bounds. The implementation therefore must preserve qualifiers in internal text and column construction as well as the ORM property. Reuse the existing column Pattern reader/writer and qualified column storage. A separate runtime object variant is preferable to an ORM extension pretending to be the native type: extension snapshots require a registered immutable value type and do not accept a native TypeRef. Keep the legacy unqualified variant for existing constructors and tests. Introduce a documented `BslValue::new_qualified_type_description(Vec<ColumnType>)` factory through the existing facade, with native type identity and the existing builtin method table. Do not silently expand cross-session graph support: existing type descriptions are rejected there, and qualified descriptions must retain that explicit rejection until a separate measured change supports it.

## SDK 0.9.7 projection
The SDK now acquires measured characteristic settings, own attributes and reference types. Project them after the immutable catalog has declared its configured reference types, reusing the existing document attribute and ColumnType projection. Add a narrow catalog builder accepting optional characteristic settings and attributes; retain unknown values rather than substituting empty lists. An unresolved configured value type fails projection explicitly. Reference types of characteristic plans participate in the same reference map as catalogs, documents and enumerations.

## AdditionalProperties
Original demo compilation reaches a bare AdditionalProperties access in the characteristic object module. Preserve the independent native control before implementation. Reuse the existing runtime Structure and one per-instance slot, as catalogs and documents already do. Expose a getter only; mutation of the returned structure is distinct from property assignment. Initialize it before the object-module body and preserve it after selection. Do not infer properties or create a new structure implementation.
