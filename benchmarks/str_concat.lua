-- Двойник str_concat.bsl. Именно `s = s .. piece`, а не table.concat:
-- строки Lua тоже неизменяемы, и сравнивать надо один и тот же способ
-- сборки. (Идиоматичный Lua писал бы через table.concat — он линейный, но
-- тогда это был бы бенчмарк другого алгоритма.)

local piece = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя0123456789"

local started = os.clock()
local text = ""
for _ = 1, 3000 do
    text = text .. piece
end
local elapsed_ms = (os.clock() - started) * 1000

print(string.format("длина (байт): %d", #text))
print(string.format("%.3f", elapsed_ms))
