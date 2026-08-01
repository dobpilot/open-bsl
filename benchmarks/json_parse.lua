-- Двойник json_parse.bsl.
--
-- ВАЖНО ПРО СОПОСТАВИМОСТЬ. У Lua нет JSON ни в стандартной библиотеке, ни
-- в этой системе (проверено: cjson, dkjson, rapidjson, lunajson — ничего).
-- Поэтому разборщик написан здесь же, и колонка Lua меряет ИНТЕРПРЕТИРУЕМЫЙ
-- разбор, тогда как bsl-cli, oscript и 1С меряют разбор, реализованный
-- внутри рантайма на компилируемом языке. Сравнивать эти числа в лоб
-- нельзя — см. README.
--
-- Разбор написан идиоматично быстро: поиск через string.find (он на C), а
-- не побайтовый цикл на Lua. Медленная реализация здесь оболгала бы Lua не
-- меньше, чем отсутствие колонки вовсе.

local find, sub, byte, concat = string.find, string.sub, string.byte, table.concat

local ESCAPES = {
    ['"'] = '"', ['\\'] = '\\', ['/'] = '/',
    b = '\b', f = '\f', n = '\n', r = '\r', t = '\t',
}

local function parse_json(s)
    local pos = 1

    local function skip_ws()
        local _, e = find(s, '^[ \t\n\r]*', pos)
        pos = e + 1
    end

    local function parse_string()
        pos = pos + 1 -- открывающая кавычка
        local start = pos
        local buf = nil
        while true do
            local i = find(s, '["\\]', pos)
            if not i then
                error('незакрытая строка')
            end
            if byte(s, i) == 34 then -- "
                local chunk = sub(s, start, i - 1)
                pos = i + 1
                if buf then
                    buf[#buf + 1] = chunk
                    return concat(buf)
                end
                return chunk
            end
            -- Экранирование: в этом документе не встречается, но разборщик
            -- без него был бы не разборщиком.
            buf = buf or {}
            buf[#buf + 1] = sub(s, start, i - 1)
            local esc = sub(s, i + 1, i + 1)
            if esc == 'u' then
                buf[#buf + 1] = utf8 and utf8.char(tonumber(sub(s, i + 2, i + 5), 16)) or '?'
                pos = i + 6
            else
                buf[#buf + 1] = ESCAPES[esc] or esc
                pos = i + 2
            end
            start = pos
        end
    end

    local parse_value

    local function parse_object()
        pos = pos + 1 -- {
        local out = {}
        skip_ws()
        if byte(s, pos) == 125 then -- }
            pos = pos + 1
            return out
        end
        while true do
            skip_ws()
            local key = parse_string()
            skip_ws()
            pos = pos + 1 -- :
            out[key] = parse_value()
            skip_ws()
            local c = byte(s, pos)
            pos = pos + 1
            if c == 125 then -- }
                return out
            end
        end
    end

    local function parse_array()
        pos = pos + 1 -- [
        local out = {}
        local count = 0
        skip_ws()
        if byte(s, pos) == 93 then -- ]
            pos = pos + 1
            return out
        end
        while true do
            count = count + 1
            out[count] = parse_value()
            skip_ws()
            local c = byte(s, pos)
            pos = pos + 1
            if c == 93 then -- ]
                return out
            end
        end
    end

    parse_value = function()
        skip_ws()
        local c = byte(s, pos)
        if c == 123 then -- {
            return parse_object()
        elseif c == 91 then -- [
            return parse_array()
        elseif c == 34 then -- "
            return parse_string()
        elseif c == 116 then -- t
            pos = pos + 4
            return true
        elseif c == 102 then -- f
            pos = pos + 5
            return false
        elseif c == 110 then -- n
            pos = pos + 4
            return nil
        else
            local i = find(s, '[^%-%+%d%.eE]', pos) or (#s + 1)
            local num = tonumber(sub(s, pos, i - 1))
            pos = i
            return num
        end
    end

    return parse_value()
end

local RECORDS = 2000
local PASSES = 20

-- --- сборка документа (вне замера) ---------------------------------------
local parts = { '[' }
local flag = 'false'
for i = 1, RECORDS do
    if i > 1 then
        parts[#parts + 1] = ','
    end
    flag = (flag == 'true') and 'false' or 'true'
    parts[#parts + 1] = '{"ид":' .. i
        .. ',"имя":"запись номер ' .. i .. '"'
        .. ',"активен":' .. flag
        .. ',"цена":' .. i .. '.75'
        .. ',"теги":["альфа","бета","гамма"]'
        .. ',"вложенное":{"ключ":"значение","число":' .. i .. '}}'
end
parts[#parts + 1] = ']'
local text = concat(parts)

-- --- замер ----------------------------------------------------------------
local started = os.clock()
local records = 0
local data
for _ = 1, PASSES do
    data = parse_json(text)
    records = records + #data
end
local elapsed_ms = (os.clock() - started) * 1000

-- --- проверка (вне замера) -------------------------------------------------
local sum = 0
for _, rec in ipairs(data) do
    sum = sum + rec['ид'] + rec['вложенное']['число']
end

print(string.format('длина документа (байт): %d, записей: %d, сумма: %d', #text, records, sum))
print(string.format('%.3f', elapsed_ms))
