-- Двойник table_sort.bsl.
--
-- Две разницы семантики, из-за которых Lua здесь заведомо в выигрыше и это
-- НЕ повод «догонять» (см. README):
--   * table.sort неустойчива, наша Сортировать устойчива и обязана такой
--     остаться — от этого зависит порядок при равных ключах;
--   * сравнение строк в Lua побайтовое, у нас — через collate (приближение
--     локали: сперва ВРег, потом исходный вид), то есть две операции
--     приведения регистра на каждое сравнение.

local ROWS = 100000

local t = {}
local seed = 1
for i = 1, ROWS do
    seed = (seed * 1103515245 + 12345) % 2147483648
    t[i] = { key = seed, name = "имя" .. i }
end

local started = os.clock()

table.sort(t, function(a, b) return a.key < b.key end)
table.sort(t, function(a, b) return a.key > b.key end)
table.sort(t, function(a, b) return a.name < b.name end)

local needle = "имя" .. ROWS
local found = nil
for i = 1, #t do
    if t[i].name == needle then
        found = t[i]
        break
    end
end

local elapsed_ms = (os.clock() - started) * 1000

print(string.format("строк: %d, нашлась: %s", #t, tostring(found ~= nil)))
print(string.format("%.3f", elapsed_ms))
