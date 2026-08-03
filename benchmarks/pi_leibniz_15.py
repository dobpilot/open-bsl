"""Двойник `pi_leibniz_15.bsl` с округлением после каждого шага."""

from decimal import Decimal, ROUND_HALF_UP, localcontext
from time import perf_counter


DIVISION_QUANTUM = Decimal("1e-27")
ROUND_QUANTUM = Decimal("1e-15")


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
            total = (total + term).quantize(
                ROUND_QUANTUM,
                rounding=ROUND_HALF_UP,
            )
    result = (total * 4).quantize(ROUND_QUANTUM, rounding=ROUND_HALF_UP)
    print(format(result, "f").rstrip("0").rstrip("."))
    return result


started = perf_counter()
calculate()
print(f"{(perf_counter() - started) * 1_000:.3f}")
