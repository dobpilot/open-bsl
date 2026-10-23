# Why
Original buh session initialization reaches its chart-of-accounts manager, whose bare predefined account names are not projected into the existing manager context.

# What Changes
Extend the existing configuration reference and predefined manager property path to charts of accounts. Resolve actual record identities through the current session query adapter.

# Impact
bsl-orm, bsl-config-db, bsl-session; no resolver priority change or new dependency. Account objects, posting, and accounting register operations are outside this change.
