-- Двойник goto_bench.bsl: тот же граф из восьми блоков и те же
-- условные и безусловные переходы. Все локальные объявлены до первой
-- метки, поэтому Lua разрешает обратный переход к диспетчеру.

local iteration = 0
local state = 0
local sum = 0
local started = os.clock()

::dispatcher::
if state == 0 then goto branch0 end
if state == 1 then goto branch1 end
if state == 2 then goto branch2 end
if state == 3 then goto branch3 end
if state == 4 then goto branch4 end
if state == 5 then goto branch5 end
if state == 6 then goto branch6 end
goto branch7

::branch0::
sum = sum + 1
state = 1
goto next_iteration

::branch1::
sum = sum + 3
state = 2
goto next_iteration

::branch2::
sum = sum + 5
state = 3
goto next_iteration

::branch3::
sum = sum + 7
state = 4
goto next_iteration

::branch4::
sum = sum + 11
state = 5
goto next_iteration

::branch5::
sum = sum + 13
state = 6
goto next_iteration

::branch6::
sum = sum + 17
state = 7
goto next_iteration

::branch7::
sum = sum + 19
state = 0

::next_iteration::
iteration = iteration + 1
if iteration < 2000000 then goto dispatcher end

local elapsed_ms = (os.clock() - started) * 1000
print(string.format("сумма: %d", sum))
print(string.format("%.3f", elapsed_ms))
