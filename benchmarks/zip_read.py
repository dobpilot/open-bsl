"""Двойник `zip_read.bsl` на стандартном модуле `zipfile`.

ЧТО ЭТА КОЛОНКА МЕРЯЕТ НА САМОМ ДЕЛЕ. Не Python. `zipfile` написан на C
(`zlib`/`libz`), inflate уходит вне интерпретатора, поэтому пара чисел в
строке таблицы отвечает не на вопрос «какой язык быстрее», а на вопрос
«сколько стоит собственный inflate open-bsl против системной libz». Если
понадобится сравнение именно языков — его даёт двойник на Lua, которого у
этого сценария нет (прочерк, как у `edata_writer`): у Lua нет стандартной
библиотеки ZIP.

Контрольная величина считается после остановки часов — как и у соседних
сценариев: это сверка, а не нагрузка.
"""

from __future__ import annotations

import pathlib
from time import perf_counter
import zipfile

PATH = "benchmarks/data/zip-corpus.zip"
DEST = "/tmp/open-bsl-zip-read-out"

started = perf_counter()
with zipfile.ZipFile(PATH) as zf:
    zf.extractall(DEST)
elapsed_ms = (perf_counter() - started) * 1_000

total = 0
count = 0
with zipfile.ZipFile(PATH) as zf:
    for info in zf.infolist():
        count += 1
        total += (pathlib.Path(DEST) / info.filename).stat().st_size

print(f"записей: {count}, байт: {total}")
print(f"{elapsed_ms:.3f}")
