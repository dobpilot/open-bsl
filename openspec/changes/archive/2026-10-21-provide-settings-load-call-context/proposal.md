# Provide the active call context to settings reads

## Why
A database settings adapter must initialize and read the active Session's separation parameters and materialize its stored values. The current callback receives only the principal and request.

## What Changes
Add a context-aware load callback with a default delegation to the existing callback. Route standard settings Load through it with the current call context. Preserve existing adapters and absent-provider behavior.

## Impact
Additive Rust adapter API in bsl-session; no new BSL members, dependencies or platform semantics.
