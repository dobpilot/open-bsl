#!/usr/bin/env python3
"""Готовит архив ZIP, который распаковывает сценарий `zip_read`.

    python3 benchmarks/make-zip-corpus.py [путь]

По умолчанию пишет `benchmarks/data/zip-corpus.zip` — тот самый файл,
который лежит в репозитории и который читает сценарий. Перезапускать
скрипт для прогона бенчмарка не нужно; он здесь затем, чтобы файл был
воспроизводим, а не был двоичным подарком без исходника.

СОДЕРЖИМОЕ ДЕТЕРМИНИРОВАНО. В архив укладываются три уже лежащих в `data/`
файла — `input_regexp.txt`, `EnterpriseData_1_0_1.xsd`,
`ExchangeMessage.xsd` — поэтому состав байтов входа фиксирован. Даты
записей выставлены в 1980-01-01 (как в conformance-фикстурах `zip-read.*`),
чтобы архив не зависел от времени сборки: дата в записи ZIP — это
`ЧтениеФайлаАрхива.Элементы[i].ВремяИзменения`, и у двух прогонов
генератора она обязана совпадать. Сжатие — deflate (способ 8), имя
каждой записи — базовое имя файла без пути.

ПОЧЕМУ ИМЕННО ЭТИ ТРИ ФАЙЛА. `input_regexp.txt` (6,6 МБ текста) даёт
основной объём распаковки, два XSD (496 КБ + 1 КБ) — разноплановый XML
поверх. Отдельного «плохо сжимаемого» случая здесь нет: профиль сжатия
однобокий, но сценарий `zip_read` меряет inflate, а не разнообразие
сжимаемости. Хочется плохо сжимаемого ввода — кладите в архив свой файл.
"""

import pathlib
import sys
import zipfile

DEFAULT_PATH = pathlib.Path(__file__).resolve().parent / "data" / "zip-corpus.zip"
DATA_DIR = pathlib.Path(__file__).resolve().parent / "data"
SOURCES = ["input_regexp.txt", "EnterpriseData_1_0_1.xsd", "ExchangeMessage.xsd"]
TIMESTAMP = (1980, 1, 1, 0, 0, 0)


def main():
    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PATH)
    path.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for name in SOURCES:
            src = DATA_DIR / name
            info = zipfile.ZipInfo(name, date_time=TIMESTAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            with open(src, "rb") as f:
                zf.writestr(info, f.read())

    total_in = sum((DATA_DIR / name).stat().st_size for name in SOURCES)
    size = path.stat().st_size
    print(f"{path}: {size} байт сжато, {total_in} байт исходных, записей {len(SOURCES)}")


if __name__ == "__main__":
    main()
