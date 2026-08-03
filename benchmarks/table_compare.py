"""Двойник `table_compare.bsl` с разреженным хранением строк."""

from __future__ import annotations

from array import array
from time import perf_counter


COLUMN_COUNT = 18
DIMENSION_COUNT = 4
ROW_COUNT = 50_000


generation_started = perf_counter()
column_names = [f"Колонка_{column}" for column in range(1, COLUMN_COUNT + 1)]
table0_columns = bytearray()
table1_columns = bytearray()
table0_values: list[str] = []
table1_values: list[str] = []

for row in range(1, ROW_COUNT + 1):
    for column, column_name in enumerate(column_names, start=1):
        key = f"{row}_{column_name}"
        table0_columns.append(column)
        table0_values.append("A_" + key)
        table1_columns.append(column)
        table1_values.append("B_" + key)

generation_ms = (perf_counter() - generation_started) * 1_000
print(
    f"генерация: {generation_ms:.0f} мс, "
    f"строк в каждой таблице: {len(table0_values)}"
)

comparison_started = perf_counter()

# Копия правой таблицы со знаком 1, затем строки левой со знаком -1.
columns = table1_columns + table0_columns
values = table1_values + table0_values
signs = array("b", [1]) * len(table1_values)
signs.extend(array("b", [-1]) * len(table0_values))
counts = bytearray(b"\x01") * len(values)

# Отдельное соответствие каждой логической колонки повторяет группировку
# `Свернуть` по всем 18 колонкам разреженных тестовых строк.
slots_by_column: list[dict[str, int]] = [
    {} for _ in range(COLUMN_COUNT + 1)
]
group_count = 0
for read_position, value in enumerate(values):
    column = columns[read_position]
    slots = slots_by_column[column]
    slot = slots.get(value)
    if slot is None:
        slots[value] = group_count
        if group_count != read_position:
            columns[group_count] = column
            values[group_count] = value
            signs[group_count] = signs[read_position]
            counts[group_count] = counts[read_position]
        group_count += 1
    else:
        signs[slot] += signs[read_position]
        counts[slot] += counts[read_position]

answer_columns = bytearray()
answer_values: list[str] = []
answer_signs = array("b")
for position in range(group_count):
    if counts[position] == 1:
        answer_columns.append(columns[position])
        answer_values.append(values[position])
        answer_signs.append(signs[position])
del slots_by_column

# Для разреженной строки ключ четырёх измерений однозначно сводится к
# категории колонки и её единственному значению. Равные ключи сохраняют
# исходный порядок благодаря устойчивой сортировке Python.
order = list(range(len(answer_values)))


def dimension_key(position: int) -> tuple[int, str]:
    column = answer_columns[position]
    if column > DIMENSION_COUNT:
        return 0, ""
    return DIMENSION_COUNT + 1 - column, answer_values[position]


order.sort(key=dimension_key)
answer_columns = bytearray(answer_columns[position] for position in order)
answer_values = [answer_values[position] for position in order]
answer_signs = array("b", (answer_signs[position] for position in order))

comparison_ms = (perf_counter() - comparison_started) * 1_000
print(
    f"сравнение: {comparison_ms:.0f} мс, "
    f"строк результата: {len(answer_values)}"
)
print(f"{comparison_ms:.3f}")
