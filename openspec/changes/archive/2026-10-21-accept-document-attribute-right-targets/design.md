# Design

Document attribute wrappers already implement the metadata value protocol but hold only the descriptor array and index. Add immutable catalog and owner provenance to the private collection/attribute wrappers, keeping the existing property and identity behavior. Validate catalog provenance using the same catalog identity rule as existing MetadataValue targets; this is a host isolation invariant, not a new inferred platform rule.

Add AccessRightRequest::attribute() -> Option<&DocumentAttribute>. metadata() identifies the owner for attribute requests; metadata_value() remains the original child BSL value. Existing owner/root requests return None. The acquired descriptor supplies the actual attribute UUID and name, avoiding a new MetadataKind variant, fabricated member paths or another rights interpreter. Both global operations use the existing adapter entry points and preserve explicit subjects/optional standard paths.

The current database adapter has a Document owner-Read shortcut. It must explicitly reject a request with an attribute descriptor before this shortcut; otherwise native-inapplicable attribute Read would become an owner grant. SDK support will replace that explicit unsupported branch in a separately assigned change. This host change does not declare current database attribute checks successful or implement applicability/grant policy locally.

The immutable catalog and descriptors are shared; wrappers do not retain mutable State, principal, connection or thread-local data. Existing attribute property reads and semantic identity remain stable. Original metadata values are retained for journal payloads. No BSL descriptor-table or bytecode-format change is required.

Native observations remain the authority: ordinary attribute Read is inapplicable for AccessRight and succeeds for VerifyAccessRights, while View depends on acquired parent/child policy. These observations motivate delegation; they are not hard-coded as a second host evaluator. No new unmeasured platform decision is introduced.
