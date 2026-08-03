"""Двойник `str_find.bsl`: поиск подстроки в конце длинной строки."""

from time import perf_counter


piece = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя0123456789"
haystack = piece * 5_000 + "ИГОЛКА"

started = perf_counter()
position = 0
for _ in range(500):
    # `СтрНайти` возвращает позицию с единицы, а `str.find` — с нуля.
    position = haystack.find("ИГОЛКА") + 1
elapsed_ms = (perf_counter() - started) * 1_000

print(f"длина стога: {len(haystack)}, позиция: {position}")
print(f"{elapsed_ms:.3f}")
