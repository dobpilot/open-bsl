# Allow opaque exported placeholder parameters

## Why
Original UNF and buh contain a protected exported declaration with named parameters, opaque flags and no default table. The user authorizes a deliberately simplified mandatory signature for placeholder loading.

## What Changes
- Permit this opaque header shape in interface-only loading.
- Keep both parameter names and require all declared arguments, without decoding opaque flags or inventing defaults.
- Keep full decompilation strict and all malformed name/count checks.

## Impact
This is explicit host placeholder policy, not inferred native semantics. No dependency or VM changes.
