# Design

The native control in docs/research/fixtures/session-started has seven complete lines on 1C 8.3.27.2342: Date type, nonempty value, SessionStarted alias, repeated-read equality and rejected assignment. The installed vendor help page SessionStarted7561.html confirms the read-only Date contract.

Add optional session_started: BslDate to SessionResources and SessionRuntime, set by with_session_started. The existing current-session object copies that value; its getter does not consult the clock. Missing host data fails explicitly. Background opening takes its own resources and does not inherit this field from the parent.

The CLI is the host of its own session. It captures SystemClock and the default SystemTimeZone immediately before opening it, using the existing local_date_from_utc_seconds conversion exported through open-bsl. This value is a host-assigned start time, not a claim to have acquired a native 1C server session timestamp. No second calendar conversion is written and no new clock configuration is added.
