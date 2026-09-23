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
- Provide the client settings-storage and home-page surface, including the standard form-data store, through an adapter explicitly attached to the current Session.
- Expose the platform's global XDTO serializer through the existing XML/XDTO component surface.
- Execute data reads under the current principal's rights and RLS.
- Start background jobs as separate sessions for an explicitly selected principal while copying the parent session parameters.
- Expose the frozen current principal through the information-base users manager.
- Provide the measured data-item send and receive system enumerations reached by configuration startup.
- Route metadata access-right checks through the current Session's rights-aware adapter.
- Expose the frozen Session principal name through the standard global function.
- Support the measured mutable-map constructor from a fixed map reached by startup code.

## Impact
The bridge extends the public open-bsl facade and existing call context but reuses the current compiler, VM, scheduler, State, and module instances. It adds no database knowledge to the interpreter and no second BSL dispatcher.

`МенеджерПользователейИнформационнойБазы.ТекущийПользователь()` is included because the recorded UNF startup graph reaches it. The returned description is backed by the principal already frozen for that Session; the change does not add user-catalog mutation or cross-session lookup.
