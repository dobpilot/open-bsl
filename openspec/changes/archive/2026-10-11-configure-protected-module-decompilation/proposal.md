# Configure protected module decompilation

## Why
Protected modules can prevent snapshot loading because complete body recovery
is unsupported. The host needs an explicit choice between an exported
interface with intentionally empty implementations and complete decompilation.

## What Changes
- Default database loading to exported declarations from image-only modules:
  empty procedures and functions returning True.
- Add loader options and `--decompile-protected-modules` to request the existing
  strict complete-body recovery, including initializer and private methods.
- Preserve text modules, active-extension processing and ordinary BSL execution.

## Impact
This intentionally changes protected-module default execution. Interface-only
sources are placeholders and do not claim equivalence to the original module.
No external dependency, VM behavior or platform semantic assumption is added.
