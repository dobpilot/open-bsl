# Defer background runtime preparation

## Why
Original UNF compiles, then StateBuilder::build exhausts memory while printing the entire configuration into the background worker recipe, before the session initialization handler executes. The retained crash stack identifies write_image through jobs::prepare::runtime_for_engine. A state that has not requested background services must not pay this preparation cost.

## What Changes
Prepare the existing shared background runtime at the first request for its capability, rather than during State construction. Keep the existing worker recipe, shared registry, host profiles, session tokens, execution and explicit Engine::job_runtime entry point. Introduce a default readiness hook on BackgroundJobService so existing external implementations remain ready without changes; failed preparation retains CapabilityMissing at CallContext access.

## Impact
A narrow runtime initialization correction in bsl-rt and open-bsl, with no new dependency, BSL method, serialized format or configuration object implementation. It does not remove the remaining size cost if a program actually requests the background capability.
