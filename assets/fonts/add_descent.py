"""Gives Press Start 2P - renamed, as its licence asks of a modified face - a descent of 250 units - 2 px at 8 px - leaving every
glyph as it is: the face ships with none, so two lines set in it touch."""
import struct, sys

src, dst = sys.argv[1], sys.argv[2]
d = bytearray(open(src, 'rb').read())
n = struct.unpack('>H', d[4:6])[0]
tables = {}
for i in range(n):
    tag, _, off, ln = struct.unpack('>4sIII', d[12 + 16 * i:28 + 16 * i])
    tables[tag.decode('latin-1')] = (off, ln, 12 + 16 * i)
hh = tables['hhea'][0]
struct.pack_into('>h', d, hh + 6, -250)  # descender
os2 = tables['OS/2'][0]
struct.pack_into('>h', d, os2 + 70, -250)  # sTypoDescender
struct.pack_into('>H', d, os2 + 76, 250)  # usWinDescent
# The OFL reserves the name 'Press Start 2P': a modified face may not carry
# it. Renamed in place, the same length in every encoding the table holds.
nm = tables['name'][0]
for old, new in [(b'Press Start 2P', b'SNES Pixel 8px'), (b'PressStart2P', b'SNESPixel8px')]:
    for o, n2 in [(old, new), (old.decode().encode('utf-16-be'), new.decode().encode('utf-16-be'))]:
        start = nm
        end = nm + tables['name'][1]
        while True:
            i = d.find(o, start, end)
            if i < 0:
                break
            d[i:i + len(o)] = n2
            start = i + len(n2)
# Recompute the tables' checksums so a strict reader still takes them.
for tag in ('hhea', 'OS/2', 'name'):
    off, ln, rec = tables[tag]
    data = bytes(d[off:off + ln]) + b'\0' * (-ln % 4)
    cs = sum(struct.unpack('>%dI' % (len(data) // 4), data)) & 0xFFFFFFFF
    struct.pack_into('>I', d, rec + 4, cs)
open(dst, 'wb').write(d)
print('ok')
