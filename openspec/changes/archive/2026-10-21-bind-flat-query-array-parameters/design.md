# Design

The SDK already supports ParameterValue::List, empty IN lists and reference list elements. Convert each flat array element using the existing scalar parameter_value codec and wrap the owned result in List. Capture values when executing the request, leaving the BSL array and query parameter structure unchanged. Explicitly reject nested arrays and table containers inside a list rather than recursively traversing potentially cyclic application object graphs. This is a supported-profile limit, not a claim about every native container shape.

Native 1C 8.3.27.2342 evidence: docs/research/fixtures/query-array-parameter-native-1015. An own catalog record matches a reference array; a nil-only and empty array return no records. Query.Parameters retains the same array object and sees its later append. The own record is removed by the control. Original databases were not modified.

Implement the reached query binding failure first; the metadata root string indexing reached by the application's exception handler is not the first failure and is outside this change.
