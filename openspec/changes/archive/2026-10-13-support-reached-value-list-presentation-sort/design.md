# Design

The method belongs to the existing ValueList MethodDescriptor table, not BuiltinMethod. Both names use the existing component dispatch. The accepted argument count is zero or one; omitted and Undefined direction select ascending order. Other supplied values must be a SortDirection enumeration value.

Native 1C 8.3.27.2342 results are retained in tests/conformance/measure/1c/measure-value-list-presentation-sort.platform.txt. Missing presentations remain empty sorting keys rather than falling back to item values. Sorting reorders the existing items and preserves their identities.

The retained controls distinguish ASCII case, Cyrillic case and Е/Ё. Existing table collation and raw string comparison do not reproduce these controls. Any separate comparison logic must explain this incompatibility at its code site. Pairwise controls are evidence for their measured inputs, not proof of arbitrary Unicode collation. Further semantic choices require native measurements or the repository's unmeasured-question machinery.

Before implementation, extend the native controls to verify descending equal-key stability and mixed multi-character strings. Keep existing SortByValue behavior unchanged.
