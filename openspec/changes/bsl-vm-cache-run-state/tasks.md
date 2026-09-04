# Задачи

## 1. Контракт до реализации

- [ ] 1.1 Тест `bsl-vm`: на старте прогона у каждого чанка по ячейке
  обоих кэшей на инструкцию (инвариант, ушедший из `image::verify`).
- [ ] 1.2 Тест `bsl-vm`: второй прогон той же `Program` стартует с
  холодным кэшем — прогрев первого ему не виден.
- [ ] 1.3 Тест `bsl-vm`: размер ячейки кэша методов зафиксирован в 32
  байта (переносится из `bsl-bytecode` вместе с типом).

## 2. `bsl-bytecode`: чанк без состояния

- [ ] 2.1 Удалить поля `prop_cache`/`method_cache`, методы-аксессоры и
  типы ячеек из `chunk.rs`; перенести типы в `bsl-vm`.
- [ ] 2.2 `image::finalize`/`finalize_lone_chunk*` больше не заводят и
  не сбрасывают ячейки; `image::verify` больше не проверяет их длину.
  Снять тесты `finalization_does_not_keep_warm_inline_caches` и
  `a_short_inline_cache_is_rejected_before_the_first_instruction`
  (их предмет закрывают 1.1–1.2).
- [ ] 2.3 Вычистить сборку ячеек из тестовых обвязок `bundle.rs`,
  `analysis.rs`, `text.rs` и интеграционных тестов; поправить
  round-trip-корпус (`text_round_trip.rs`) — утверждение о длине кэша
  уходит вместе с кэшем.

## 3. `bsl-vm`: состояние запуска

- [ ] 3.1 Ввести `RunCaches` (по вектору ячеек на чанк) с конструктором
  `for_program`; владение — `ProgramExecution`, каталожные модули — в
  `attach_catalog` тем же индексом, что `session_modules`.
- [ ] 3.2 Протянуть `&RunCaches` через `poll_linked` → `step` →
  `step_cold`; выбор набора текущего модуля — рядом с выбором
  `(cur_program, cur_linked)`. Помощники `prop_cache`/`method_cache`/
  `cached_component_method` переходят на адресацию `(func_id, pc)`.
- [ ] 3.3 JIT: поле `caches` в `JitCtx`, протяжка в четыре шима свойств
  и `shim_call_object_method`; заглушка не-x86-64 получает тот же
  параметр.

## 4. Проверка

- [ ] 4.1 `cargo fmt --all -- --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc
  --workspace --no-deps`, `cargo build --workspace`.
- [ ] 4.2 `cargo test --workspace`; отдельно —
  `the_jit_agrees_with_the_interpreter_on_every_script` (весь корпус) и
  полный конформанс `bsl-cli`.
- [ ] 4.3 Диф `docs/reference/bsl-api/api.md` пуст: поверхность BSL API
  не задета (страховка, генератор не перезапускается по этой правке).
- [ ] 4.4 Чередующийся A/B против базового бинарника ветки, условия по
  плану (`performance`, сеть, низкий load): `empty_for` (контроль),
  `call_overhead`, `pi_leibniz`, `bmp_rotate`, `json_write` или
  `table_total`. Вердикт по ИНСТРУКЦИЯМ; ожидание из перепроверки
  4 сентября — ~1 % на кэш-плотных, до ~3 % на `call_overhead`; заметно
  больше — разобрать до принятия. Числа ЗАПИСАТЬ в строку стадии 1
  таблицы `docs/plans/bsl-vm-refactor.md`.
- [ ] 4.5 `benchmarks/hot-code-diff.sh`: сдвиг `step` ожидаем — записать
  размеры до/после там же.

## 5. Завершение

- [ ] 5.1 Влить дельту в `openspec/specs/bytecode-image/spec.md`,
  переместить изменение в
  `openspec/changes/archive/YYYY-MM-DD-bsl-vm-cache-run-state/`.
- [ ] 5.2 Отметить стадию 1 в `docs/plans/bsl-vm-refactor.md` сделанной,
  с заполненной строкой таблицы цены.
