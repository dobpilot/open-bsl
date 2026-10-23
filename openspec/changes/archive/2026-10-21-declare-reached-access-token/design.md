# Design

The whole native control in docs/research/fixtures/access-token-native-1016 confirms Russian/English constructors accept exactly 0 or 2 arguments; 1/3 fail. Two arguments are empty Maps in the control. Vendor ctor337 requires Headers/Payload Maps; ctor338 is default.

Use existing bsl-crypto ConstructorDescriptor and Arity::one_of(&[0,2]), register its TypeDescriptor, and return an explicit unsupported error from the callback. Do not fabricate an object or validate/implement JWT payload behavior. The compiler can retain a dormant constructor call; runtime execution remains intentionally unavailable.

Declare only reached HS256 using existing enum tables: АлгоритмПодписиТокенаДоступа / AccessTokenSignAlgorithm, member HS256, value presentation HS256. Native type of that value is AccessTokenSignAlgorithm and aliases compare equal; ordering fails. A required enum-family metatype presentation must be separately measured before adding its table entry; do not infer an Enum prefix. Other vendor-listed members are outside this change.

Existing local/module precedence and unknown diagnostics are preserved by existing resolver paths. No external dependency, VM-format, SDK, token signing/serialization or authentication change. Native evidence, new external integration tests, focused checks, generated API reference and strict validation provide reviewable proof. No existing test is edited.

The additional independent owned filebase control retains both Russian and English bare-family results: type and presentation are exactly EnumAccessTokenSignAlgorithm. It ran before enum implementation and does not serialize any token.
