"""Двойник ``goto_bench.bsl`` с тем же автоматом из восьми состояний.

В Python нет штатного ``goto``, поэтому переход к блоку представлен
ветвью ``if/elif``. Число проходов, порядок состояний и контрольная сумма
совпадают с BSL- и Lua-сценариями.
"""

from time import perf_counter


iteration = 0
state = 0
total = 0
started = perf_counter()

while iteration < 2_000_000:
    if state == 0:
        total += 1
        state = 1
    elif state == 1:
        total += 3
        state = 2
    elif state == 2:
        total += 5
        state = 3
    elif state == 3:
        total += 7
        state = 4
    elif state == 4:
        total += 11
        state = 5
    elif state == 5:
        total += 13
        state = 6
    elif state == 6:
        total += 17
        state = 7
    else:
        total += 19
        state = 0
    iteration += 1

elapsed_ms = (perf_counter() - started) * 1_000
print(f"сумма: {total}")
print(f"{elapsed_ms:.3f}")
