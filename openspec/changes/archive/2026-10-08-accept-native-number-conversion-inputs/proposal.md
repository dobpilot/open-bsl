# Accept native numeric conversion inputs

## Why
The existing `Число` implementation rejects numbers and Boolean values, while
native 1C accepts them. This prevents equivalent recovery of opcode 70.

## What Changes
Accept measured numeric and Boolean inputs through the existing builtin.
Keep string parsing and unsupported-value errors unchanged.

## Impact
The VM builtin interception changes; no API names or dependencies are added.
