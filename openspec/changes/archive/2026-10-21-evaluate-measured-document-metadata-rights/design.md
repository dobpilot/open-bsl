# Design

Use resolve_metadata_right for Document owner/Number/Date and resolve_document_attribute_right for an acquired ordinary own attribute. The latter requires complete acquired membership plus a unique child descriptor. Preserve the original AccessRightRequest metadata value for journal payloads.

Resolve structure and applicability before privileged bypass. NotApplicable maps to an AccessRight error and successful VerifyAccessRights; it must not become an owner grant. For Applicable targets, use existing subject selection and SDK AccessStore role evaluation, including measured verification-only independent-child behavior. Catalog defaults guards stay unchanged.

Native evidence: document-right-applicability-1019 and document-right-profiles-1019. Reuse exact role resources and acquired Document descriptor in host replay. Synthetic DBNames entries are an explicit test index, not a claimed Config extraction. SDK source and final delivery require independent review before integration.

This does not expand query field rights, RLS, document writes, child kinds or rights beyond retained controls. Explicit role/user authorization and denial writer failure propagation retain their existing contracts. Missing/foreign/ambiguous acquired targets fail before privilege handling and do not create denial journal entries merely because structure is unsupported.
