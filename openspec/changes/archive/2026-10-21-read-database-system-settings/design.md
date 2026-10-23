# Design
Share the existing settings read path. Choose the SDK Common or System compiler from the requested storage; derive separation scope from that storage's acquired table. Never substitute the Common table for System. Reuse principal checks, read-only per-session connection, payload cardinality checks and bounded decoder. Other stores and operations retain explicit errors.

Resolve global separation conditions before use/value parameters. A disabled condition selects the canonical SDK scope without initializing either parameter. Inactive scope does not read the value parameter. The SDK independently checks complete physical scope. Returned reference values use the active configuration catalog and runtime context.

Native evidence is retained in docs/research/fixtures/system-settings-native-1015: String Independent root/A/B reads, distinct Common/System values under identical keys, missing key, explicit current user, ValueStorage(Structure), and Numeric Shared conditional scopes. Numeric false/0 and true/0 remain distinct; no unsupported profile is inferred. PostgreSQL is the only supported provider for this path. Acquisition already supplies System schema, as confirmed by the UNF snapshot receipt.

The adapter integration remains in the isolated consumer checkout until its prepared SDK stack is reviewable. SDK 0.9.17 supplies compile_system_settings_load; main SDK is not silently replaced. Original UNF replay is a separate acceptance check and must retain its actual terminal result.
