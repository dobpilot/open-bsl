# Design

StateBuilder installs its per-state EngineJobService when a catalog exists, retaining the Engine, caller token and host profile. It does not invoke Engine::job_runtime. Engine keeps the existing OnceLock and explicit public job_runtime operation.

BackgroundJobService receives a default readiness hook returning true. CallContext::background_jobs checks readiness and maps absence or failed readiness to the existing CapabilityMissing with the same path. The facade service resolves the Engine's shared runtime in readiness and delegates operations to that same runtime. WorkerJobService and external host services use the ready default. No thread-local state and no new serialization mechanism are introduced.

Test that constructing and executing an ordinary catalog state leaves the runtime OnceLock empty; the first capability access initializes it once, including across Engine clones. Retain failure behavior and existing submission, waiting and worker isolation regressions. Do not change existing expectations without the separately required approval.

This correction addresses the observed UNF crash before its session handler. Printing the complete worker image remains expensive on actual first background use and is not claimed solved. Original startup must be rerun with read-only isolation, an address-space ceiling and core dumps disabled after focused checks.

The facade reexports BackgroundJobService and JobWaitOutcome, closing the host-service signatures without requiring bsl-rt names in consumers. A new facade-only consumer test checks these paths.
