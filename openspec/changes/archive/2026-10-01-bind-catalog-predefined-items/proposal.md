# Bind acquired catalog predefined items

Original UNF catalog manager source reads the predefined item `Основной` as a bare context property. Configuration managers currently expose methods only, so the unchanged source fails semantic analysis before session initialization.

Acquire the finite predefined-name inventory using the existing open-sdbl decoder, bind it through the existing receiver-property mechanism, and resolve actual references through the current session data adapter. Preserve ordinary variable priority and standalone behavior.
