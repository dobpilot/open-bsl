"""Двойник `xml_parse.bsl` на потоковом разборщике Expat."""

from __future__ import annotations

from time import perf_counter
from xml.parsers import expat

from benchmark_data import PASSES, xml_document


text = xml_document()
encoded = text.encode("utf-8")

nodes = 0
elements = 0
attributes = 0
id_sum = 0
value_length = 0

started = perf_counter()
for _ in range(PASSES):
    nodes = 0
    elements = 0
    attributes = 0
    id_sum = 0
    value_length = 0

    parser = expat.ParserCreate()

    def start_element(name: str, attrs: dict[str, str]) -> None:
        global nodes, elements, attributes, id_sum, value_length
        nodes += 1
        elements += 1
        attributes += len(attrs)
        value_length += sum(len(value) for value in attrs.values())
        if name == "запись":
            id_sum += int(attrs["ид"])

    def end_element(_name: str) -> None:
        global nodes
        nodes += 1

    def character_data(data: str) -> None:
        global nodes
        if data:
            nodes += 1

    parser.StartElementHandler = start_element
    parser.EndElementHandler = end_element
    parser.CharacterDataHandler = character_data
    parser.Parse(encoded, True)

elapsed_ms = (perf_counter() - started) * 1_000

print(
    f"длина документа: {len(text)}, узлов: {nodes}, элементов: {elements}, "
    f"атрибутов: {attributes}, сумма ид: {id_sum}, "
    f"длина значений: {value_length}"
)
print(f"{elapsed_ms:.3f}")
