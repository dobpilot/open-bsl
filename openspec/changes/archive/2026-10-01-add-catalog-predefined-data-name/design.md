# Design

Primary source: platform-syntax-help:syntax-context:objects/catalog125/catalog126/object130/properties/PredefinedDataName9808.html, installed 8.3.27.2342, Russian locale. This read/write String property identifies a configured predefined item; empty name removes designation, invalid names fail.

Measure initial state, access during module initialization, accepted names, case handling, conversion and failure preservation before runtime implementation. Store staged state per concrete object and validate against acquired names. Do not change predefined-manager lookup, metadata extraction, persistence, variable priority or core conversion behavior.

Native oracle contains 16 rows. Body reads empty String; body writes fail. Known names use folded lookup but preserve assigned spelling. Unknown names and surrounding whitespace fail and retain the prior value. Undefined and Null clear the name. Measured Number, Boolean and Array assignments fail. A second object stays empty. Store a String value per object; selected-object guard precedes assignment, and acquired names validate nonempty strings without trimming or canonicalizing them.
