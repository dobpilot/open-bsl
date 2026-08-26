-- Двойник `cbr_rates.bsl` на LuaSec и vendored-парсере SLAXML.
-- Lua не содержит HTTPS и JSON в стандартной библиотеке: транспорт даёт
-- установленный `ssl.https`, XML разбирается `benchmarks/lib/slaxml.lua`,
-- а небольшой JSON-парсер ниже строит весь документ без внешнего модуля.

local here = arg[0]:match('^(.*)/[^/]+$') or '.'
package.path = here .. '/lib/?.lua;' .. package.path

local https = require('ssl.https')
local socket = require('socket')
local SLAXML = require('slaxml')

local XML_URL = 'https://www.cbr-xml-daily.ru/daily_eng_utf8.xml'
local JSON_URL = 'https://www.cbr-xml-daily.ru/daily_json.js'
local CODES = { 'USD', 'EUR', 'CNY' }

local function fetch(url)
    local body, code = https.request(url)
    if not body or tonumber(code) ~= 200 then
        error('HTTP ' .. tostring(code) .. ': ' .. url)
    end
    return body
end

local function parse_json(document)
    local pos = 1

    local function skip_ws()
        local _, last = document:find('^[ \t\n\r]*', pos)
        pos = last + 1
    end

    local function parse_string()
        pos = pos + 1
        local start = pos
        local parts
        while true do
            local index = document:find('["\\]', pos)
            if not index then error('незакрытая строка JSON') end
            if document:byte(index) == 34 then
                local chunk = document:sub(start, index - 1)
                pos = index + 1
                if parts then
                    parts[#parts + 1] = chunk
                    return table.concat(parts)
                end
                return chunk
            end
            parts = parts or {}
            parts[#parts + 1] = document:sub(start, index - 1)
            local escaped = document:sub(index + 1, index + 1)
            local escapes = {
                ['"'] = '"', ['\\'] = '\\', ['/'] = '/',
                b = '\b', f = '\f', n = '\n', r = '\r', t = '\t',
            }
            if escaped == 'u' then
                local point = tonumber(document:sub(index + 2, index + 5), 16)
                parts[#parts + 1] = utf8.char(point)
                pos = index + 6
            else
                parts[#parts + 1] = escapes[escaped] or escaped
                pos = index + 2
            end
            start = pos
        end
    end

    local parse_value

    local function parse_object()
        pos = pos + 1
        local result = {}
        skip_ws()
        if document:byte(pos) == 125 then
            pos = pos + 1
            return result
        end
        while true do
            skip_ws()
            local key = parse_string()
            skip_ws()
            pos = pos + 1 -- двоеточие
            result[key] = parse_value()
            skip_ws()
            local delimiter = document:byte(pos)
            pos = pos + 1
            if delimiter == 125 then return result end
        end
    end

    local function parse_array()
        pos = pos + 1
        local result = {}
        skip_ws()
        if document:byte(pos) == 93 then
            pos = pos + 1
            return result
        end
        while true do
            result[#result + 1] = parse_value()
            skip_ws()
            local delimiter = document:byte(pos)
            pos = pos + 1
            if delimiter == 93 then return result end
        end
    end

    parse_value = function()
        skip_ws()
        local current = document:byte(pos)
        if current == 123 then
            return parse_object()
        elseif current == 91 then
            return parse_array()
        elseif current == 34 then
            return parse_string()
        elseif current == 116 then
            pos = pos + 4
            return true
        elseif current == 102 then
            pos = pos + 5
            return false
        elseif current == 110 then
            pos = pos + 4
            return nil
        end
        local last = document:find('[^%-%+%d%.eE]', pos) or (#document + 1)
        local number = tonumber(document:sub(pos, last - 1))
        pos = last
        return number
    end

    return parse_value()
end

local function parse_xml(document)
    local rates = {}
    local field, code, nominal, value
    local callbacks = {
        startElement = function(name)
            field = name
            if name == 'Valute' then
                code, nominal, value = nil, nil, nil
            end
        end,
        text = function(text)
            if field == 'CharCode' then
                code = text
            elseif field == 'Nominal' then
                nominal = tonumber(text)
            elseif field == 'Value' then
                value = tonumber((text:gsub(',', '.')))
            end
        end,
        closeElement = function(name)
            if name == 'Valute' then
                if not code or not nominal or not value then
                    error('неполная запись Valute в XML')
                end
                rates[code] = value / nominal
            end
            field = nil
        end,
        pi = function() end,
        comment = function() end,
    }
    SLAXML:parser(callbacks):parse(document, { stripWhitespace = true })
    return rates
end

local function json_rates(document)
    local currencies = parse_json(document).Valute
    local rates = {}
    for code, currency in pairs(currencies) do
        rates[code] = currency.Value / currency.Nominal
    end
    return rates
end

local function presentation(rates)
    local parts = {}
    for _, code in ipairs(CODES) do
        if not rates[code] then error('в ответе ЦБ нет валюты ' .. code) end
        parts[#parts + 1] = string.format('%s=%.4f', code, rates[code])
    end
    return table.concat(parts, '; ')
end

local started = socket.gettime()
local xml_output = presentation(parse_xml(fetch(XML_URL)))
local json_output = presentation(json_rates(fetch(JSON_URL)))
local elapsed_ms = (socket.gettime() - started) * 1000

if xml_output ~= json_output then
    error('курсы в XML и JSON расходятся: ' .. xml_output .. ' / ' .. json_output)
end

print('XML: ' .. xml_output)
print('JSON: ' .. json_output)
print(string.format('%.3f', elapsed_ms))
