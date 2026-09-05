# Активные планы

- [Разделение обязанностей Rust-модулей](../../openspec/changes/separate-rust-module-responsibilities/proposal.md)
  — предложенный OpenSpec-план по результатам аудита: runtime, фоновые
  задания и продолжение принятого рефакторинга VM.
- [`standard-library.md`](standard-library.md) — покрытие главы 16 стандартной
  библиотеки.
- [`http-client.md`](http-client.md) — совместимость HTTP-клиента Connector.
- [`bytecode-hardening.md`](bytecode-hardening.md) — закрытие оставшихся
  невозможных состояний и служебных типов байткода.
- [`bsl-vm-refactor.md`](bsl-vm-refactor.md) — расслоение монолита
  `bsl-vm/src/lib.rs`: вынос кэшей из `Chunk` и холодной периферии в модули.

Измерительный план SSA и оптимизаций находится в
[`../research/performance/ssa-hotspot-analysis.md`](../research/performance/ssa-hotspot-analysis.md),
поскольку его решения принимаются по данным профилирования.
