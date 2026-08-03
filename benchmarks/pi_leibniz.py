"""Двойник `pi_leibniz.bsl` с десятичным делением до 27 знаков."""

from decimal import Decimal, ROUND_HALF_UP, localcontext
from time import perf_counter


DIVISION_QUANTUM = Decimal("1e-27")


def calculate() -> Decimal:
    total = Decimal(0)
    sign = -1
    with localcontext() as context:
        context.prec = 80
        for number in range(1, 1_000_001):
            sign = -sign
            term = (Decimal(sign) / (2 * number - 1)).quantize(
                DIVISION_QUANTUM,
                rounding=ROUND_HALF_UP,
            )
            total += term
    result = total * 4
    print(format(result, "f").rstrip("0").rstrip("."))
    return result


started = perf_counter()
calculate()
print(f"{(perf_counter() - started) * 1_000:.3f}")
