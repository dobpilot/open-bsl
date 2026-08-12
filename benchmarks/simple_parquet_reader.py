"""Двойник `simple_parquet_reader.bsl` на `pandas.read_parquet`.

ЧТО ЭТА КОЛОНКА МЕРЯЕТ НА САМОМ ДЕЛЕ. Не Python. `read_parquet` уходит в
pyarrow, то есть в векторизованный C++: разбор подвала, обход страниц и
раскодирование значений идут там вне интерпретатора, а Python остаётся
вызывающей стороной. Версия на BSL делает ту же работу СВОИМИ силами —
варинты, зигзаг, цепочки страниц, IEEE 754 из битов, — поэтому пара чисел
в строке таблицы отвечает не на вопрос «какой язык быстрее», а на вопрос
«сколько стоит разбор руками против готовой библиотеки на C++». Если
понадобится сравнение именно языков — его даёт двойник на Lua, который
разбирает файл сам.

Асимметрия есть и во второй половине работы: `ТаблицаЗначений`
построчная, и версия на BSL платит за перекладку колонок в строки, а
`DataFrame` остаётся колоночным и не платит за это ничего.

Контрольные величины считаются после остановки часов — как и у соседних
сценариев: это сверка, а не нагрузка.

pandas нужен только этому двойнику. Нет его — печатается подсказка вместо
миллисекунд, и `run.sh` показывает в колонке «ошибка».
"""

from time import perf_counter

PATH = "benchmarks/data/simple.parquet"

try:
    import pandas as pd
except ImportError:
    raise SystemExit(
        "нет pandas — поставьте python-pandas и python-pyarrow "
        "или venv с pip install pandas pyarrow"
    )

started = perf_counter()
frame = pd.read_parquet(PATH)
elapsed_ms = (perf_counter() - started) * 1_000

item = frame["Item"]
print(
    f"строк: {len(frame)}, колонок: {len(frame.columns)}, "
    f"первая: {item.iloc[0]}, последняя: {item.iloc[-1]}"
)
print(
    f"суммы: quantity {frame['quantity'].sum()}, price {frame['price'].sum()}, "
    f"summ {frame['summ'].sum():.0f}"
)
print(f"{elapsed_ms:.3f}")
