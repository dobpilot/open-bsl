"""Двойник `cbr_rates.bsl` на стандартной библиотеке Python."""

from __future__ import annotations

import json
from decimal import Decimal
from time import perf_counter
from urllib.request import urlopen
from xml.etree import ElementTree


XML_URL = "https://www.cbr-xml-daily.ru/daily_eng_utf8.xml"
JSON_URL = "https://www.cbr-xml-daily.ru/daily_json.js"
CODES = ("USD", "EUR", "CNY")


def fetch(url: str) -> bytes:
    with urlopen(url, timeout=30) as response:
        if response.status != 200:
            raise RuntimeError(f"HTTP {response.status}: {url}")
        return response.read()


def parse_xml(document: bytes) -> dict[str, Decimal]:
    rates: dict[str, Decimal] = {}
    root = ElementTree.fromstring(document)
    for currency in root.findall("Valute"):
        code = currency.findtext("CharCode")
        nominal = currency.findtext("Nominal")
        value = currency.findtext("Value")
        if code is None or nominal is None or value is None:
            raise ValueError("неполная запись Valute в XML")
        rates[code] = Decimal(value.replace(",", ".")) / Decimal(nominal)
    return rates


def parse_json(document: bytes) -> dict[str, Decimal]:
    data = json.loads(document, parse_float=Decimal, parse_int=Decimal)
    currencies = data["Valute"]
    return {
        code: currency["Value"] / currency["Nominal"]
        for code, currency in currencies.items()
    }


def presentation(rates: dict[str, Decimal]) -> str:
    return "; ".join(f"{code}={rates[code]:.4f}" for code in CODES)


started = perf_counter()
xml_output = presentation(parse_xml(fetch(XML_URL)))
json_output = presentation(parse_json(fetch(JSON_URL)))
elapsed_ms = (perf_counter() - started) * 1_000

if xml_output != json_output:
    raise RuntimeError(f"курсы в XML и JSON расходятся: {xml_output} / {json_output}")

print(f"XML: {xml_output}")
print(f"JSON: {json_output}")
print(f"{elapsed_ms:.3f}")
