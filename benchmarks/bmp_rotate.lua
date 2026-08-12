-- Двойник bmp_rotate.bsl.
--
-- ПРО СОПОСТАВИМОСТЬ. У Lua нет байтового буфера вообще: ни изменяемых
-- строк, ни типа вроде БуферДвоичныхДанных. Поэтому картинка здесь —
-- обычная таблица чисел по байту на элемент, а «блочная запись пиксела»
-- разворачивается в четыре присваивания. Это самый быстрый способ, какой у
-- Lua есть для этой задачи, но меряет он не то же самое: в версии на BSL
-- один вызов `ПолучитьСрез` и один `Записать` на пиксел, здесь — восемь
-- обращений к таблице и ни одного вызова. Читать колонку как «Lua
-- поворачивает картинки быстрее» нельзя.
--
-- string.pack не используется намеренно: он есть в 5.3+, а двойники
-- гоняются и под LuaJIT (диалект 5.1). Байты заголовка раскладываются
-- вручную.

local floor = math.floor

local WIDTH = 600
local HEIGHT = 400
local BPP = 4
local HEADER = 54

local started = os.clock()

local pixel_bytes = WIDTH * HEIGHT * BPP
local pixels = {}
for y = 0, HEIGHT - 1 do
    local row = y * WIDTH * BPP
    for x = 0, WIDTH - 1 do
        local i = row + x * BPP
        pixels[i + 1] = x % 256
        pixels[i + 2] = y % 256
        pixels[i + 3] = (x + y) % 256
        pixels[i + 4] = 255
    end
end

-- Целое в little-endian по месту: аналог ЗаписатьЦелое16/32.
local function put(buffer, pos, value, width)
    for k = 0, width - 1 do
        buffer[pos + k + 1] = floor(value / 256 ^ k) % 256
    end
end

local header = {}
for i = 1, HEADER do
    header[i] = 0
end
header[1] = 66 -- B
header[2] = 77 -- M
put(header, 2, HEADER + pixel_bytes, 4)
put(header, 10, HEADER, 4)
put(header, 14, 40, 4)
put(header, 18, WIDTH, 4)
put(header, 22, HEIGHT, 4)
put(header, 26, 1, 2)
put(header, 28, 32, 2)
put(header, 34, pixel_bytes, 4)
put(header, 38, 2835, 4)
put(header, 42, 2835, 4)

local image = {}
for i = 1, HEADER do
    image[i] = header[i]
end
for i = 1, pixel_bytes do
    image[HEADER + i] = pixels[i]
end

local rotated = {}
for y = 0, HEIGHT - 1 do
    local row = y * WIDTH * BPP
    local new_column = (HEIGHT - 1 - y) * BPP
    for x = 0, WIDTH - 1 do
        local i = HEADER + row + x * BPP
        local n = new_column + x * HEIGHT * BPP
        rotated[n + 1] = image[i + 1]
        rotated[n + 2] = image[i + 2]
        rotated[n + 3] = image[i + 3]
        rotated[n + 4] = image[i + 4]
    end
end

local rotated_image = {}
for i = 1, HEADER do
    rotated_image[i] = header[i]
end
put(rotated_image, 18, HEIGHT, 4)
put(rotated_image, 22, WIDTH, 4)
for i = 1, pixel_bytes do
    rotated_image[HEADER + i] = rotated[i]
end

local elapsed = (os.clock() - started) * 1000

local checksum = 0
for n = 0, WIDTH * HEIGHT - 1 do
    local i = n * BPP
    local word = rotated[i + 1]
        + rotated[i + 2] * 256
        + rotated[i + 3] * 65536
        + rotated[i + 4] * 16777216
    checksum = (checksum * 31 + word) % 2147483647
end

print(string.format("BMP %dx%d, байтов: %d, контрольная сумма: %d",
    HEIGHT, WIDTH, #rotated_image, checksum))
print(string.format("%.3f", elapsed))
