# Add catalog object predefined data name

## Why
Original UNF Catalog.ВидыКонтактнойИнформации.ObjectModule reads bare ИмяПредопределенныхДанных; the receiver lacks this standard property.

## What Changes
Measure and expose ИмяПредопределенныхДанных / PredefinedDataName on the existing concrete catalog object. Preserve variable priority and native initial, body-access, assignment and invalid-name behavior. Reuse acquired predefined names; do not reread metadata or imply database persistence.
