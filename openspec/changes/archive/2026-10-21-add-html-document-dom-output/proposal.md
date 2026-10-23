# Add HTML document DOM output

## Why
Original demo compilation now reaches zero-argument `ДокументHTML` in
`Таймлист`. Its authored helper creates a body and text node and writes
the document through `ЗаписьDOM` and `ЗаписьHTML`. The existing XML
component already owns the DOM tree and its writer traversal, but has
no HTML document or writer surface.

## What Changes
- Add the measured zero-argument and title HTML document constructors.
- Support the body and text-node construction used by the original helper.
- Write this DOM through a string-backed HTML writer with native output.
- Preserve XML constructors, tree behavior and serialization.

## Impact
Extend the existing component and its registered type/member tables.
No second DOM, BSL interpreter, SQL knowledge or external dependency is
introduced. HTML parsing and additional DOM behavior remain work to
implement if the original execution reaches them; unsupported operations
must fail explicitly rather than manufacture successful results.
