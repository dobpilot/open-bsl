# Design

Use the existing MethodDescriptor dispatch with exact zero arity and the existing runtime array constructor. Copy each item value into a new array, rather than returning value list items or presentations. Do not alter item properties or list ordering.

The installed vendor syntax help for 1C 8.3.27.2342 documents this method at platform-syntax-help:syntax-context:objects/catalog234/catalog235/ValueList/methods/UnloadValues206.html. Nine retained native lines establish empty results, scalar order, Undefined and independence of array slot replacement. Changing a Structure property through the returned array changes the same object held by the list. Ordinary Value cloning therefore preserves measured mutable-value identity; deep copying would be incorrect.
