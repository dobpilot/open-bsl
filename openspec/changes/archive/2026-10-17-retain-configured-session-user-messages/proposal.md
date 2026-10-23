# Retain pending user messages in configured Sessions

## Why
The measured business-process CheckFilling control reads one pending message,
including its attribute field, through GetUserMessages(true). The current ORM
emits the correct DTO, but the base runtime deliberately returns an empty
array because its default host displays messages immediately. Reconstructing
the receipt from an external test sink does not execute the original control.

## What changes
Add an optional pending-message retrieval operation to the existing host
message service, with an empty default preserving existing hosts. Configure
an isolated pending-message service for each bsl-session Session and use the
existing DTO materializer when BSL reads pending messages. Preserve the
standalone immediate-output path and the separate background-job history.

## Impact
A public additive Rust host-service method and configured-Session runtime
behavior change. No new dependencies, database access, interpreter, or global
thread storage. This is required to complete the existing measured
business-process CheckFilling requirement; the main change remains open.
