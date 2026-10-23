# Design

Full vendor help `platform-syntax-help:syntax-context:objects/catalog125/catalog126/object130/properties/DeletionMark1615.html` describes a read/write boolean standard property. The actual UNF failure is preserved in `/tmp/open-bsl-unf-folder-flag-095.log`.

Reuse the existing CatalogObject representation, finite property table and receiver context. Store the value on the concrete object. Before code, measure initial state, body availability, supported assignments and conversion failures. Reuse existing runtime conversion if it matches the measurement; do not implement another boolean conversion algorithm.

Changing the property stages object state and does not implicitly implement database persistence. The separate SetDeletionMark method, subordinate records and record writes remain outside this change until reached and measured.

Native 8.3.27.2342 produced 24 rows in `measure-catalog-deletion-mark.platform.txt`: reads return false in the body, body writes are rejected, later aliases address the same mutable value and a second object remains false. Booleans and numbers use the existing Boolean builtin conversion. Recognized strings include Истина, True and Yes with whitespace trimming; Да remains false. Unsupported values become false.

The existing table-column conversion does not recognize strings and therefore cannot be reused here. Lift the existing private Boolean builtin converter to `convert_to_boolean(&Value) -> RtResult<bool>` and export it through the facade. Preserve its recognized words and errors unchanged; the builtin wraps its result as Value::Boolean. The property alone defaults failed conversions to false after the selected-object guard. This avoids a second parser and preserves existing table and builtin semantics.
