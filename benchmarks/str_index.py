# Двойник str_index.bsl. Итерация по строке: `s[i]` — доступ к кодовой
# точке O(1), native для Python. Кириллица — по одной кодовой точке на
# букву, как и код-юнит UTF-16 у BSL, поэтому число шагов совпадает.

import time

alphabet = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"
s = alphabet * 10000
length = len(s)

started = time.perf_counter()
total = 0
for i in range(length):
	total += ord(s[i])
elapsed_ms = (time.perf_counter() - started) * 1000

print(f"длина: {length}, контрольная сумма: {total}")
print(f"{elapsed_ms:.3f}")
