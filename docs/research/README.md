# Исследования

- [`compatibility/`](compatibility/) — измеренная совместимость и обратная
  разработка форматов.
- [`performance/`](performance/) — профили, модели стоимости и ворота допуска
  оптимизаций.
- [`session-module-runtime-surface-2026-09-21.md`](session-module-runtime-surface-2026-09-21.md) —
  поверхность платформы для запуска модулей сеанса `demo`, `buh` и `unf`.

- [`performance/session-initialization-hotspots-2026-10-22.md`](performance/session-initialization-hotspots-2026-10-22.md) —
  реальные CPU-профили инициализации UNF, BUH и DEMO после оптимизации фабрики.

- [`performance/session-hotspot-fixes-2026-10-22.md`](performance/session-hotspot-fixes-2026-10-22.md) —
  исправления обходов коллекций и query-финализации, контрольные профили и A/B.

- [`performance/session-open-hotspots-current-2026-10-22.md`](performance/session-open-hotspots-current-2026-10-22.md) —
  свежие hotspot открытия UNF, BUH и DEMO после обоих исправлений.

- [`performance/session-four-hotspot-optimizations-2026-10-22.md`](performance/session-four-hotspot-optimizations-2026-10-22.md) —
  выполнение четырёх направлений из профиля: метаданные, имена, финализация и аллокации.

- [`performance/interned-metadata-names-2026-10-22.md`](performance/interned-metadata-names-2026-10-22.md) —
  откаченный эксперимент с общими строками и числовыми ключами; сохранено исправление Unicode-хеша.
