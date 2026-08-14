-- Двойник regexp.bsl: те же три шаблона regex-benchmark по тому же тексту,
-- счёт всех совпадений.
--
-- ПРО СОПОСТАВИМОСТЬ. Движка регулярных выражений у Lua нет, а у
-- Lua-шаблонов нет ни альтернации, ни необязательных подвыражений, поэтому
-- дословно перенести шаблоны нельзя — но работу перенести можно:
--   * почта — Lua-шаблон с теми же классами; `\w` исходника юникодный,
--     поэтому класс расширен байтами \128-\255, как в правках slaxml
--     (см. lib/README.md): иначе адрес с буквой вне ASCII рвётся на
--     части и счёт расходится;
--   * URI — базовый Lua-шаблон, а необязательные хвосты `?запрос` и
--     `#фрагмент` дотягиваются вручную от конца базового совпадения, с
--     возобновлением поиска за хвостом — так же двигается findall;
--   * IPv4 — ручной сопоставитель с порядком ветвей и жадностью исходной
--     альтернации 25[0-5] | 2[0-4]цифра | [01]?цифра цифра?: Lua-шаблоном
--     это не выразить, а менять грамматику значит считать другое.
-- Числа совпадений обязаны сходиться с regexp.bsl построчно.

local PATH = "benchmarks/data/input_regexp.txt"
local f = assert(io.open(PATH, "rb"))
local text = f:read("*a")
f:close()

local started = os.clock()

-- Почта: [\w\.+-]+@[\w\.-]+\.[\w\.-]+
local emails = 0
for _ in text:gmatch("[%w_.+%-\128-\255]+@[%w_.%-\128-\255]+%.[%w_.%-\128-\255]+") do
    emails = emails + 1
end

-- URI: [\w]+://[^/\s?#]+[^\s?#]+(?:\?[^\s#]*)?(?:#[^\s]*)?
local uris = 0
do
    local pos = 1
    while true do
        local s, e = text:find("[%w_\128-\255]+://[^/%s?#]+[^%s?#]+", pos)
        if not s then break end
        if text:byte(e + 1) == 63 then -- «?»: хвост запроса
            local _, q = text:find("^%?[^%s#]*", e + 1)
            e = q
        end
        if text:byte(e + 1) == 35 then -- «#»: хвост фрагмента
            local _, h = text:find("^#%S*", e + 1)
            e = h
        end
        uris = uris + 1
        pos = e + 1
    end
end

-- IPv4. Допустимые длины октета в позиции i — в порядке приоритета ветвей
-- исходного шаблона; nil, если в позиции нет цифры.
local function octet_lens(s, i)
    local b1, b2, b3 = s:byte(i, i + 2)
    if not b1 or b1 < 48 or b1 > 57 then return nil end
    local d2 = b2 ~= nil and b2 >= 48 and b2 <= 57
    local d3 = b3 ~= nil and b3 >= 48 and b3 <= 57
    local lens = {}
    if b1 == 50 then
        if b2 == 53 and b3 ~= nil and b3 >= 48 and b3 <= 53 then
            lens[#lens + 1] = 3 -- 25[0-5]
        end
        if b2 ~= nil and b2 >= 48 and b2 <= 52 and d3 then
            lens[#lens + 1] = 3 -- 2[0-4][0-9]
        end
    end
    if (b1 == 48 or b1 == 49) and d2 and d3 then
        lens[#lens + 1] = 3 -- [01][0-9][0-9]
    end
    if d2 then
        lens[#lens + 1] = 2
    end
    lens[#lens + 1] = 1
    return lens
end

-- Конец совпадения всего шаблона с позиции i или nil: четыре октета через
-- точки, перебор длин каждого октета с откатом, как у бэктрекинга.
local function ip_end_at(s, i)
    local function try(oct, j)
        local lens = octet_lens(s, j)
        if not lens then return nil end
        for k = 1, #lens do
            local nj = j + lens[k]
            if oct == 4 then
                return nj - 1
            elseif s:byte(nj) == 46 then -- «.»
                local r = try(oct + 1, nj + 1)
                if r then return r end
            end
        end
        return nil
    end
    return try(1, i)
end

local ips = 0
do
    local i = 1
    while true do
        i = text:find("%d", i)
        if not i then break end
        local e = ip_end_at(text, i)
        if e then
            ips = ips + 1
            i = e + 1
        else
            i = i + 1
        end
    end
end

local elapsed_ms = (os.clock() - started) * 1000

print(string.format("почта: %d", emails))
print(string.format("URI: %d", uris))
print(string.format("IP: %d", ips))
print(string.format("%.3f", elapsed_ms))
