# Match the native configuration extension hash type

## Why
Original demo startup already has a matching extension version record, but the ORM exposes HashSum as String. Base64String(String) correctly returns an empty string, so the computed description fails to match. A direct native control establishes BinaryData despite the installed help page declaring String.

## What Changes
Expose HashSum and its alias as BinaryData decoded from the existing acquired Base64 value. Re-export the existing runtime Base64 decoder through the facade; do not introduce a second codec. Preserve SDK metadata, builtin conversion behavior and database records.

## Impact
The observable property type is corrected. Public Rust constructor inputs remain unchanged. One old fixture uses placeholder text and assumes a String property; prepare its measured correction for approval before changing its expectations.
