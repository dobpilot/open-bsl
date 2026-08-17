"""Двойник `zip_write.bsl` на стандартном модуле `zipfile`.

ЧТО ЭТА КОЛОНКА МЕРЯЕТ НА САМОМ ДЕЛЕ. Не Python. `zipfile` уходит в
`zlib`/`libz` на C, deflate идёт вне интерпретатора, поэтому пара чисел
отвечает не на вопрос «какой язык быстрее», а на вопрос «сколько стоит
собственный deflate open-bsl против системной libz». Lua-двойника нет
(прочерк): у Lua нет стандартной библиотеки ZIP.

Контрольная величина считается после остановки часов — как и у соседних
сценариев: это сверка, а не нагрузка.
"""

from __future__ import annotations

from time import perf_counter
import zipfile

SOURCES = [
    "benchmarks/data/input_regexp.txt",
    "benchmarks/data/EnterpriseData_1_0_1.xsd",
    "benchmarks/data/ExchangeMessage.xsd",
]
DEST = "/tmp/open-bsl-zip-write-out.zip"

started = perf_counter()
with zipfile.ZipFile(DEST, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in SOURCES:
        zf.write(path, arcname=path.rsplit("/", 1)[-1])
elapsed_ms = (perf_counter() - started) * 1_000

total = 0
count = 0
with zipfile.ZipFile(DEST) as zf:
    for info in zf.infolist():
        count += 1
        total += info.file_size

print(f"записей: {count}, байт: {total}")
print(f"{elapsed_ms:.3f}")
