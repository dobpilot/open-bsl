local function calc()
    local sum = 0.0
    local flip = -1.0

    for i = 1, 1000000 do
        flip = -flip
        sum = sum + flip / (2 * i - 1)
    end

    print(string.format("%.15f", sum * 4))
end

local started = os.clock()
calc()
local elapsed_ms = (os.clock() - started) * 1000

print(string.format("%.3f", elapsed_ms))
