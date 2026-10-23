# Why
Original BUH compilation reaches SpreadsheetDocumentDetailUse in the PaymentCalendar manager module. Its sources use WithoutProcessing and Cell, missing from the runtime tables.

# What Changes
Add the two reached values through existing enumeration and type tables, with native names and presentations. No spreadsheet event processing or new resolver mechanism is added.

# Impact
Base runtime tables, generated API reference and native focused tests. No external dependencies or VM opcodes change.
