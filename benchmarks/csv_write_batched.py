"""Двойник `csv_write_batched.bsl` с одной записью готовой строки."""

from time import perf_counter


values = [str(number) for number in range(1, 21)]
values.insert(13, values[12])
row = (";".join(values) + "\r\n").encode()

started = perf_counter()
with open("test.csv", "wb") as output:
    output.write(b"\xef\xbb\xbf")
    for _ in range(300_001):
        output.write(row)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"{elapsed_ms:.3f}")
