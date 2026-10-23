# Declare the reached data processor manager template method

## Why
Original UNF DataProcessor.НастройкиПользователей.ManagerModule contains a bare ПолучитьМакет("МакетОтчета") in an exported report-building function. Startup linking rejects the original source because the processor manager has no declared method context.

## What Changes
Declare the platform-defined ПолучитьМакет / GetTemplate method for data processor managers with one required argument and reuse existing receiver method contexts. Execution remains an explicit unavailable-template error until actual template resources are supported; no empty document is fabricated.

## Impact
bsl-orm manager descriptors and generated BSL reference. No core dependency, new interpreter mechanism or source rewriting.
