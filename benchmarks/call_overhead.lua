-- Двойник call_overhead.bsl: тот же миллион вызовов с тем же телом.
-- Функция ГЛОБАЛЬНАЯ, а не local: в BSL все функции модуля видны по
-- имени, и local-функция Lua (её вызов дешевле на поиск в таблице
-- глобалов) сравнивала бы разные вещи.

function add(a, b)
    return a + b
end

local started = os.clock()
local sum = 0
for i = 1, 1000000 do
    sum = add(sum, i)
end
local elapsed_ms = (os.clock() - started) * 1000

print(string.format("сумма: %d", sum))
print(string.format("%.0f", elapsed_ms))
