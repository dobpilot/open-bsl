# Accept acquired document attributes as rights targets

## Why
Native Document controls retain144 baseline and288 role-profile calls. Ordinary attribute metadata is a valid AccessRight/VerifyAccessRights target. The host returns such metadata through its existing document attribute collection, but access_right_metadata rejects that representation before invoking the data adapter. The collection also lacks the catalog/owner provenance needed to validate the target.

## What Changes
- Preserve catalog and owning-document identity in existing immutable attribute wrappers.
- Forward an acquired document attribute through the existing AccessRightRequest, exposing its descriptor with an additive Rust accessor.
- Preserve the original BSL metadata value for adapter diagnostics/journal data.
- Keep the current database adapter explicit about unsupported child checks, preventing the owner-Read shortcut from granting attribute Read.

## Impact
No new BSL names, syntax, arities, database queries, rights evaluator or external dependencies. Existing owner/root checks retain their behavior. Actual acquired role evaluation remains an SDK task; this change supplies the target identity and safe unsupported behavior. The broad configuration-session change remains incomplete.

Evidence: docs/research/fixtures/document-right-applicability-1019/analysis.json and docs/research/fixtures/document-right-profiles-1019/analysis.json.
