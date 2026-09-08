-- Двойник numeric_precision.bsl на двоичных числах, не эмуляция чисел 1С.
-- Округление положительных приближений; точность остаётся двоичной.
local function round_fraction27(value)
    return math.floor(value * 1e27 + 0.5) / 1e27
end

local function small_rump(a, b)
    local b2 = b * b
    local b4 = b2 * b2
    local b6 = b4 * b2
    local b8 = b4 * b4
    local a2 = a * a
    local t1 = 333.75 * b6
    local inner = 11.0 * a2 * b2 - b6 - 121.0 * b4 - 2.0
    local t2 = a2 * inner
    local t3 = 5.5 * b8
    local t4 = a / (2.0 * b)
    local s1 = t1 + t2
    local s2 = s1 + t3
    local result = s2 + t4
    return result
end

local function multiply_rounding()
    local x = 1234567890.123456789
    local y = 9876543210.987654321
    return x * y
end

local function harmonic_series()
    local sum = 0.0
    local n = 1.0
    while n <= 10000000.0 do
        sum = sum + (1.0 / n)
        n = n + 1.0
    end
    return sum
end

local function binet_fibonacci()
    local sqrt5 = 2.2360679774997896964091736687
    local phi = (1.0 + sqrt5) / 2.0
    local psi = (1.0 - sqrt5) / 2.0
    local phi_pow = 1.0
    local psi_pow = 1.0
    local i = 1
    while i <= 75 do
        phi_pow = phi_pow * phi
        psi_pow = psi_pow * psi
        i = i + 1
    end
    return (phi_pow - psi_pow) / sqrt5
end

local function subtractive_cancellation()
    local root_x = 10000000000000.0
    local root_x_plus_1 = 10000000000000.00000000000005
    return root_x_plus_1 - root_x
end

local function calculate_euler()
    local e_val = 1.0
    local term = 1.0
    local i = 1.0
    while term > 0.0 do
        term = term / i
        if term == 0.0 then
            break
        end
        e_val = e_val + term
        i = i + 1.0
    end
    return e_val
end

local function sqrt2_newton()
    local s = 2.0
    local x = 1.0
    local prev_x = 0.0
    while x ~= prev_x do
        prev_x = x
        x = round_fraction27(0.5 * (x + (s / x)))
    end
    return x
end

local function arc_tan_taylor(x)
    local sum = 0.0
    local term = x
    local x2 = x * x
    local n = 1.0
    local sign = 1.0
    while term > 0.0 do
        sum = sum + sign * (term / n)
        term = round_fraction27(term * x2)
        n = n + 2.0
        sign = 0.0 - sign
    end
    return sum
end

local function calculate_pi()
    local part1 = 16.0 * arc_tan_taylor(1.0 / 5.0)
    local part2 = 4.0 * arc_tan_taylor(1.0 / 239.0)
    return part1 - part2
end

local function mortgage_annuity()
    local principal = 1000000000.0
    local annual_rate = 0.035
    local months = 360.0
    local r = annual_rate / 12.0
    local power = 1.0
    local i = 1.0
    while i <= months do
        power = power * (1.0 + r)
        i = i + 1.0
    end
    local pmt = principal * (r * power) / (power - 1.0)
    return pmt
end

local started = os.clock()
print(string.format("small_rump: %.17g", small_rump(77.0, 33.0)))
print(string.format("multiply_rounding: %.17g", multiply_rounding()))
print(string.format("harmonic_series: %.17g", harmonic_series()))
print(string.format("binet_fibonacci: %.17g", binet_fibonacci()))
print(string.format("subtractive_cancellation: %.17g", subtractive_cancellation()))
print(string.format("euler: %.17g", calculate_euler()))
print(string.format("sqrt2_newton: %.17g", sqrt2_newton()))
print(string.format("pi_machin: %.17g", calculate_pi()))
print(string.format("mortgage_annuity: %.17g", mortgage_annuity()))
print(string.format("%.3f", (os.clock() - started) * 1000))
