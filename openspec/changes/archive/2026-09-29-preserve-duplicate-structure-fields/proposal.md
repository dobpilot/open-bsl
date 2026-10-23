## Why

UNF contains literal `Structure` constructors with repeated field names. The
compiler currently interns every literal field into a shape, making duplicate
names share a `NameId`; bytecode verification then rejects the complete
configuration. The computed constructor also loses the last spelling of a
repeated key during iteration.

## What Changes

- Keep the shaped bytecode path for literals with distinct folded names.
- Evaluate literals with repeated folded names through the existing runtime
  constructor, preserving the platform's single field, last value and last
  spelling behavior.
- Preserve the last spelling when `Insert` replaces an existing Structure
  field, including iteration and string-bearing copies.

## Impact

This changes the BSL `Structure` constructor and method behavior to match the
measured 1C platform. It adds no external dependency or bytecode opcode.
