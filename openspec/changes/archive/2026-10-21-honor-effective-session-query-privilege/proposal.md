# Why
Original UNF explicitly enables privileged mode before reading extension session settings. The database adapter applies ordinary field and row access decisions without consulting the effective frame mode, so the reached query is refused.

# What Changes
Honor the existing CallContext effective privileged mode when applying query access policy. Keep ordinary session access and safe-mode suppression intact.

# Impact
bsl-config-db policy integration only. No roles, user identity, SQL write access, resolver priorities or global state are changed.
