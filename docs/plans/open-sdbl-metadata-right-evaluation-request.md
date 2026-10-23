# Evaluate metadata rights without conflating denial and inapplicability

## Status and boundary

Prepared request; no SDK code changed. Agent assignment and release version are
unassigned. The inspected SDK is 0.9.21, branch
`feat/extension-purpose-0.9.15`, HEAD `305f4f1`, with the existing uncommitted
SDK stack. Use that available working-tree state, not HEAD alone; preserve
other changes and do not roll back or commit the pre-existing stack as part
of this request. This is metadata-right evaluation for
AccessRight/VerifyAccessRights, not SQL/RLS field-access checking. Keep the
existing query authorization path and its measured programmatic Read rules.
Do not add database knowledge to bsl-rt/bsl-vm or a second rights interpreter
to open-bsl.

## Native evidence

1C 8.3.27.2342, compatibility 8.3.27, owned filebase only:
`docs/research/fixtures/verify-access-rights-native-1017/`.

- `standard-paths/`: two authenticated users, Read/View, undefined/empty,
  Russian Code, English Code/Description and an unknown standard name.
- `standard-privileged/`: the same matrix under effective privileged mode,
  including UnknownNativeRight. Unknown right names, empty/unknown standard
  paths and right applicability are still validated before grants are bypassed.
- `standard-profiles/`: eight single-role View/Code profiles, native CF,
  input role XML, extracted compiled rights and results. Profile flags include
  defaults and independent child rights. No outcomes come from open-bsl.
- `standard-description-profiles/`: explicit Description grant/refusal with
  opposite defaults, plus Russian Description and differently cased Code names.

Observed distinction: valid standard Code/Description with Read is **not
applicable**. AccessRight raises a cannot-apply error, while VerifyAccessRights
returns successfully, even when owner Read is refused. Both reject empty or
unknown paths. View is applicable and follows the retained role profiles.
A bool cannot distinguish these cases.

The compiled member keys are pairs, not GUIDs alone. In the measured catalog:
Code is `(-2, 03f171e8-326f-41c6-9fa5-932a0b12cddf)`, Description is
`(-3, 03f171e8-326f-41c6-9fa5-932a0b12cddf)`. The same GUID occurs in both.
Do not infer other kinds from these two negative markers without measurement.
The native CF uses `<role-guid>.0`; the earlier `.7` assumption and its failed
extraction are retained, not treated as a platform failure.

Native import materializes owner Read=true when owner View=true, although the
input XML explicitly refused Read. Use the canonical native compiled resource;
do not invent a second implication pass from the XML. Explicit child refusal
with attribute defaults enabled is retained; a missing child uses the measured
default. The complete observation index is `standard-right-results.json`.

## Requested API shape

Reuse RoleRights/ObjectRights/ObjectMember and the acquired metadata/role cache.
Provide a pure structural resolution stage before any privileged bypass, and a
cached role-evaluation stage. One concrete proposed shape is:

```rust
pub struct ResolvedMetadataRight { /* private */ }
pub enum MetadataRightApplicability { Applicable, NotApplicable }

pub fn resolve_metadata_right(
    snapshot: &MetadataSnapshot,
    owner: &Guid,
    right_name: &str,
    standard_attribute: Option<&str>,
) -> Result<ResolvedMetadataRight, MetadataRightError>;

impl ResolvedMetadataRight {
    pub fn applicability(&self) -> MetadataRightApplicability;
    pub fn right(&self) -> &Right;
}
impl AccessStore {
    pub fn metadata_right(
        &self,
        roles: &[Guid],
        target: &ResolvedMetadataRight,
    ) -> Result<bool, DbError>;
}
```

Equivalent reuse of an existing API is preferable if it preserves this exact
separation. Resolution must validate known right names, owner and standard
path/existence, preserve native case-insensitive bilingual aliases, and return
NotApplicable only for a **valid** target/right combination. Invalid input,
unknown/missing metadata, incomplete rights and unmeasured unsupported targets
must not become either Granted or NotApplicable. A resolved applicable target
can skip role evaluation when the caller's effective privileged mode permits
it; ordinary callers evaluate the already acquired role set. The SDK must not
know CallContext, State, VM frames, event writers or BSL exception handlers.

The first required native cases are catalog owner Read/View and standard
Code/Description Read/View. Preserve existing owner-check behavior elsewhere;
document unsupported cases explicitly. Do not claim that these fixtures prove
all platform rights or metadata kinds. Three measured two-role Code/View combinations are retained in
`verify-access-rights-native-1017/multiple-roles/`: split parent/child grants
across roles remain denied, a complete second-role grant is allowed, and
two child refusals remain denied. Apply the measured parent gating inside
each role before combining its result; unioning parent and child grants
first contradicts the native control. Other multiple-role combinations and
branches outside these profiles still require native controls. Reuse the supplied owned fixture/runner or
create an isolated copy; never change DEMO/BUH/UNF.

## Consumer mapping and acceptance

open-bsl owns the two API policies: AccessRight reports NotApplicable as a
catchable applicability error; VerifyAccessRights treats it as successful
validation. An applicable refusal remains false for AccessRight and becomes
a catchable error plus the current Session's native-shaped independent journal
entry for VerifyAccessRights. Do not put logging or source rewriting in SDK.
The host will remove its unvalidated non-root privileged shortcut only through
its own OpenSpec/test workflow; it needs the SDK structural stage first.

Required SDK proof:

1. Replay every retained native row for the supported cases, distinguishing
   allowed, refused, inapplicable, invalid name/path and privileged validation.
2. Use the native CF rights resources; verify member pairs and role defaults,
   explicit refusals and parent gating. Do not compare member GUID alone.
3. Missing/unreadable rights and unsupported metadata remain explicit errors.
4. Keep ordinary SQL/RLS authorization regression tests unchanged.
5. No new runtime, connection, per-Session fingerprint calculation, mutable
   cross-user cache, SQL writes or external dependencies are required.
6. Specify the public API in SDK OpenSpec before implementation, strictly
   validate, and run fmt, Clippy, full tests, rustdoc and OpenSpec checks.
7. Return exact API signatures and the commit/branch for the host integration;
   assign the release version only after the user's instruction.

## Delivered correction, 2026-10-18

SDK0.9.22 is transferred to canonical. Its 492-row replay exposed two independent-child verification outcomes overlooked in the earlier prose: independent=true/defaults=false/owner View=false/Code View=true refuses AccessRight but allows VerifyAccessRights. Consumers must select metadata_right versus metadata_right_for_verification after common resolve_metadata_right. The broad statement about independent flags above is superseded by these exact native observations. Other unsupported combinations remain explicit errors. SDK evidence: `docs/research/fixtures/open-sdbl-metadata-right-evaluation-1018/`. Host integration remains open.

## Current consumer boundary, 2026-10-19

SDK0.9.25 retains the delivered structural/evaluation API. Consumer integration of the measured catalog/standard-attribute and explicit-subject paths is completed; the preceding “Host integration remains open” describes the earlier checkpoint, not the current state. Actual Document owner Read and journal payload transfer are also verified through the existing owner-grant path. Evidence: `docs/research/fixtures/document-owner-read-right-1019/` and `docs/research/fixtures/session-instance-value-names-1019/actual-journal/verification/`.

The broad specification is still not complete. `resolve_metadata_right` currently limits structural applicability to Catalog Read/View and measured Code/Description paths. Other owner kinds, rights and paths cannot be certified by the completed catalog replay or replaced with a host-side guess. Further expansion requires its own native coverage and an explicitly assigned SDK request/version; this note does not authorize another agent or release. Source revalidated at SHA-256 `f090e04fd24a32aed4fe0954451c0fdb15aeae896d9127da79207d1e8e7af235` (`../open-sdbl/src/metadata/metadata_right.rs`).

## Additional Document applicability evidence, 2026-10-19

`docs/research/fixtures/document-right-applicability-1019/analysis.json` retains144 actual native calls on Document owner, Number/Date paths and ordinary Attribute metadata across three users and privilege modes, plus the native CF/document XML. Read on the standard/attribute targets is inapplicable for AccessRight but VerifyAccessRights succeeds even with privilege; View follows the observed refusals/privileged grants. Russian/English standard names match. This batch does not establish ordinary positive View or explicit child overrides; measure those before certifying their SDK evaluation. Current SDK comparison and release/agent assignment remain pending. Do not copy Catalog member identifiers or use this metadata applicability rule as SQL/RLS field authorization.

## Document View profile completion, 2026-10-19

`docs/research/fixtures/document-right-profiles-1019/analysis.json` adds288 native calls across six profiles with input XML, platform-canonical XML, native CF and extracted role resources. It supplies the ordinary positive/child-refusal controls missing from the preceding batch. Number uses(-2,03f171e8-326f-41c6-9fa5-932a0b12cddf), Date uses(-3,the same GUID); ordinary Attribute uses its own UUID rights object, not a fabricated standard-member path. Existing SDK parse_role_rights matches canonical flags and native owner grants. Native independent-child/default-true without owner View also permits VerifyAccessRights; do not certify this from the current generic unsupported guard. Resolver and actual Session adapter integration remain pending, as do a new SDK assignment/version. These data do not generalize to other metadata classes or multiple-role combinations.

## Host attribute target prepared, 2026-10-19

The active host change `accept-document-attribute-right-targets` now retains catalog/owner provenance in ordinary Document attribute metadata wrappers. `AccessRightRequest::metadata()` is the owning Document, `attribute() -> Option<&DocumentAttribute>` is the acquired child descriptor (including its UUID), and `metadata_value()` remains the original child BSL value. Both rights operations preserve the existing explicit-subject/standard-path contracts. Foreign catalogs are rejected before the adapter, even for equal fingerprints.

Until the SDK resolves these measured targets, the database adapter explicitly rejects an attribute request before its owner Read shortcut, including in privileged frames. This is an integration preparation, not completed Document applicability/evaluation. A future SDK request should accept the acquired child's own UUID separately from Number/Date standard paths; it must not fabricate a `(0, attribute UUID)` member on the owning Document. Evidence and current verification: `docs/research/fixtures/document-attribute-right-targets-1019/`. No SDK version or agent is assigned by this note.
