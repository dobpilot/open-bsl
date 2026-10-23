# Create catalog object instances

## Why
Original configuration manager modules call CreateItem. The existing ORM supports
catalog references but cannot construct the mutable catalog object that native
1C creates without writing a database row. Native measurements also show that
two objects of one catalog have independent object-module variables.

## What Changes
- Carry acquired catalog standard-attribute settings and object-module bindings
  into the immutable ORM catalog, without assuming defaults for unknown settings.
- Implement CreateItem and the measured new-object surface through the existing
  component object protocol and owned configuration value types.
- Execute object-module methods through the existing VM, with memory belonging
  to the concrete object instead of one shared instance per module identifier.
- Preserve session-shared common modules and the existing manager call path.

## Impact
Public Rust metadata and instance APIs, registered catalog manager and object BSL
members, the configuration source adapter and VM module invocation. No new
dependencies in the runtime or VM, no database write implied by creation.
