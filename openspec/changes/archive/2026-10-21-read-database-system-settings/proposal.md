# Why
Original UNF session initialization reaches SystemSettingsStorage.Load after its root access-right check succeeds. The database adapter currently rejects this store, although the acquired snapshot includes its schema.

# What Changes
Read the reached System settings through the SDK scoped SQL compiler and existing bounded settings decoder. Preserve current-principal checks, active session context and Common settings behavior.

# Impact
Database settings adapter and integration checks. No new dependencies, database writes or language resolver changes.
