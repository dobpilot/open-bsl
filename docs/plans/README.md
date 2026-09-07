# Активные планы

- [`standard-library.md`](standard-library.md) — покрытие главы 16 стандартной
  библиотеки.
- [`http-client.md`](http-client.md) — совместимость HTTP-клиента Connector.
- [`bytecode-hardening.md`](bytecode-hardening.md) — закрытие оставшихся
  невозможных состояний и служебных типов байткода.

Измерительный план SSA и оптимизаций находится в
[`../research/performance/ssa-hotspot-analysis.md`](../research/performance/ssa-hotspot-analysis.md),
поскольку его решения принимаются по данным профилирования.

Разделение Rust-модулей завершено: [архив OpenSpec](../../openspec/changes/archive/2026-09-07-separate-rust-module-responsibilities/proposal.md).
[Принятый план VM](bsl-vm-refactor.md) сохранён как карта стадий,
ограничений и измерений; активного checklist реализации в нём нет.
