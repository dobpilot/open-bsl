# Read reached database common settings

## Why
Original demo session initialization reaches CommonSettingsStorage.Load. The SQL and byte codecs are now measured and available in the SDK draft, but the database adapter does not connect them to the active Session.

## What Changes
Implement current-user Common settings loads on DatabaseDataAccess through the context-aware callback. Resolve reached string separation parameters through the current Session, compile SQL with the SDK, reject duplicate rows and materialize through existing value/storage codecs. Unsupported stores, writes and unmeasured scopes fail explicitly.

## Impact
Database-backed adapter implementation; no database writes, new BSL members or dependencies. Main integration waits for the prepared SDK APIs.
