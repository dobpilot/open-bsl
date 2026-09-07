# Архитектура

Здесь лежат действующие решения о границах и владении компонентами.

- [`component-architecture.md`](component-architecture.md) — нормативная
  топология конфигурационного каталога, runtime-компонентов и фоновых заданий.
- [`rust-module-boundaries.md`](rust-module-boundaries.md) — текущие
  внутренние владельцы операций runtime, фоновых заданий и периферии VM.
- [`vm-code-layout.md`](vm-code-layout.md) — измеренные ограничения на
  разбиение `bsl-vm` и размещение кэшей исполнения.

Проекты уже завершённых перестроений находятся в
[`../archive/refactors/`](../archive/refactors/).
