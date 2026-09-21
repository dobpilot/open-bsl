# Start an isolated configuration session

## Why
A loaded configuration needs a host object that combines the existing open-bsl engine with live metadata, one database connection, a selected information-base user, and the platform startup handler. Running a script alone does not establish this session context.

## What Changes
- Add `bsl-orm` for the measured metadata, role, query, manager, register, and constant surface.
- Add `bsl-session` with `SessionFactory` and `Session`.
- Start each session by invoking `УстановкаПараметровСеанса(Неопределено)` and publish it only after successful startup.
- Add an optional `bsl-cli --config-db` mode while preserving every existing invocation when the option is absent.

## Impact
The new crates compose only public `open-bsl` facade APIs for interpreter access. Database dependencies remain outside `open-bsl`, `bsl-rt`, and `bsl-vm`. The first implementation is limited to the measured runtime surface required by the `demo`, `buh`, and `unf` session startup paths.
