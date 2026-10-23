# Design

## Scope and evidence
Existing worker_main constructs an engine before drive_local can inspect the queue. The default pool can therefore start many expensive parsers concurrently even when only one worker has useful work. The saved per-thread allocation diagnostics attribute concurrent retained growth to worker catalog construction, rather than to image verification on the main thread. This change extends the existing deferred background-runtime preparation without changing the pool size.

## Ownership and scheduling
Move cold engine ownership into the worker drive loop. A helping caller supplies a borrowed engine; a cold worker starts without one. Construct the existing engine only after a queued job has been claimed, outside the registry mutex, and retain it for that worker loop. Reuse existing build_worker_engine and all image verification. Do not introduce a second parser, cross-thread Engine sharing, a global catalog cache or a new public entry point.

Check runtime and claimed-record state again after construction: cancellation, terminal completion or shutdown during parsing must not be overwritten by a Running transition. Failed construction retains the existing broken-runtime/fail-all policy while preserving an already Closed runtime. No registry lock is held across parsing. A worker's engine must outlive its resident jobs; nested helping uses the borrowed engine supplied by the running job service.

## Verification
Use new external cfg(test) files and test-only gates/counters, not timing sleeps. An idle worker reaches its wait boundary without building a catalog. Two actual jobs on one worker build once. Controlled construction gates cover shutdown and cancellation during construction, failure and retained terminal states. Existing bootstrap, helping, waiting, cancellation and worker-isolation tests remain unchanged. Eager per-instruction cache creation remains unchanged.

## Risks and boundaries
Cold first-job latency moves from idle bootstrap to first useful dispatch. Many workers with actual queued jobs may still construct catalogs concurrently; this change does not serialize useful work or promise bounded memory for all workloads. The original read-only BUH replay under the same ceiling is required before claiming the observed selected-parameter failure resolved. No speed claim is made.
