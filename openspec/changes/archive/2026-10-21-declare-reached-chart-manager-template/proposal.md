# Why
Original buh session initialization links its chart-of-accounts manager and encounters an unresolved bare GetTemplate call in a dormant exported function.

# What Changes
Extend the existing finite manager template method context to the reached chart manager. Preserve explicit failure when real template retrieval is unavailable; do not fabricate documents.

# Impact
bsl-orm method declarations and documentation; existing resolver and other manager kinds retain their contracts. Real resource decoding is required if original initialization actually executes this path.
