# Design

Reuse one private conversion helper for StrFind and StrReplace. Strings and unsupported types pass through unchanged; Undefined, Null, Boolean and Number use existing formatting. Numbers require no grouping, a decimal point and an explicit zero. This keeps UUID receiver behavior in the existing runtime method instead of creating another implementation. Native controls cover all three argument positions.
