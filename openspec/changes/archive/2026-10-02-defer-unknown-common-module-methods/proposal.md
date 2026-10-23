# Defer unknown common-module methods in configuration sessions

## Why

Native 1C 8.3.27.2342 allows a valid export to run when another export calls
an unknown method of a known common module. The failing export raises a
catchable error before its body and arguments execute. Current configuration
compilation rejects the entire graph instead, blocking original buh startup.

## What Changes

- Under the existing common-module name-error opt-in, defer an unresolved
  imported method in a common-module function body until that function is called.
- Keep standalone compilation, non-common modules and unknown bare functions strict.
- Reuse the existing method-body error mechanism; add no executor or opcode.

## Impact

Sema resolution under the facade policy and native-oracle tests are affected.
This does not recover protected source or make compiled platform images executable.
