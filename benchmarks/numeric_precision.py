"""Двойник `numeric_precision.bsl` на двоичных float, не эмуляция чисел 1С."""

from time import perf_counter


def small_rump(a, b):
    b2 = b * b
    b4 = b2 * b2
    b6 = b4 * b2
    b8 = b4 * b4
    a2 = a * a
    t1 = 333.75 * b6
    inner = 11.0 * a2 * b2 - b6 - 121.0 * b4 - 2.0
    t2 = a2 * inner
    t3 = 5.5 * b8
    t4 = a / (2.0 * b)
    s1 = t1 + t2
    s2 = s1 + t3
    result = s2 + t4
    return result


def multiply_rounding():
    x = 1234567890.123456789
    y = 9876543210.987654321
    return x * y


def harmonic_series():
    total = 0.0
    n = 1.0
    while n <= 10000000.0:
        total = total + (1.0 / n)
        n = n + 1.0
    return total


def binet_fibonacci():
    sqrt5 = 2.2360679774997896964091736687
    phi = (1.0 + sqrt5) / 2.0
    psi = (1.0 - sqrt5) / 2.0
    phi_pow = 1.0
    psi_pow = 1.0
    i = 1
    while i <= 75:
        phi_pow = phi_pow * phi
        psi_pow = psi_pow * psi
        i = i + 1
    return (phi_pow - psi_pow) / sqrt5


def subtractive_cancellation():
    root_x = 10000000000000.0
    root_x_plus_1 = 10000000000000.00000000000005
    return root_x_plus_1 - root_x


def calculate_euler():
    e_val = 1.0
    term = 1.0
    i = 1.0
    while term > 0.0:
        term = term / i
        if term == 0.0:
            break
        e_val = e_val + term
        i = i + 1.0
    return e_val


def sqrt2_newton():
    s = 2.0
    x = 1.0
    prev_x = 0.0
    while x != prev_x:
        prev_x = x
        # То же ограничение дробных знаков, что в BSL-двойнике.
        x = round(0.5 * (x + (s / x)), 27)
    return x


def arc_tan_taylor(x):
    total = 0.0
    term = x
    x2 = x * x
    n = 1.0
    sign = 1.0
    while term > 0.0:
        total = total + sign * (term / n)
        term = round(term * x2, 27)
        n = n + 2.0
        sign = 0.0 - sign
    return total


def calculate_pi():
    part1 = 16.0 * arc_tan_taylor(1.0 / 5.0)
    part2 = 4.0 * arc_tan_taylor(1.0 / 239.0)
    return part1 - part2


def mortgage_annuity():
    principal = 1000000000.0
    annual_rate = 0.035
    months = 360.0
    r = annual_rate / 12.0
    power = 1.0
    i = 1.0
    while i <= months:
        power = power * (1.0 + r)
        i = i + 1.0
    pmt = principal * (r * power) / (power - 1.0)
    return pmt


def main():
    started = perf_counter()
    print(f"small_rump: {small_rump(77.0, 33.0):.17g}")
    print(f"multiply_rounding: {multiply_rounding():.17g}")
    print(f"harmonic_series: {harmonic_series():.17g}")
    print(f"binet_fibonacci: {binet_fibonacci():.17g}")
    print(f"subtractive_cancellation: {subtractive_cancellation():.17g}")
    print(f"euler: {calculate_euler():.17g}")
    print(f"sqrt2_newton: {sqrt2_newton():.17g}")
    print(f"pi_machin: {calculate_pi():.17g}")
    print(f"mortgage_annuity: {mortgage_annuity():.17g}")
    print(f"{(perf_counter() - started) * 1000:.3f}")


if __name__ == "__main__":
    main()
