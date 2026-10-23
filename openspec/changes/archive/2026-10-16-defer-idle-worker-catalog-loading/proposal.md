# Defer idle worker catalog loading

## Why
The original BUH startup succeeds, but a separate selected-parameter check exhausts the unchanged address-space ceiling while idle background workers concurrently construct independent configuration catalogs. Per-thread diagnostics show that the main verification thread does not retain the observed memory. An idle worker must not parse the complete worker recipe before it has work.

## What Changes
Keep the configured operating-system thread count and bootstrap behavior. Defer each worker's existing catalog construction until it claims its first queued job, then reuse that engine for subsequent jobs on the same thread. Helping execution continues to borrow its existing engine. Preserve verification, per-instruction eager caches, isolation, cancellation, shutdown and broken-runtime failure handling.

## Impact
Internal open-bsl worker resource management only. No public API, bytecode format, SDK, dependency or BSL platform-semantic change. Original startup and selected initialization remain separate validation outcomes; this proposal does not claim a successful replay before measurement.
