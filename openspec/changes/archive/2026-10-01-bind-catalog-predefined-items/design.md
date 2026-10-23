# Design

The full vendor help document `platform-syntax-help:syntax-context:objects/catalog125/catalog126/object128/properties/prop2542.html` describes read-only catalog manager properties named by predefined items and returning the owning catalog reference. Independent UNF XML lists `Основной`, predefined identity `9b6065e7-092d-4be9-bed8-5b580763721c`, for `ВидыЗаказовПокупателей`.

Use the existing owned receiver-property names and ManagerValue property access. Do not introduce another resolver or change variable precedence. The acquired MetadataSnapshot already contains predefined names decoded by open-sdbl from `.1c`; project that inventory, without reading or retaining those resources a second time. Resolve the reference with the existing QueryRequest/data-adapter path and its existing VALUE translation.

Metadata predefined identities and actual database record references are separate inputs; never fabricate a record UUID from a metadata identity. Before implementing resolution, preserve native measurements and inspect the existing metadata/query adapter's measured mapping. Resolve through a session-bound adapter with its existing rights and data-separation handling. No global or thread-local connection cache.

Failure and missing-record behavior require a native probe before implementation. XML is evidence for this change; implementing a general XML configuration reader remains outside it.

Native 8.3.27.2342 measurement `measure-catalog-predefined-items.platform.txt` proves bare/qualified equality, parameter priority, assignment rejection, own reference type, filled state and equality with `PredefinedValue`. The fresh file database produced record UUID `aac7644e-bd2a-11f1-9a17-02d78b8d8e99`, different from the predefined metadata identity. The UUID line is evidence of that particular database and must not become a cross-database fixed expectation. Missing-record and module-variable probes remain pending.

The native `measure-catalog-predefined-uninitialized.platform.txt` probe uses another fresh file database with DontAutoUpdate. Reading the declared property raises “The predefined item is not found in the infobase data”. A NULL returned by the existing VALUE subquery therefore raises a missing-predefined-record error, not an empty reference. The module-variable probe is still running and has not produced an oracle.

The module-variable probe was rejected by native Designer CheckConfig: catalog manager modules contain only procedure/function definitions. The compiler output is preserved in `catalog-predefined-module-variables.compiler.platform.txt`. Module variable precedence remains the existing host-engine compatibility requirement, not a newly claimed native manager capability. The runnable native variable probe therefore declares only local variables inside methods.
