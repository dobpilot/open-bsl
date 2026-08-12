"""Двойник `bmp_rotate.bsl` на `bytearray` и срезах."""

import struct
from time import perf_counter

WIDTH = 600
HEIGHT = 400
BPP = 4
HEADER = 54

started = perf_counter()

pixel_bytes = WIDTH * HEIGHT * BPP
pixels = bytearray(pixel_bytes)
for y in range(HEIGHT):
    row = y * WIDTH * BPP
    for x in range(WIDTH):
        i = row + x * BPP
        pixels[i] = x % 256
        pixels[i + 1] = y % 256
        pixels[i + 2] = (x + y) % 256
        pixels[i + 3] = 255

header = bytearray(HEADER)
header[0:2] = b"BM"
struct.pack_into("<I", header, 2, HEADER + pixel_bytes)
struct.pack_into("<I", header, 10, HEADER)
struct.pack_into("<I", header, 14, 40)
struct.pack_into("<i", header, 18, WIDTH)
struct.pack_into("<i", header, 22, HEIGHT)
struct.pack_into("<H", header, 26, 1)
struct.pack_into("<H", header, 28, 32)
struct.pack_into("<I", header, 34, pixel_bytes)
struct.pack_into("<I", header, 38, 2835)
struct.pack_into("<I", header, 42, 2835)

image = bytearray(HEADER + pixel_bytes)
image[0:HEADER] = header
image[HEADER:] = pixels

# `memoryview` — ближайший аналог `ПолучитьСрез`: окно в те же байты, без
# копирования, ровно за этим срез в оригинале и взят.
source = memoryview(image)[HEADER:]
rotated = bytearray(pixel_bytes)

for y in range(HEIGHT):
    row = y * WIDTH * BPP
    new_column = (HEIGHT - 1 - y) * BPP
    for x in range(WIDTH):
        i = row + x * BPP
        n = new_column + x * HEIGHT * BPP
        rotated[n : n + BPP] = source[i : i + BPP]

rotated_image = bytearray(HEADER + pixel_bytes)
rotated_image[0:HEADER] = header
struct.pack_into("<i", rotated_image, 18, HEIGHT)
struct.pack_into("<i", rotated_image, 22, WIDTH)
rotated_image[HEADER:] = rotated

elapsed_ms = (perf_counter() - started) * 1_000

checksum = 0
for n in range(WIDTH * HEIGHT):
    word = int.from_bytes(rotated[n * BPP : n * BPP + BPP], "little")
    checksum = (checksum * 31 + word) % 2147483647

print(
    f"BMP {HEIGHT}x{WIDTH}, байтов: {len(rotated_image)}, "
    f"контрольная сумма: {checksum}"
)
print(f"{elapsed_ms:.3f}")
