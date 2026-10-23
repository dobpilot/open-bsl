# Design

ClientSettingsAdapter::load_with_context(principal, request, context) defaults to load(principal, request). The standard manager clones the immutable Principal before borrowing the same active CallContext mutably. The adapter is cloned from current SessionRuntime before invoking it, avoiding a host-data borrow across a reentrant parameter initialization. Neither contexts nor mutable values are stored globally. Existing adapters keep their behavior through the default implementation; no thread-local state is introduced.

## Verification

Six client-settings tests pass, including existing default adapters and a new context-aware adapter invoked from two different Sessions. Package all-target Clippy, warning-free rustdoc, workspace formatting and strict change validation pass. Full workspace gates remain pending after this additive API change.
