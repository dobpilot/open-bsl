"""Двойник `call_overhead.bsl`: миллион вызовов одной функции."""

from time import perf_counter


def add(left: int, right: int) -> int:
    return left + right


started = perf_counter()
total = 0
for number in range(1, 1_000_001):
    total = add(total, number)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"сумма: {total}")
print(f"{elapsed_ms:.3f}")
