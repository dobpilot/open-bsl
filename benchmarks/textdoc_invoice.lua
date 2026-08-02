-- Двойник textdoc_invoice.bsl.
--
-- BSL-версия меряет `ТекстовыйДокумент`: области макета, параметры и
-- `Вывести` итогового документа. В Lua такого объекта нет, поэтому двойник
-- собирает ТОТ ЖЕ текст напрямую через массив фрагментов. Это нижняя
-- граница для Lua, а не семантически равная реализация макета.
--
-- Контрольная строка совпадает с BSL/1С: длина считается в кодовых точках
-- Unicode. В данных нет символов вне BMP, поэтому это то же число, что
-- `СтрДлина` в BSL считает в код-юнитах UTF-16.

local concat = table.concat

local POSITIONS = 100000
local SPACES = "                                "

local function right_field(value, width)
    local text = tostring(value)
    return string.sub(SPACES .. text, -width)
end

local function money_from_cents(cents)
    return string.format("%d.%02d", math.floor(cents / 100), cents % 100)
end

local function utf8_len(s)
    local n = 0
    for i = 1, #s do
        local b = string.byte(s, i)
        if b < 0x80 or b >= 0xC0 then
            n = n + 1
        end
    end
    return n
end

local started = os.clock()

local parts = {
    "                                Расходная накладная\n",
    " \n",
    "Номер         00011      \n",
    "Дата          10.05.2002  \n",
    "Контрагент    Эльбрус     \n",
    " \n",
    "+--------------------------------+----------------+------------+------------+\n",
    "|        Номенклатура            |   Количество   |    Цена    |    Сумма   |\n",
    "+--------------------------------+----------------+------------+------------+\n",
}

local total_cents = 0
for i = 1, POSITIONS do
    local num = tostring(i)
    local qty = 1 + (i % 10)
    local price_cents = 1000 + (i % 997) * 10
    local sum_cents = qty * price_cents
    total_cents = total_cents + sum_cents
    local item = "Товар расходной накладной " .. num

    parts[#parts + 1] = "|"
        .. item
        .. string.rep(" ", 32 - (26 + #num))
        .. "|"
        .. right_field(qty, 16)
        .. "|"
        .. right_field(money_from_cents(price_cents), 12)
        .. "|"
        .. right_field(money_from_cents(sum_cents), 12)
        .. "|\n"
    parts[#parts + 1] = "+--------------------------------+----------------+------------+------------+\n"
end

parts[#parts + 1] = "|Итого                           |                |            |"
    .. right_field(money_from_cents(total_cents), 12)
    .. "|\n"
parts[#parts + 1] = "+--------------------------------+----------------+------------+------------+\n"
parts[#parts + 1] = " \n"
parts[#parts + 1] = "Склад         Основной склад         \n"

local document = concat(parts)
local elapsed_ms = (os.clock() - started) * 1000

print(string.format(
    "позиций: %d, строк: %d, длина: %d, итог: %s",
    POSITIONS,
    select(2, string.gsub(document, "\n", "")),
    utf8_len(document),
    money_from_cents(total_cents)
))
print(string.format("%.3f", elapsed_ms))
