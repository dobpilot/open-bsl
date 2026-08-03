"""Двойник `empty_for.bsl`: накладные расходы пустого цикла."""

from time import perf_counter


started = perf_counter()
for _ in range(10):
    for _ in range(1_000_001):
        pass

print(f"{(perf_counter() - started) * 100:.3f}")
