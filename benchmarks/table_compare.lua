-- Двойник table_compare.bsl.
--
-- Генератор заполняет в каждой строке ровно одну из 18 колонок. Поэтому
-- логическая строка хранится как пара «номер заполненной колонки, значение»:
-- остальные 17 значений неявно равны Неопределено. Плотный массив из 18
-- ячеек на 1,8 млн строк измерял бы главным образом аллокатор Lua и не
-- помещался бы в адресное пространство GC LuaJIT.
--
-- Свёртка всё равно выполняет настоящую группировку: отдельное соответствие
-- для каждой колонки сопоставляет значению слот группы. После неё ответ
-- копируется и физически переставляется по четырём измерениям.

local COLUMN_COUNT = 18
local DIMENSION_COUNT = 4
local ROW_COUNT = 50000

local function new_sparse_table()
    return { columns = {}, values = {}, size = 0 }
end

local generation_started = os.clock()

local column_names = {}
for column = 1, COLUMN_COUNT do
    column_names[column] = "Колонка_" .. column
end

local table0 = new_sparse_table()
local table1 = new_sparse_table()
for row = 1, ROW_COUNT do
    for column = 1, COLUMN_COUNT do
        local pos = table0.size + 1
        local key = row .. "_" .. column_names[column]
        table0.columns[pos] = column
        table0.values[pos] = "A_" .. key
        table1.columns[pos] = column
        table1.values[pos] = "B_" .. key
        table0.size = pos
        table1.size = pos
    end
end

local generation_ms = (os.clock() - generation_started) * 1000
print(string.format(
    "генерация: %.0f мс, строк в каждой таблице: %d",
    generation_ms,
    table0.size
))

local comparison_started = os.clock()

-- Копия правой таблицы со знаком 1, затем строки левой со знаком -1.
local columns = {}
local values = {}
local signs = {}
local counts = {}
local size = table1.size
for pos = 1, table1.size do
    columns[pos] = table1.columns[pos]
    values[pos] = table1.values[pos]
    signs[pos] = 1
end
for source_pos = 1, table0.size do
    local target_pos = size + source_pos
    columns[target_pos] = table0.columns[source_pos]
    values[target_pos] = table0.values[source_pos]
    signs[target_pos] = -1
end
size = size + table0.size
for pos = 1, size do
    counts[pos] = 1
end

-- Группировка по всем логическим колонкам. Пара «колонка, значение» однозначно
-- задаёт строку, потому что остальные значения строки равны Неопределено.
local slots_by_column = {}
for column = 1, COLUMN_COUNT do
    slots_by_column[column] = {}
end

local group_count = 0
for read_pos = 1, size do
    local column = columns[read_pos]
    local slots = slots_by_column[column]
    local slot = slots[values[read_pos]]
    if slot then
        signs[slot] = signs[slot] + signs[read_pos]
        counts[slot] = counts[slot] + counts[read_pos]
    else
        group_count = group_count + 1
        slots[values[read_pos]] = group_count
        if group_count ~= read_pos then
            columns[group_count] = column
            values[group_count] = values[read_pos]
            signs[group_count] = signs[read_pos]
            counts[group_count] = counts[read_pos]
        end
    end
end
for pos = group_count + 1, size do
    columns[pos] = nil
    values[pos] = nil
    signs[pos] = nil
    counts[pos] = nil
end
size = group_count
slots_by_column = nil
collectgarbage("collect")

-- Аналог Скопировать(Счет = 1): в ответ не попадает служебная колонка Счет.
local answer_columns = {}
local answer_values = {}
local answer_signs = {}
local answer_size = 0
for pos = 1, size do
    if counts[pos] == 1 then
        answer_size = answer_size + 1
        answer_columns[answer_size] = columns[pos]
        answer_values[answer_size] = values[pos]
        answer_signs[answer_size] = signs[pos]
    end
end

-- Устойчивая сортировка по четырём измерениям. table.sort сама неустойчива,
-- поэтому исходная позиция служит последним ключом при равных измерениях.
local order = {}
for pos = 1, answer_size do
    order[pos] = pos
end
table.sort(order, function(left, right)
    local left_column = answer_columns[left]
    local right_column = answer_columns[right]
    for dimension = 1, DIMENSION_COUNT do
        local left_has_value = left_column == dimension
        local right_has_value = right_column == dimension
        if left_has_value ~= right_has_value then
            return not left_has_value -- Неопределено идёт перед строкой.
        end
        if left_has_value then
            local left_value = answer_values[left]
            local right_value = answer_values[right]
            if left_value ~= right_value then
                return left_value < right_value
            end
        end
    end
    return left < right
end)

local sorted_columns = {}
local sorted_values = {}
local sorted_signs = {}
for pos = 1, answer_size do
    local source_pos = order[pos]
    sorted_columns[pos] = answer_columns[source_pos]
    sorted_values[pos] = answer_values[source_pos]
    sorted_signs[pos] = answer_signs[source_pos]
end
answer_columns = sorted_columns
answer_values = sorted_values
answer_signs = sorted_signs

local comparison_ms = (os.clock() - comparison_started) * 1000
print(string.format(
    "сравнение: %.0f мс, строк результата: %d",
    comparison_ms,
    answer_size
))
print(string.format("%.3f", comparison_ms))
