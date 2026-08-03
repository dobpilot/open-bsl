"""Двойник `textdoc_invoice.bsl`, собирающий тот же текст накладной."""

from time import perf_counter


POSITIONS = 100_000


def money_from_cents(cents: int) -> str:
    return f"{cents // 100}.{cents % 100:02d}"


started = perf_counter()
parts = [
    "                                Расходная накладная\n",
    " \n",
    "Номер         00011      \n",
    "Дата          10.05.2002  \n",
    "Контрагент    Эльбрус     \n",
    " \n",
    "+--------------------------------+----------------+------------+------------+\n",
    "|        Номенклатура            |   Количество   |    Цена    |    Сумма   |\n",
    "+--------------------------------+----------------+------------+------------+\n",
]

total_cents = 0
for number in range(1, POSITIONS + 1):
    quantity = 1 + number % 10
    price_cents = 1_000 + (number % 997) * 10
    amount_cents = quantity * price_cents
    total_cents += amount_cents
    item = "Товар расходной накладной " + str(number)
    parts.append(
        f"|{item:<32}|{quantity:>16}|"
        f"{money_from_cents(price_cents):>12}|"
        f"{money_from_cents(amount_cents):>12}|\n"
    )
    parts.append(
        "+--------------------------------+----------------+------------+------------+\n"
    )

parts.append(
    "|Итого                           |                |            |"
    f"{money_from_cents(total_cents):>12}|\n"
)
parts.extend(
    (
        "+--------------------------------+----------------+------------+------------+\n",
        " \n",
        "Склад         Основной склад         \n",
    )
)
document = "".join(parts)
elapsed_ms = (perf_counter() - started) * 1_000

# Запись, как и в BSL-сценарии, не входит в измеряемый участок.
with open("benchmarks/textdoc_invoice.txt", "w", encoding="utf-8", newline="") as output:
    output.write(document)

print(
    f"позиций: {POSITIONS}, строк: {document.count(chr(10))}, "
    f"длина: {len(document)}, итог: {money_from_cents(total_cents)}"
)
print(f"{elapsed_ms:.3f}")
