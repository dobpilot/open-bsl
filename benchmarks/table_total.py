"""Двойник `table_total.bsl` с колоночным хранением значений."""

from __future__ import annotations

from time import perf_counter


ROWS = 200_000
PASSES = 50

started = perf_counter()
numbers: list[int] = []
names: list[str] = []
for number in range(1, ROWS + 1):
    numbers.append(number)
    names.append("строка")

total = 0
for _ in range(PASSES):
    total = sum(numbers)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"строк: {len(numbers)}, итог: {total}")
print(f"{elapsed_ms:.3f}")
