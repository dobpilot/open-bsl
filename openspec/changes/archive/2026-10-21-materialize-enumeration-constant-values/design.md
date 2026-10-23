# Design

Delegate enumeration owners to the existing enumeration_reference_value helper. Convert canonical UUID bytes with the existing UUID formatter; nil UUID maps to the existing empty identity. The database adapter routes enumeration and chart references through configuration_reference_value; other dispatch stays unchanged.

Native 1C 8.3.27.2342 proof is retained in docs/research/fixtures/enum-constant-native-1015: a constant set to First equals that enumeration member and has its declared EnumRef type; setting EmptyRef returns the same empty enumeration reference. This ran in an own file infobase. An initial probe incorrectly called UUID retrieval on an enumeration value and is not positive evidence.
