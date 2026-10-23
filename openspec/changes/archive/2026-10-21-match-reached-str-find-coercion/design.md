# Design

String formatting remains in bsl-vm through the existing bsl-format path, like StrCompare and StrOccurrenceCount. bsl-rt remains free of a formatting dependency. Native controls cover Undefined in both positions, empty needles with direction/start/occurrence, numbers, booleans and Null. An empty needle returns one before optional search argument handling. Do not insert a fabricated application parameter or modify original configuration code.

Native numeric controls require the existing numeric format with grouping disabled, decimal point and explicit zero. Restrict new conversion to String, Undefined, Null, Boolean and Number; retain the old path for other types and for ordinary string arguments.
