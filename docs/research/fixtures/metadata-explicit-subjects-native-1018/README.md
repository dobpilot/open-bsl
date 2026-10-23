# Explicit subjects of metadata AccessRight

Platform8.3.27.2342, owned copied file infobase only. The matrix retains120
observations: two authenticated users, two privilege modes, five subjects,
three standard-attribute paths and Read/View. Role names do not stand for their
grants; effective native CF/XML is retained for exact decoding.

The measured current-principal check may bypass grants in privileged mode.
Explicit role and explicit user checks still return that subject's role grants;
privilege does not turn their denied grants into success. An explicit user
description requires Administration in ordinary mode, including the current
user's own description. Inapplicable Read/standard pairs fail before this
administrative check in the retained controls. These observations do not infer
other rights, metadata kinds or malformed-subject error priority.

The first harness attempt unconditionally called SetPrivilegedMode(False) in a
function that had not enabled it. It never reached the matrix and remains in
`before-privileged-probe-correction/`. The corrected helper enables/disables only
when requested; both60-row executions complete. Native measurements alone do not establish runtime completion; the subsequent
host integration is recorded below.

`results.json`, original `.oracle` files, native modules, CF/XML, source hashes
and process/export receipts retain the evidence. Full Syntax Assistant page
opening was unavailable through the current connector behavior; see
`help-boundary.json`. A search snippet was not used as a behavioral oracle.


## Host integration

OpenSpec was extended and strictly validated before execution-code changes.
The shared configuration-root subject-role selection now also serves measured
catalog Read/View. Only implicit-current-principal grants are bypassed in
privileged mode; explicit users retain their Administration prerequisite in
ordinary frames, with applicability checked first. Role resources remain a
loaded immutable snapshot; missing roles fail explicitly and no implicit fetch
or second cache is introduced.

The external test replays all120 rows through BSL/Session and the shared
production helper. It also checks missing role rights under privilege and an
unacquired configuration root. Its user identities are explicitly model values;
rights and catalog descriptors come from retained native CF bytes, with the
existing explicitly synthetic owner/kind index recorded separately. The prior
492-row matrix and root-right checks remain green (8 verification tests and
4 root-right tests). Initial fixture-kind preparation failures and the genuine
pre-implementation unsupported-subject failure remain separately retained.

`actual-database/` then restores the actual native DT, acquires real users,
metadata and role resources, and replays all120 rows through DatabaseDataAccess
with separate PostgreSQL connections. A third Session omits the requested role
resource and confirms explicit failure under privilege. The database is read-only
during probing and temporary storage is removed. An empty diagnostic session
initializer is added because the measurement configuration has none.

Focused/actual controls pass; full consumer gates are running separately in
`root-gates/`. The broad metadata-right requirement and main change remain open
for kinds/rights outside delivered coverage. The existing SDK0.9.23 CLI startup
candidate is pinned before this host extension, so its ongoing receipt must not
be presented as a production startup of this later source state.
