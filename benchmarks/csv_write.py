"""Двойник `csv_write.bsl` с отдельным вызовом на каждый фрагмент."""

from time import perf_counter


values = [str(number).encode() for number in range(1, 21)]
# BSL-сценарий намеренно повторяет поле `d13`.
values.insert(13, values[12])
separator = b";"

started = perf_counter()
with open("test.csv", "wb") as output:
    # `ЗаписьТекста` в 1С пишет UTF-8 с BOM и преобразует LF в CRLF.
    output.write(b"\xef\xbb\xbf")
    for _ in range(300_001):
        for index, value in enumerate(values):
            output.write(value)
            if index + 1 < len(values):
                output.write(separator)
        output.write(b"\r\n")
elapsed_ms = (perf_counter() - started) * 1_000

print(f"{elapsed_ms:.3f}")
