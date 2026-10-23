# Evidence
Native1C8.3.27.2342 controls are preserved in the catalog-find-by-code and catalog-find-by-description fixture directories. Original demo has single-argument code calls and an exact description call. These are static calls, not proof of startup reachability.

# Mechanism
Build a parameterized query through the existing QueryRequest and DataAccess::execute_query path, preserving the current principal, lazy parameters, rights and RLS. Do not add a separate database executor or broaden the own-attribute lookup contract. Known standard settings are required; unknown settings and unimplemented modes return explicit errors.

Native controls distinguish case and leading spaces while ignoring trailing spaces. Database collations can admit extra candidates, so select the reference and searched field and validate candidates using the measured exact or prefix comparison in the host. Do not apply TOP1 before candidate validation; otherwise an admitted wrong-case candidate could hide a valid one. Existing adapter query limits continue to bound results. Keep values in parameters and never interpolate caller input into query text. Validate returned reference ownership and catalog identity as the existing attribute search does.

Preserve existing same-named manager exports using the existing export-priority mechanism. Bare receiver calls continue through the ordinary resolver and variable precedence is unchanged. Unmeasured modes remain unsupported and are not presented as platform outcomes.
