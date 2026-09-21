# Complete configuration session runtime dispatch

## Why
Full session-parameter initialization calls manager modules, performs live queries under the selected user's rights, and recursively initializes unset session parameters. Component callbacks currently cannot enter a linked BSL module in the same State, so these paths cannot preserve the platform session topology.

## What Changes
- Add one facade-level bridge from a component operation to an export of an existing BSL module instance in the current State.
- Combine platform manager methods with exports of the linked BSL manager module.
- Implement the measured lazy session-parameter state machine.
- Provide the measured stored-value objects used by managed-client startup code.
- Expose the current information-base session and its explicit background-job identity.
- Forward configuration event-log records to a writer owned by the current Session.
- Provide the client settings-storage and home-page surface through an adapter explicitly attached to the current Session.
- Execute data reads under the current principal's rights and RLS.
- Start background jobs as separate sessions for an explicitly selected principal while copying the parent session parameters.

## Impact
The bridge extends the public open-bsl facade and existing call context but reuses the current compiler, VM, scheduler, State, and module instances. It adds no database knowledge to the interpreter and no second BSL dispatcher.

`МенеджерПользователейИнформационнойБазы.ТекущийПользователь()` is not part of this change. Its syntax-helper description remains research input and does not authorize an implementation task.
