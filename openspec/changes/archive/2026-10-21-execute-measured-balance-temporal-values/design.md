# Design

Use the existing PointInTime runtime value, reference conversion, enum BoundaryType and SDK compiler. An immutable ORM Boundary object retains its original Value and boundary kind. Its constructor accepts the measured Date, PointInTime, Number, Boolean, String, Null and Undefined inputs; accepting a wrapper does not make its payload valid in a query.

The readonly properties are Value/Значение and BoundaryType/ВидГраницы. PointInTime/МоментВремени is not a property. The default kind is Including; an explicitly supplied Undefined or Number kind fails. Reuse existing value equality for the measured scalar/date/point cases, without inventing comparison ordering.

Retained evidence: docs/research/fixtures/temporal-balance-native-1019 (60 observations) and docs/research/fixtures/boundary-object-native-1019 (independent object probe). English type presentation is Bound and object presentation Boundary. Russian presentation was not measured and must not be inferred from the English run.

For Balance only, an absent recorder reference and typed empty recorder references follow the measured date cutoff, while nonempty references use the metadata recorder kind and physical reference. This does not alter generic PointInTime comparison. Reuse SDK projection recipes to reconstruct logical PointInTime results; reject malformed column recipes explicitly. No labels, caller ordinals or SQL text guessing.

SDK 0.9.28 must be independently reviewed before transferring its actual API. Do not implement against an assumed draft signature. SQL Server generation checks do not substitute for a live backend run. Existing placeholder-constructor assertions require a concrete approved test patch; preserve arity assertions and all unrelated old expectations.
