# Design

The existing SessionResources boolean represents an explicitly supplied host session state. It is not a database metadata property. Native control retained in docs/research/fixtures/dynamic-configuration-session-history establishes old A false → true → true and new B false against the same updated database; it rules out a single acquisition-level Boolean.

The CLI opens a new logical Session after preparing its fixed Factory. Initialize that session's host resource to the measured birth state false immediately before opening it. This value must not be justified by a stable fingerprint, lack of ConfigSave or a guess about database contents. The Session library remains strict when an arbitrary host supplies no state.

The current bool resource cannot observe external updates over time. This change covers startup initialization, not full live dynamic-update compatibility. Such observation would require a separate host contract and its own measurements; no listener, reload operation or second interpreter is introduced here.

Validate the host-resource path through public facade integration tests and repeat the original read-only demo/UNF startup, recording the next terminal result rather than claiming full success from the resource alone.
