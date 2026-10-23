# Design

Use the existing root collection reader to identify the authoritative common-attribute IDs, and the SDK snapshot descriptors for acquired usage and types. Do not identify the full collection by filtering common_attribute_usage.is_some(): that would silently omit unrecognized forms. Explicit Content entries override AutoUse in the two measured catalog modes; an absent entry follows the acquired default. Any unknown collection, usage or referenced descriptor remains unknown or causes an explicit acquisition error, never a claimed complete partial collection.

Store applied common attributes separately from own attributes, using the existing acquired attribute representation and qualifier projection helpers rather than duplicating the type decoder. Keep custom names out of the fixed global descriptor table. Extend the current object-module receiver property-name list, preserving all resolver priorities.

Object values belong to the existing catalog instance. Reuse current typed conversion helpers where their measured behavior agrees with the common string probe; factor the helper if needed rather than copying it. Initialize the measured string value and reject writes until the object is selected, as the native body probes establish. Unsupported types fail when accessed rather than manufacture values or query SQL. Common metadata membership does not change the own Attributes collection.

Native receipts cover AutoUse DontUse with explicit inclusion and AutoUse Use with explicit exclusion, case-insensitive names, string defaults, number conversion, length 150 and object isolation. Data separation, persisted rows and other common-attribute types remain broader runtime obligations.
