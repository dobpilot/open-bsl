# Design

The existing UserMessageSink is the boundary for complete owned message DTOs.
Extend that service rather than introducing a second callback or depending
on bsl-session from the runtime. A default retrieval implementation returns
no pending messages so existing sinks and standalone execution keep their
behavior. Reuse UserMessageObject::from_dto and the existing FixedArray value.

A configured Session owns its own pending collection. Read without removal
returns a snapshot; read with removal returns the same snapshot and removes
only those pending entries. The native probe shows that repeated reads share the queued message objects:
mutating a returned object is visible on the next read and on removal. Cache
materialized objects within the Session queue rather than repeatedly cloning
DTOs. Source-object mutation after enqueue does not alter the retained message. Retrieval does not read the database and must not use thread-local state.
A new or background Session begins with an independent collection; parent
pending messages are not copied. Background-job historical messages remain
owned by the existing registry and its retrieval API.

The earlier main-change decision to avoid runtime queue changes applied to
native bytecode Message recovery. Do not alter that recovery or replace the
default immediate-output host. This change explicitly adds the configured
Session host behavior needed by the original CheckFilling control. Host
presentation and pending-message consumption must be distinguished before
wiring CLI output; do not silently drop previously visible startup diagnostics.

Native evidence already proves the reached CheckFilling message text, field,
count and read-with-removal call. Before implementing other observable queue
semantics, measure repeated reads, removal, source mutation and Session
boundaries on the platform. Do not derive an oracle from open-bsl. Do not
invent queue size or delivery policy as platform semantics: reuse existing
host resource limits, or record a concrete unresolved host-policy question.

## Native evidence, 2026-10-17

`docs/research/fixtures/session-pending-user-messages/measure.platform.txt`
was collected on 1C 8.3.27.2342 in an isolated temporary file infobase.
Default and explicit false reads both retain one entry. Source mutation keeps
its original text/field; returned-object mutation persists through later reads.
Read-with-removal returns that object and the next read is empty.
The Message method uses its English alias deliberately: the repository runner
rewrites Russian Сообщить calls for receipt output, including qualified calls.

## Configured host presentation policy

Configured Sessions retain pending messages and also echo their text to the
existing State stdout stream as host diagnostics. This preserves visible CLI
startup diagnostics. Echo is not acknowledgement of the platform-style
pending collection; only GetUserMessages(true) consumes it. The host service
opts into this diagnostic echo explicitly, with a false default for existing
sinks and the standalone PendingUserMessages collector. No job-history sink
is changed. This is an explicit host presentation policy, not a claim about
when a graphical 1C client displays server messages.
