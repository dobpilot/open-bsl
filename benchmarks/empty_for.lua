local started = os.clock()

for _ = 1, 10 do
    for _ = 0, 1000000 do
    end
end

print(string.format("%.3f", (os.clock() - started) * 1000 / 10))
