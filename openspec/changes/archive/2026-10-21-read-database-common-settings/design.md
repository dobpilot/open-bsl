# Design

DatabaseDataAccess implements ClientSettingsAdapter, sharing its existing per-Session connection and principal checks. Its context-free load fails explicitly; load_with_context accepts only Common storage and the current principal's exact name. Other-user access remains explicitly unsupported instead of bypassing rights. The reached scope supports measured Independent and IndependentAndShared string/numeric separators. Resolve a bound global condition with the existing constant reader before requesting use/value parameters; false selects SDK ConditionDisabled and skips both parameters. With no condition or true, resolve the use flag, then resolve the typed value only when active. Unsupported profiles fail before SQL. Resolve only fields present in the acquired common-settings table. False use flags do not read value parameters. SQL compilation in the SDK independently verifies complete physical coverage.

Execute the existing read-only query path with a two-row bound; zero means absence, one requires one binary payload, two means ambiguity. Decode with SDK decode_common_setting using the existing eight MiB materialization budget. Reuse value_from_string_internal_with_resolver or value_storage_from_internal_with_resolver with the existing reference resolver. No thread-local cache, inferred empty data or new serialization engine is introduced. The returned values belong to the active context.

Native evidence: own scoped SDK results and native root/A/B settings under docs/research/fixtures/user-settings-native-1015. Tests must cover the final BSL boundary and explicit unsupported operations; original demo success remains a separate gate.

The adapter Debug implementation exposes only its type name, excluding the connection, principal and settings contents.

Native numeric controls confirm canonical true/0 and no parameter reads under false global conditions in both modes. Existing string controls confirm true/empty string. The SDK scope enum owns typed-empty construction; the adapter does not duplicate it.
