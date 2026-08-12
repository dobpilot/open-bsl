-- Двойник simple_parquet_reader.bsl: тот же разбор parquet вручную.
--
-- ПРО СОПОСТАВИМОСТЬ. Готового читателя parquet у Lua нет ни в языке, ни в
-- поставке, поэтому здесь построчно повторён тот же разбор, что и в версии
-- на BSL: компактный протокол Thrift в подвале, цепочка страниц данных,
-- значения PLAIN. Это тот редкий случай, когда двойник делает ровно ту же
-- работу, а не её удобное подобие: строки режутся по одной, числа
-- собираются из байтов по одному.
--
-- string.pack и string.unpack не используются намеренно: они появились в
-- 5.3, а двойники гоняются и под LuaJIT, который остаётся диалектом 5.1.
-- По той же причине нет ни `//`, ни побитовых операторов — только
-- арифметика и math.floor, которые понимают обе ветки.
--
-- ЧЕГО У ЭТОГО ДВОЙНИКА НЕТ. Точных целых шире 2^53: числа в Lua — это
-- double (в 5.4 есть целые, в LuaJIT их нет), и 64-разрядное значение за
-- этой границей потеряло бы младшие разряды. В этом файле price лежит в
-- диапазоне 10..100, так что вопрос теоретический, но на чужом файле
-- колонка INT64 читалась бы приблизительно — в отличие от версии на BSL,
-- где число точное десятичное.
--
-- Таблица значений здесь — массив массивов: именованных колонок с типами
-- у Lua нет, ближе к построчной таблице ничего не найти.

local byte = string.byte
local sub = string.sub
local floor = math.floor

local PATH = "benchmarks/data/simple.parquet"

-- Типы полей компактного протокола Thrift.
local T_TRUE, T_FALSE, T_BYTE = 1, 2, 3
local T_I16, T_I32, T_I64, T_DOUBLE = 4, 5, 6, 7
local T_BINARY, T_LIST, T_SET, T_MAP, T_STRUCT = 8, 9, 10, 11, 12

-- Позиции везде 1-базные, как принято в строках Lua, а смещения из файла —
-- 0-базные: перевод делается один раз, в месте использования.

-- Целое переменной длины: по семь бит на байт, младшие вперёд.
local function varint(s, p)
    local out, scale = 0, 1
    while true do
        local b = byte(s, p)
        p = p + 1
        out = out + (b % 128) * scale
        if b < 128 then
            return out, p
        end
        scale = scale * 128
    end
end

-- Знаковое целое: младший бит — знак, остальное сдвинуто влево.
local function zigzag(s, p)
    local n
    n, p = varint(s, p)
    if n % 2 == 0 then
        return n / 2, p
    end
    return -(n + 1) / 2, p
end

local function binary(s, p)
    local n
    n, p = varint(s, p)
    return sub(s, p, p + n - 1), p + n
end

-- Заголовок поля структуры: тетрада приращения номера и тетрада типа.
-- Нулевой байт — конец структуры, и тогда номер нулевой.
local function field(s, p, last)
    local head = byte(s, p)
    p = p + 1
    if head == 0 then
        return 0, 0, p
    end
    local kind = head % 16
    local delta = floor(head / 16)
    local num
    if delta == 0 then
        num, p = zigzag(s, p)
    else
        num = last + delta
    end
    return num, kind, p
end

-- Заголовок списка: количество и тип элемента. Пятнадцать в тетраде
-- означает «длина следующим варинтом».
local function list_header(s, p)
    local head = byte(s, p)
    p = p + 1
    local count = floor(head / 16)
    if count == 15 then
        count, p = varint(s, p)
    end
    return count, head % 16, p
end

local skip

-- Пропуск значения неинтересного поля; структуры вложены, поэтому
-- рекурсия здесь настоящая.
skip = function(s, p, kind)
    if kind == T_TRUE or kind == T_FALSE then
        return p
    elseif kind == T_BYTE then
        return p + 1
    elseif kind == T_I16 or kind == T_I32 or kind == T_I64 then
        local _
        _, p = zigzag(s, p)
        return p
    elseif kind == T_DOUBLE then
        return p + 8
    elseif kind == T_BINARY then
        local _
        _, p = binary(s, p)
        return p
    elseif kind == T_LIST or kind == T_SET then
        local count, item
        count, item, p = list_header(s, p)
        for _ = 1, count do
            p = skip(s, p, item)
        end
        return p
    elseif kind == T_MAP then
        local count
        count, p = varint(s, p)
        if count > 0 then
            local head = byte(s, p)
            p = p + 1
            local key, value = floor(head / 16), head % 16
            for _ = 1, count do
                p = skip(s, p, key)
                p = skip(s, p, value)
            end
        end
        return p
    elseif kind == T_STRUCT then
        local last = 0
        while true do
            local num, ftype
            num, ftype, p = field(s, p, last)
            if num == 0 then
                return p
            end
            last = num
            p = skip(s, p, ftype)
        end
    end
    error("parquet: неизвестный тип поля " .. kind)
end

-- ColumnMetaData: из четырнадцати полей нужны пять. Номера — из
-- parquet.thrift: 1 тип, 3 путь в схеме, 4 кодек, 5 число значений,
-- 9 смещение первой страницы данных, 11 смещение словарной страницы.
local function column_meta(s, p)
    local column = { name = "", kind = -1, codec = 0, values = 0, offset = 0, dict = 0 }
    local last = 0
    while true do
        local num, ftype
        num, ftype, p = field(s, p, last)
        if num == 0 then
            return column, p
        end
        last = num
        if num == 1 then
            column.kind, p = zigzag(s, p)
        elseif num == 3 then
            local count, part
            count, _, p = list_header(s, p)
            for i = 1, count do
                part, p = binary(s, p)
                column.name = i == 1 and part or (column.name .. "." .. part)
            end
        elseif num == 4 then
            column.codec, p = zigzag(s, p)
        elseif num == 5 then
            column.values, p = zigzag(s, p)
        elseif num == 9 then
            column.offset, p = zigzag(s, p)
        elseif num == 11 then
            column.dict, p = zigzag(s, p)
        else
            p = skip(s, p, ftype)
        end
    end
end

-- FileMetaData: число строк (поле 3) и единственная группа строк (поле 4),
-- внутри неё список кусков колонок (поле 1) с вложенной meta_data (3).
local function metadata(s, p)
    local rows, columns = 0, {}
    local last = 0
    while true do
        local num, ftype
        num, ftype, p = field(s, p, last)
        if num == 0 then
            return rows, columns
        end
        last = num
        if num == 3 then
            rows, p = zigzag(s, p)
        elseif num == 4 then
            local groups
            groups, _, p = list_header(s, p)
            if groups ~= 1 then
                error("parquet: ожидалась одна группа строк, а их " .. groups)
            end
            local glast = 0
            while true do
                local gnum, gtype
                gnum, gtype, p = field(s, p, glast)
                if gnum == 0 then
                    break
                end
                glast = gnum
                if gnum == 1 then
                    local chunks
                    chunks, _, p = list_header(s, p)
                    for _ = 1, chunks do
                        local clast = 0
                        while true do
                            local cnum, ctype
                            cnum, ctype, p = field(s, p, clast)
                            if cnum == 0 then
                                break
                            end
                            clast = cnum
                            if cnum == 3 then
                                local column
                                column, p = column_meta(s, p)
                                columns[#columns + 1] = column
                            else
                                p = skip(s, p, ctype)
                            end
                        end
                    end
                else
                    p = skip(s, p, gtype)
                end
            end
        else
            p = skip(s, p, ftype)
        end
    end
end

-- PageHeader: тип (1), сжатый размер (3) и вложенный DataPageHeader (5), из
-- которого нужны число значений и кодирование.
local function page_header(s, p)
    local page = { kind = -1, compressed = 0, values = 0, encoding = -1 }
    local last = 0
    while true do
        local num, ftype
        num, ftype, p = field(s, p, last)
        if num == 0 then
            return page, p
        end
        last = num
        if num == 1 then
            page.kind, p = zigzag(s, p)
        elseif num == 3 then
            page.compressed, p = zigzag(s, p)
        elseif num == 5 then
            local dlast = 0
            while true do
                local dnum, dtype
                dnum, dtype, p = field(s, p, dlast)
                if dnum == 0 then
                    break
                end
                dlast = dnum
                if dnum == 1 then
                    page.values, p = zigzag(s, p)
                elseif dnum == 2 then
                    page.encoding, p = zigzag(s, p)
                else
                    p = skip(s, p, dtype)
                end
            end
        else
            p = skip(s, p, ftype)
        end
    end
end

local function u32(s, p)
    local b1, b2, b3, b4 = byte(s, p, p + 3)
    return b1 + b2 * 256 + b3 * 65536 + b4 * 16777216
end

local function i32(s, p)
    local v = u32(s, p)
    if v >= 2147483648 then
        return v - 4294967296
    end
    return v
end

local function i64(s, p)
    local lo = u32(s, p)
    local hi = u32(s, p + 4)
    if hi >= 2147483648 then
        return lo + (hi - 4294967296) * 4294967296
    end
    return lo + hi * 4294967296
end

-- IEEE 754 двойной точности: знак, одиннадцать бит порядка, пятьдесят два
-- бита мантиссы. Умножение на степень двойки в double точно, поэтому
-- собирается оно прямо так, без возни с делением.
local function double_at(s, p)
    local b1, b2, b3, b4, b5, b6, b7, b8 = byte(s, p, p + 7)
    local exponent = (b8 % 128) * 16 + floor(b7 / 16)
    local mantissa = ((((((b7 % 16) * 256 + b6) * 256 + b5) * 256 + b4) * 256 + b3) * 256 + b2)
        * 256
        + b1
    local value
    if exponent == 0 then
        if mantissa == 0 then
            value = 0
        else
            value = mantissa * 2 ^ -1074
        end
    elseif exponent == 2047 then
        error("parquet: бесконечность или не-число в колонке")
    else
        value = (mantissa + 4503599627370496) * 2 ^ (exponent - 1075)
    end
    if b8 >= 128 then
        return -value
    end
    return value
end

-- Колонка целиком: цепочка страниц, значения подряд, ветвление по типу
-- вынесено из цикла.
local function read_column(s, column)
    if column.codec ~= 0 then
        error("parquet: колонка " .. column.name .. " сжата, кодек " .. column.codec)
    end
    if column.dict > 0 then
        error("parquet: у колонки " .. column.name .. " словарная страница")
    end

    local values = {}
    local total = 0
    local pos = column.offset + 1
    local left = column.values
    while left > 0 do
        local page
        page, pos = page_header(s, pos)
        if page.kind ~= 0 then
            error("parquet: страница типа " .. page.kind .. " в колонке " .. column.name)
        end
        if page.encoding ~= 0 then
            error("parquet: страница колонки " .. column.name
                .. " закодирована способом " .. page.encoding)
        end
        local start = pos
        local count = page.values

        if column.kind == 6 then
            local p = start
            for _ = 1, count do
                local size = u32(s, p)
                p = p + 4
                total = total + 1
                values[total] = sub(s, p, p + size - 1)
                p = p + size
            end
        elseif column.kind == 1 then
            local p = start
            for _ = 1, count do
                total = total + 1
                values[total] = i32(s, p)
                p = p + 4
            end
        elseif column.kind == 2 then
            local p = start
            for _ = 1, count do
                total = total + 1
                values[total] = i64(s, p)
                p = p + 8
            end
        elseif column.kind == 5 then
            local p = start
            for _ = 1, count do
                total = total + 1
                values[total] = double_at(s, p)
                p = p + 8
            end
        else
            error("parquet: тип данных " .. column.kind .. " в колонке "
                .. column.name .. " не поддержан")
        end

        pos = start + page.compressed
        left = left - count
    end
    return values
end

local started = os.clock()

local handle = assert(io.open(PATH, "rb"))
local data = handle:read("*a")
handle:close()

if sub(data, 1, 4) ~= "PAR1" then
    error("parquet: нет подписи PAR1 в начале файла")
end
local footer_len = u32(data, #data - 7)
local row_count, columns = metadata(data, #data - 7 - footer_len)

local by_column = {}
for i = 1, #columns do
    by_column[i] = read_column(data, columns[i])
end

-- Колонки лежат подряд, а таблица построчная: перекладка входит в замер.
local rows = {}
for i = 1, row_count do
    local row = {}
    for c = 1, #columns do
        row[c] = by_column[c][i]
    end
    rows[i] = row
end

local elapsed = (os.clock() - started) * 1000

local index = {}
for i = 1, #columns do
    index[columns[i].name] = i
end

local function total(name)
    local values = by_column[index[name]]
    local sum = 0
    for i = 1, #values do
        sum = sum + values[i]
    end
    return sum
end

local item = index["Item"]
print(string.format("строк: %d, колонок: %d, первая: %s, последняя: %s",
    #rows, #columns, rows[1][item], rows[#rows][item]))
print(string.format("суммы: quantity %.0f, price %.0f, summ %.0f",
    total("quantity"), total("price"), total("summ")))
print(string.format("%.3f", elapsed))
