# Design
Reuse EnumKind, EnumValue and TypeId. Native 8.3.27.2342 reports QueryRecordType for members, EnumQueryRecordType for the bare enum and its meta-type, with Detailed record, Total by group, Total by hierarchy and Grand total in that order. Native controls confirm aliases and both Type() names. Evidence and the full installed vendor page are retained in docs/research/fixtures/query-record-type-1015.

The original manager fails during compilation at line416; this is not evidence that selection.RecordType executes during session initialization. Add no query execution or selection behavior to resolve an absent global enum.
