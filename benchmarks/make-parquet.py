#!/usr/bin/env python3
"""Готовит файл parquet, который читает сценарий `simple_parquet_reader`.

    python3 benchmarks/make-parquet.py [путь]

По умолчанию пишет `benchmarks/data/simple.parquet` — тот самый файл,
который лежит в репозитории и который читает сценарий. Перезапускать
скрипт для прогона бенчмарка не нужно; он здесь затем, чтобы файл был
воспроизводим, а не был двоичным подарком без исходника.

СОДЕРЖИМОЕ ДЕТЕРМИНИРОВАНО. Генератор случайных чисел засеян
фиксированным числом, поэтому файл побайтно один и тот же на любой
машине, а контрольные суммы в выводе сценария имеют смысл. Менять зерно
или число строк нельзя, не переснимая эти суммы И не перезаписывая
файл в репозитории.

ПОЧЕМУ ИМЕННО ТАКИЕ КЛЮЧИ ЗАПИСИ. Читатель на BSL разбирает подмножество
формата, а не формат целиком, и каждый ключ здесь ровно на то, чтобы
подмножество осталось честным:

    compression=None      без него pyarrow пишет SNAPPY, а распаковка —
                          это отдельная подсистема, которой в дереве нет
    use_dictionary=False  иначе строки уедут в словарную страницу с
                          кодированием RLE_DICTIONARY вместо PLAIN
    nullable=False        у обязательной колонки в странице нет уровней
                          определённости, и значения идут подряд
    data_page_version 1.0 у страниц V2 уровни вынесены в заголовок
                          страницы, разбор у них другой
    write_statistics      статистика в метаданных читателю не нужна, а
                          заголовки от неё пухнут

Размер страницы данных НЕ задаётся намеренно: pyarrow режет колонку по
мегабайту, строковая колонка на 100 000 строк в одну страницу не влезает,
и читатель обязан уметь идти по цепочке страниц — как на любом реальном
файле.
"""

import pathlib
import random
import sys

import pandas as pd
import pyarrow as pa
import pyarrow.parquet as pq

ROWS = 100_000
SEED = 20260812
DEFAULT_PATH = pathlib.Path(__file__).resolve().parent / "data" / "simple.parquet"


def main():
    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PATH)
    path.parent.mkdir(parents=True, exist_ok=True)

    rnd = random.Random(SEED)
    quantity = [rnd.randint(10, 100) for _ in range(ROWS)]
    price = [rnd.randint(10, 100) for _ in range(ROWS)]

    frame = pd.DataFrame(
        {
            "Item": [f"Номенклатура {i + 1}" for i in range(ROWS)],
            "quantity": quantity,
            "price": price,
            "summ": [float(q * p) for q, p in zip(quantity, price)],
        }
    )

    schema = pa.schema(
        [
            pa.field("Item", pa.string(), nullable=False),
            pa.field("quantity", pa.int32(), nullable=False),
            pa.field("price", pa.int64(), nullable=False),
            pa.field("summ", pa.float64(), nullable=False),
        ]
    )

    table = pa.Table.from_pandas(frame, schema=schema, preserve_index=False)
    pq.write_table(
        table,
        path,
        compression=None,
        use_dictionary=False,
        write_statistics=False,
        data_page_version="1.0",
        row_group_size=ROWS,
        store_schema=False,
    )

    size = path.stat().st_size
    total = sum(q * p for q, p in zip(quantity, price))
    print(f"{path}: {size} байт, строк {ROWS}, сумма summ {total}")


if __name__ == "__main__":
    main()
