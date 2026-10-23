# Design

Reuse EnumKind, EnumValue, TypeId and their existing alias and presentation tables. Declare only the reached БезУзора / WithoutPattern member of ТипУзораТабличногоДокумента / SpreadsheetDocumentPatternType. Native 8.3.27.2342 evidence in docs/research/fixtures/spreadsheet-pattern-native-1016 establishes names, case folding, value presentation No pattern, type presentation and alias equality. No ordinal or numeric conversion is inferred.

The original failure occurs during compilation. Adding the value does not claim implementation of РисунокДокумента.Узор or successful session initialization; replay decides the next reached requirement. Other pattern members remain outside this change. Preserve current RT/VM normal dependencies and existing tests.

The first full CLI comparison caught a candidate meta-presentation mismatch: the platform uses EnumSpreadsheetDocumentPatternType for the namespace and its metatype. Retain that initial mismatch and correct the existing meta-name table; do not change the native oracle.
