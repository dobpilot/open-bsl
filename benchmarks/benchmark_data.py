"""Общие данные для Python-двойников JSON- и XML-бенчмарков."""

from __future__ import annotations

import json
from io import StringIO
from typing import TextIO
from xml.sax.saxutils import XMLGenerator
from xml.sax.xmlreader import AttributesImpl


RECORDS = 10_000
PASSES = 3


def make_record(number: int) -> dict[str, object]:
    """Создаёт одну запись той же формы, что и BSL-сценарии."""
    text_number = str(number)
    return {
        "ид": number,
        "имя": "запись номер " + text_number,
        "активен": number % 2 == 1,
        "цена": number + 0.75,
        "теги": ["альфа", "бета", "гамма"],
        "вложенное": {"ключ": "значение", "число": number},
        "контрагент": {
            "инн": "77010" + text_number,
            "кпп": "770101001",
            "наименование": "ООО Ромашка " + text_number,
            "банковскийСчет": {
                "номер": "4070281090000000" + text_number,
                "банк": "АО Банк " + text_number,
                "бик": "044525225",
                "коррсчет": "30101810400000000225",
            },
            "юрАдрес": {
                "регион": "Москва",
                "город": "Москва",
                "индекс": "101000",
                "адрес": "ул. Тверская, д. " + text_number,
            },
            "фактическийАдрес": {
                "регион": "Московская область",
                "город": "Химки",
                "индекс": "141400",
                "адрес": "ул. Победы, д. " + text_number,
            },
        },
    }


def make_records() -> list[dict[str, object]]:
    """Создаёт полный набор записей."""
    return [make_record(number) for number in range(1, RECORDS + 1)]


def json_document() -> str:
    """Сериализует документ без пробелов и ASCII-экранирования кириллицы."""
    return json.dumps(
        make_records(),
        ensure_ascii=False,
        separators=(",", ":"),
    )


def write_xml_document(target: TextIO) -> None:
    """Пишет XML-документ потоковыми вызовами стандартной библиотеки."""
    writer = XMLGenerator(target, encoding="UTF-8", short_empty_elements=False)
    attrs = AttributesImpl

    # `startDocument()` добавляет перевод строки, которого нет у `ЗаписьXML`.
    target.write('<?xml version="1.0" encoding="UTF-8"?>')
    writer.startElement("контрагенты", attrs({}))

    for number in range(1, RECORDS + 1):
        text_number = str(number)
        writer.startElement(
            "запись",
            attrs(
                {
                    "ид": text_number,
                    "активен": "Да" if number % 2 == 1 else "Нет",
                }
            ),
        )

        writer.startElement("имя", attrs({}))
        writer.characters("запись номер " + text_number)
        writer.endElement("имя")

        writer.startElement("цена", attrs({}))
        writer.characters(text_number + ".75")
        writer.endElement("цена")

        writer.startElement("теги", attrs({}))
        for tag in ("альфа", "бета", "гамма"):
            writer.startElement("тег", attrs({}))
            writer.characters(tag)
            writer.endElement("тег")
        writer.endElement("теги")

        writer.startElement("вложенное", attrs({"число": text_number}))
        writer.characters("значение")
        writer.endElement("вложенное")

        writer.startElement(
            "контрагент",
            attrs({"инн": "77010" + text_number, "кпп": "770101001"}),
        )
        writer.startElement("наименование", attrs({}))
        writer.characters("ООО Ромашка " + text_number)
        writer.endElement("наименование")

        writer.startElement(
            "банковскийСчет",
            attrs(
                {
                    "номер": "4070281090000000" + text_number,
                    "бик": "044525225",
                    "коррсчет": "30101810400000000225",
                }
            ),
        )
        writer.startElement("банк", attrs({}))
        writer.characters("АО Банк " + text_number)
        writer.endElement("банк")
        writer.endElement("банковскийСчет")

        writer.startElement("юрАдрес", attrs({"индекс": "101000"}))
        for name, value in (
            ("регион", "Москва"),
            ("город", "Москва"),
            ("адрес", "ул. Тверская, д. " + text_number),
        ):
            writer.startElement(name, attrs({}))
            writer.characters(value)
            writer.endElement(name)
        writer.endElement("юрАдрес")

        writer.startElement("фактическийАдрес", attrs({"индекс": "141400"}))
        for name, value in (
            ("регион", "Московская область"),
            ("город", "Химки"),
            ("адрес", "ул. Победы, д. " + text_number),
        ):
            writer.startElement(name, attrs({}))
            writer.characters(value)
            writer.endElement(name)
        writer.endElement("фактическийАдрес")

        writer.endElement("контрагент")
        writer.endElement("запись")

    writer.endElement("контрагенты")


def xml_document() -> str:
    """Возвращает XML-документ строкой для сценария разбора."""
    target = StringIO()
    write_xml_document(target)
    return target.getvalue()
