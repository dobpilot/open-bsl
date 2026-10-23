# Add configuration reference values

## Why
The reached demo and UNF manager modules call `GetRef` and `EmptyRef` in
their own platform context. A reference has a configuration-specific type;
the facade currently supports only static component type descriptors.

## What Changes
- Add immutable host-declared value types to the existing engine type catalog.
- Allow a component object to report that declared value type through TypeOf.
- Declare a manager's own method context explicitly, using the existing method
  descriptors and method-call path instead of resolving arbitrary unknown calls.
- Implement the native-measured catalog reference creation, identity and UUID
  surface in bsl-orm.

## Impact
The facade and runtime expose additive registration and object hooks.
TypeRef becomes Clone rather than Copy to own dynamic type descriptions.
Existing static type behavior, standalone resolution and crate dependencies
remain unchanged. No database logic or second BSL executor enters the core.
