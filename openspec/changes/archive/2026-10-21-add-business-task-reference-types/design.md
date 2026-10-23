# Design

The independent control XML was loaded and compiled by native 1C 8.3.27.2342 through the public Unica runtime. Its compiled Config confirms BusinessProcessRef and TaskRef TypeIds at position 5 in measured forms 30/49 and 33/52. The source XML is not a native XML dump; evidence retains that provenance explicitly.

Use the existing immutable configuration ValueTypeDescriptor declarations. Register Russian and English qualified lookup names from the configured owner name and the native reference category. Add both kinds to the existing SDK reference map used by ColumnType projection. Do not duplicate the Config parser or the reference representation. Unknown reference types still fail characteristic value-type projection explicitly. Existing declared references and resolver priorities remain unchanged. Manager methods, creation, persisted reads and writes remain outside this change.
