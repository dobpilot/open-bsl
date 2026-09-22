# Support quoted multiline string fragments

## Why
Real configuration modules use the platform's second multiline-string form: adjacent quoted fragments without `+`. open-bsl currently parses the first fragment as a complete expression and rejects the next string token.

## What Changes
- Combine adjacent quoted string fragments into one string expression.
- Insert one LF between every pair of fragments, regardless of separating whitespace or comment lines.
- Preserve ordinary escaped quotes and the existing `|` continuation form.

## Impact
Only primary-expression parsing changes. Runtime strings, bytecode, and crate dependencies remain unchanged.
