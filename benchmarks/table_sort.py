"""Двойник `table_sort.bsl`: три сортировки и линейный поиск."""

from __future__ import annotations

from time import perf_counter


ROWS = 100_000
table: list[list[object]] = []
seed = 1
for number in range(1, ROWS + 1):
    seed = (seed * 1_103_515_245 + 12_345) % 2_147_483_648
    table.append([seed, "имя" + str(number)])

started = perf_counter()
table.sort(key=lambda row: row[0])
table.sort(key=lambda row: row[0], reverse=True)
# `ТаблицаЗначений.Сортировать` сравнивает строки сначала без учёта
# регистра, затем в исходном виде.
table.sort(key=lambda row: (str(row[1]).upper(), row[1]))

needle = "имя" + str(ROWS)
found = next((row for row in table if row[1] == needle), None)
elapsed_ms = (perf_counter() - started) * 1_000

print(f"строк: {len(table)}, нашлась: {found is not None}")
print(f"{elapsed_ms:.3f}")
