# Tasks

- [x] 1. Validate this change strictly before Rust implementation and retain a pre-fix behavioral failure for a valid attribute target.
- [x] 2. Preserve immutable catalog/owner provenance in attribute wrappers without changing existing reads or identity.
- [x] 3. Document and expose the optional attribute descriptor on AccessRightRequest; forward both operations and original metadata values.
- [x] 4. Verify current/explicit subject forwarding, foreign-catalog rejection, unchanged parent requests and attribute identity/name behavior.
- [x] 5. Prove the current database adapter explicitly rejects child Read instead of using the owner shortcut, in ordinary and privileged modes.
- [x] 6. Pass focused checks, formatting, workspace Clippy/tests/rustdoc, facade consumer, unchanged RT/VM normal dependencies and strict OpenSpec validation; retain broad SDK gaps.
