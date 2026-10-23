# Design
The native 1C8.3.27.2342 control confirms `Диаграмма`/`Chart`, zero arguments, omitted parentheses, case-insensitive naming, matching type lookup aliases and presentation `Chart`. One and two arguments fail. Evidence is retained in `docs/research/fixtures/chart-constructor-1016/`.

Use `bsl-spreadsheet`, which already owns platform document types and the explicit unsupported geographical-schema constructor. Append ConstructorCode4 and a component TypeDescriptor; preserve codes1–3. Register the ordinary constructor/type tables and use the existing facade/compiler/VM pipeline. No core TypeId, new crate, new dependency, source rewriting or custom name resolution is needed.

The reached original helper stores a chart in a result structure. We have not observed this helper execute during session initialization. Its declaration must compile; actual construction deliberately raises a clear DynamicError until a measured object model is implemented. This is an explicit host implementation boundary, not a claim that 1C construction fails. Do not return an empty structure, Undefined or a fake chart. If original startup executes this constructor, that remains a real blocker requiring actual measured behavior.

The existing constructor-density test fixes the prior count at3. Prepare its one-line3→4 proposal separately; do not apply without the user approval required by the repository task. Add new facade integration tests instead of moving or rewriting existing tests. Archive only after the required regression gate is green.

The full vendor constructor page (`Chart/ctors/ctor_Auto.html`) confirms the zero-argument signature and is retained as `vendor-constructor.json`. An additional native control retains the rejected-argument descriptions: both report Constructor not found. The native successful construction result remains distinct from the deliberate host unsupported-model error.
