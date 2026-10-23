# Tasks

- [x] Validate the change strictly before implementation.
- [x] Add focused tests for an ordinary session, closed connection, principal mismatch and unsupported setters.
- [x] Implement the getter in the existing database adapter.
- [x] Repeat original demo startup with the prepared SDK.
- [x] Run required checks and archive after a green run.

Live own PostgreSQL control on port 55439: both focused tests passed on 2026-10-13. The unimplemented getter was reproduced before implementation. The control server was stopped after testing.

Original demo compiled in 120.11 seconds and passed the getter. Its next failure is DataHashing argument validation; startup success remains unproven.
