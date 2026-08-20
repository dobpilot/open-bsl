-- Двойник str_index.bsl. Итерация по строке: `string.byte(s, i)` —
-- байтовый доступ O(1), native для Lua.
--
-- ВАЖНО про сопоставимость: строки Lua байтовые, наши — код-юниты UTF-16.
-- Кириллица в UTF-8 занимает 2 байта на букву, поэтому Lua проходит вдвое
-- большим числом шагов и суммирует байты, а не коды символов. Это разница
-- семантики, а не измерения: см. README — индексация O(1) на обоих, и
-- число показывает стоимость одного доступа, помноженную на длину хранилища.

local alphabet = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"
local parts = {}
for _ = 1, 10000 do
	parts[#parts + 1] = alphabet
end
local s = table.concat(parts)
local len = #s

local started = os.clock()
local sum = 0
for i = 1, len do
	sum = sum + string.byte(s, i)
end
local elapsed_ms = (os.clock() - started) * 1000

print(string.format("длина (байт): %d, контрольная сумма: %d", len, sum))
print(string.format("%.3f", elapsed_ms))
