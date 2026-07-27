local started = os.clock()

local file = assert(io.open("test.csv", "w+"))

local data = {
    d1 = "1",
    d2 = "2",
    d3 = "3",
    d4 = "4",
    d5 = "5",
    d6 = "6",
    d7 = "7",
    d8 = "8",
    d9 = "9",
    d10 = "10",
    d11 = "11",
    d12 = "12",
    d13 = "13",
    d14 = "14",
    d15 = "15",
    d16 = "16",
    d17 = "17",
    d18 = "18",
    d19 = "19",
    d20 = "20",
}

io.output(file)

for _ = 0, 300000 do
    io.write(data.d1)
    io.write(";")
    io.write(data.d2)
    io.write(";")
    io.write(data.d3)
    io.write(";")
    io.write(data.d4)
    io.write(";")
    io.write(data.d5)
    io.write(";")
    io.write(data.d6)
    io.write(";")
    io.write(data.d7)
    io.write(";")
    io.write(data.d8)
    io.write(";")
    io.write(data.d9)
    io.write(";")
    io.write(data.d10)
    io.write(";")
    io.write(data.d11)
    io.write(";")
    io.write(data.d12)
    io.write(";")
    io.write(data.d13)
    io.write(";")
    io.write(data.d13)
    io.write(";")
    io.write(data.d14)
    io.write(";")
    io.write(data.d15)
    io.write(";")
    io.write(data.d16)
    io.write(";")
    io.write(data.d17)
    io.write(";")
    io.write(data.d18)
    io.write(";")
    io.write(data.d19)
    io.write(";")
    io.write(data.d20)
    io.write("\n")
end

io.close(file)

print(string.format("elapsed time: %.2f seconds", os.clock() - started))
