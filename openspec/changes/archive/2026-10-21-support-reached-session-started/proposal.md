# Support reached current session start time

## Why
Original demo now finds its existing extension version and reaches InfoBaseSession.SessionStarted, which is absent. Native controls establish a stable read-only Date property and its aliases.

## What Changes
Allow the host to provide a start Date with SessionResources and expose it from the existing current-session object. The configured CLI captures its host session start immediately before opening the Session. Reuse the existing local date conversion through the facade.

## Impact
One optional Rust builder method and one BSL property are added. No database telemetry is invented, no thread-local state or new dependency is introduced, and standalone CLI behavior remains unchanged.
