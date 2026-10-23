## Why

The original demo module `РаботаСФайламиВызовСервера` contains a native
GeographicalSchema constructor in its file-extension branch. Its unsupported
name stops compilation before session initialization can be investigated.

## What Changes

Register the documented zero-argument constructor and distinct type in the
document component. Preserve an explicit execution error until the actual
geographical object model and file reader are implemented. This change does
not claim successful geographic-file processing or complete demo startup.
