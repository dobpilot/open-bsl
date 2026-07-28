-- Двойник str_find.bsl. `string.find` с plain = true — поиск без шаблонов,
-- то же, что делает СтрНайти.
--
-- ВАЖНО про сопоставимость: строки Lua байтовые, наши — код-юниты UTF-16.
-- Кириллица в UTF-8 занимает 2 байта на букву, у нас — 2 байта на код-юнит,
-- так что объём просматриваемой памяти совпадает, а вот число ЭЛЕМЕНТОВ у
-- Lua вдвое больше. Это разница семантики, а не измерения: см. README.

local piece = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя0123456789"
local parts = {}
for _ = 1, 5000 do
    parts[#parts + 1] = piece
end
local hay = table.concat(parts) .. "ИГОЛКА"

local started = os.clock()
local pos = 0
for _ = 1, 500 do
    pos = string.find(hay, "ИГОЛКА", 1, true)
end
local elapsed_ms = (os.clock() - started) * 1000

print(string.format("длина стога (байт): %d, позиция: %d", #hay, pos))
print(string.format("%.3f", elapsed_ms))
