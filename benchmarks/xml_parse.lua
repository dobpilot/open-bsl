-- Двойник `xml_parse.bsl` на Lua: тот же документ разбирается SLAXML.
--
-- SLAXML — потоковый (SAX-подобный) разборщик на ЧИСТОМ Lua, лежит в
-- `benchmarks/lib` (происхождение и лицензия — в README рядом с ним).
-- Отсюда та же оговорка, что и у `json_parse.lua`: колонки Lua здесь меряют
-- ИНТЕРПРЕТИРУЕМЫЙ разбор, тогда как bsl-cli, OneScript и 1С — разбор,
-- реализованный внутри рантайма на компилируемом языке. Это сравнение не
-- языков, а способа доставки XML в язык.
--
-- Контракт бенчмарка: последняя напечатанная строка — миллисекунды числом.

local here = arg[0]:match('^(.*)/[^/]+$') or '.'
package.path = here .. '/lib/?.lua;' .. package.path
local SLAXML = require('slaxml')

local concat = table.concat

local RECORDS = 10000
local PASSES = 3

-- --- сборка документа (вне замера) ---------------------------------------
-- Текст обязан совпадать с тем, что пишет `xml_write.bsl`, БАЙТ В БАЙТ:
-- иначе рантаймы разбирали бы разные входные данные. Сверено сравнением
-- файлов, а не на глаз.
local function build()
    local out = {}
    local n = 0
    local function put(s) n = n + 1; out[n] = s end

    put('<?xml version="1.0" encoding="UTF-8"?><контрагенты>')
    local flag = false
    for i = 1, RECORDS do
        flag = not flag
        local num = tostring(i)
        put('<запись ид="' .. num .. '" активен="' .. (flag and 'Да' or 'Нет') .. '">')
        put('<имя>запись номер ' .. num .. '</имя>')
        -- `Формат(н + 0.75, "ЧГ=0; ЧРД=.")` — без разделителя групп, точка
        -- как дробный разделитель.
        put('<цена>' .. num .. '.75</цена>')
        put('<теги><тег>альфа</тег><тег>бета</тег><тег>гамма</тег></теги>')
        put('<вложенное число="' .. num .. '">значение</вложенное>')
        put('<контрагент инн="77010' .. num .. '" кпп="770101001">')
        put('<наименование>ООО Ромашка ' .. num .. '</наименование>')
        put('<банковскийСчет номер="4070281090000000' .. num
            .. '" бик="044525225" коррсчет="30101810400000000225">')
        put('<банк>АО Банк ' .. num .. '</банк></банковскийСчет>')
        put('<юрАдрес индекс="101000"><регион>Москва</регион><город>Москва</город>')
        put('<адрес>ул. Тверская, д. ' .. num .. '</адрес></юрАдрес>')
        put('<фактическийАдрес индекс="141400"><регион>Московская область</регион>')
        put('<город>Химки</город><адрес>ул. Победы, д. ' .. num .. '</адрес>')
        put('</фактическийАдрес></контрагент></запись>')
    end
    put('</контрагенты>')
    return concat(out)
end

local text = build()

-- Длина в КОД-ЮНИТАХ UTF-16, а не в байтах: `СтрДлина` в BSL меряет
-- именно их, и на кириллице байтовая длина разошлась бы с остальными
-- колонками от разницы кодировок, а не от разницы разбора. Все символы
-- документа лежат в BMP, поэтому код-юниты совпадают с числом символов, а
-- их считает число НЕпродолжающих байтов UTF-8. `utf8.len` не годится:
-- в LuaJIT его нет.
local function units(s)
    local _, count = s:gsub('[^\128-\191]', '')
    return count
end

-- --- замер ---------------------------------------------------------------
local nodes, elements, attributes, id_sum, value_units = 0, 0, 0, 0, 0
local in_record = false

local callbacks = {
    startElement = function(name)
        nodes = nodes + 1
        elements = elements + 1
        in_record = name == 'запись'
    end,
    closeElement = function()
        nodes = nodes + 1
    end,
    text = function()
        nodes = nodes + 1
    end,
    attribute = function(name, value)
        attributes = attributes + 1
        value_units = value_units + units(value)
        if in_record and name == 'ид' then
            id_sum = id_sum + tonumber(value)
        end
    end,
    -- Объявление XML приходит сюда инструкцией обработки. Узлом оно НЕ
    -- считается: платформа его тоже не отдаёт, и включить его значило бы
    -- разойтись с остальными колонками на единицу за проход.
    pi = function() end,
    comment = function() end,
}

local parser = SLAXML:parser(callbacks)

local t0 = os.clock()
for _ = 1, PASSES do
    nodes, elements, attributes, id_sum, value_units = 0, 0, 0, 0, 0
    parser:parse(text, { stripWhitespace = false })
end
local elapsed_ms = (os.clock() - t0) * 1000

-- --- проверка (вне замера) ------------------------------------------------
print(string.format(
    'длина документа: %d, узлов: %d, элементов: %d, атрибутов: %d, сумма ид: %d, длина значений: %d',
    units(text), nodes, elements, attributes, id_sum, value_units))
print(string.format('%.0f', elapsed_ms))
