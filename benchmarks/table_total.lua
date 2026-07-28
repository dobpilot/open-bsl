-- Двойник table_total.bsl. Ближайший аналог ТаблицыЗначений в Lua —
-- массив записей: колонок как отдельных сущностей у Lua нет, поэтому и
-- хранение получается СТРОЧНОЕ, а не колоночное. Это тоже часть разницы
-- семантик, а не измерения (см. README).

local ROWS = 200000
local PASSES = 50

local started = os.clock()

local t = {}
for i = 1, ROWS do
    t[i] = { nom = i, name = "строка" }
end

local total = 0
for _ = 1, PASSES do
    total = 0
    for i = 1, ROWS do
        total = total + t[i].nom
    end
end

local elapsed_ms = (os.clock() - started) * 1000

print(string.format("строк: %d, итог: %d", #t, total))
print(string.format("%.3f", elapsed_ms))
