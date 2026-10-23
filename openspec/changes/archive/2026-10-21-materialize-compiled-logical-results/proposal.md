# Why
Original demo and UNF session initialization both reach the Boolean|String constant ПрофильБезопасностиИнформационнойБазы. The database compiler emits three physical cells; the adapter incorrectly attempts to expose each as a separate BSL value and rejects the one-byte discriminator.

# What Changes
Consume the SDK's complete logical result projection to produce one BSL column per selected value. Decode only the selected active member of composite values. Preserve scalar conversion, parameter handling, rights and data separation. Do not infer grouping from SQL labels or field origin.

# Impact
Database data-access adapter and focused integration tests. SDK supplies structural projection; BSL value construction stays in the existing consumer. No language syntax, database dependencies in runtime/VM, or new external crates.
