# Design

Use query_selection_field for a string index and retain the numeric path unchanged. This shares lookup and row positioning with property access. Native proof: docs/research/fixtures/query-selection-index-native-1015, 1C 8.3.27.2342, own file infobase. Exact and mixed-case column names return 42, numeric index 0 returns 42, missing name raises. No claim is made yet that this is the receiver of the latest original demo BadIndex.
