# Design

Use bsl-session's existing unsupported platform-constructor pattern. Native 8.3.27.2342 accepts Russian АдминистрированиеСервера and English ServerAdministration with zero, one or two arguments; without a listening ras endpoint creation fails with Connection refused. Three arguments fail with Constructor not found. Preserve the receipts and the syntax-help locator in docs/research/server-administration-constructor-native.md.

The session library already carries reached platform-service constructors such as collaboration services and InternetMailMessage; use its next unused constructor code. No type descriptor is needed until an actual object protocol exists. A dedicated Russian error states that the ras transport is not connected. Do not translate that limitation into native Connection refused: no network attempt occurs in this implementation. Do not broaden deferred compilation errors or alter name resolution.

Verify both names and zero through two arguments compile, three arguments retain semantic arity rejection, an unused constructor in a common module does not prevent a valid export, and actual invocation fails explicitly. Repeat original buh and report the next real outcome. This is an intermediate declaration, not a completed ras implementation or a successful configuration session.
