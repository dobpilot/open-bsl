"""Двойник `json_parse.bsl` на стандартном модуле `json`."""

from __future__ import annotations

import json
from time import perf_counter

from benchmark_data import PASSES, json_document


text = json_document()

started = perf_counter()
record_count = 0
data: list[dict[str, object]] = []
for _ in range(PASSES):
    data = json.loads(text)
    record_count += len(data)
elapsed_ms = (perf_counter() - started) * 1_000

checksum = 0
complete = 0
for record in data:
    nested = record["вложенное"]
    contractor = record["контрагент"]
    assert isinstance(nested, dict) and isinstance(contractor, dict)
    account = contractor["банковскийСчет"]
    legal = contractor["юрАдрес"]
    actual = contractor["фактическийАдрес"]
    tags = record["теги"]
    assert isinstance(account, dict) and isinstance(legal, dict)
    assert isinstance(actual, dict) and isinstance(tags, list)

    checksum += int(record["ид"]) + int(nested["число"])
    checksum += len(str(contractor["инн"])) + len(str(contractor["кпп"]))
    checksum += len(str(account["номер"])) + len(str(account["бик"]))
    checksum += len(str(account["коррсчет"]))
    checksum += len(str(legal["индекс"])) + len(str(actual["индекс"]))
    if (
        contractor["наименование"]
        and legal["адрес"]
        and actual["город"]
        and tags[2]
    ):
        complete += 1

print(
    f"длина документа: {len(text)}, записей: {record_count}, "
    f"сумма: {checksum}, полных: {complete}"
)
print(f"{elapsed_ms:.3f}")
