# Why
Background initialization needs a real parent and a distinct child connection. SessionFactory currently has no public clone/kind builder, forcing hosts to rebuild the identical server program for another runtime kind. Engine already supports shared compiled configuration through Clone.

# What Changes
Implement Clone for SessionFactory and add with_kind(self, SessionKind) -> Self. The builder changes the kind of future sessions and reuses the existing compiled Engine. Preserve existing open/open_background checks and per-Session state/resources.

# Impact
Small additive Rust API in bsl-session, Russian rustdoc and focused isolation/reuse tests. No new dependencies, compiler profile, VM behavior or platform semantics.
