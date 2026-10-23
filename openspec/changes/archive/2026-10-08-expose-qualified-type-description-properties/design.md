# Design

The red ORM replay uses the saved `accumulation-register-balance/empty.bsl`
and its native receipt. It currently returns `ОписаниеТипов` instead of
`String`, then fails accessing the string qualifier property.

Expose existing string and number qualifier objects from the stored column
types, reusing their current property tables. Preserve unknown/unsupported
cases explicitly until measured. Measure aliases, absent-type qualifiers,
read-only assignment and composite descriptions before extending those cases.
The native receipt uses English type presentation; establish the current
formatting contract and locale evidence before changing presentation globally.
No dependency, SQL generator or ORM-specific value representation is needed.

The column ValueType getter currently maps each ColumnType to its TypeRef,
discarding qualifiers. Use QualifiedTypeDescription when qualifiers are present;
retain the existing public Rust TypeDescription representation for an empty
qualifier list, including acquired value types. The unchanged
`table_column_round_trip_preserves_a_declared_type` regression checks this
boundary; qualifier replay must continue preserving nonempty qualifiers.
The new native control under `type-description-qualifiers` confirms 11
observations: length 12, number 38/2, variable/any policies, zero qualifiers
for an absent type, read-only assignment, English string-property alias,
and Russian string/number presentation. It does not establish composite
presentation or every qualifier policy.

The constructor previously accepted but discarded string qualifiers and
rejected number qualifier objects. Retain the measured native positions
(number first, string second after the type list) through ColumnType. The
eleven-line facade replay now differs only in two existing enum display
strings (`Variable-length`/`Any` versus Russian platform text); do not change
global enum presentation solely to satisfy this mixed-locale receipt.

The independent native `measure-encoding` control establishes the existing
serialized string qualifier policy: `S,12,1` is variable and `S,12,0` is
fixed. Constructor retention, property reads and adjustment must all use
these native codes. Treating 1 as fixed caused three existing SDK-acquired
business-process replays to fail; correct the implementation without changing
their receipts. Catalog code and document number construction must distinguish
the string policy from numeric fraction zero, preserving the existing explicit
document-number fixed-length setting.
