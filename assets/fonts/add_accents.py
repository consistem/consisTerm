"""Adds the Latin-1 accented letters a pixel font lacks, drawn on its own grid.

Each new glyph is the base letter's outline (an `i` or `j` without its dot)
plus the accent as pixel squares, one font pixel per square. Only letters the
font does not map already are added. Usage: accents.py in.ttf out.ttf
"""
import struct, sys
from math import gcd
from functools import reduce

ACCENTS = {
    "grave": ["#.", ".#"],
    "acute": [".#", "#."],
    "circ": [".#.", "#.#"],
    "tilde": [".#.#", "#.#."],
    "diaer": ["#.#"],
}
CEDILLA = [".#", "..#", "##."]

LETTERS = {}
for base, marks in {
    "a": "grave acute circ tilde diaer", "e": "grave acute circ diaer",
    "i": "grave acute circ diaer", "o": "grave acute circ tilde diaer",
    "u": "grave acute circ diaer", "n": "tilde", "y": "acute diaer",
}.items():
    for m in marks.split():
        LETTERS.setdefault(base, []).append(m)
UNI = {
    ("a", "grave"): 0xE0, ("a", "acute"): 0xE1, ("a", "circ"): 0xE2, ("a", "tilde"): 0xE3, ("a", "diaer"): 0xE4,
    ("e", "grave"): 0xE8, ("e", "acute"): 0xE9, ("e", "circ"): 0xEA, ("e", "diaer"): 0xEB,
    ("i", "grave"): 0xEC, ("i", "acute"): 0xED, ("i", "circ"): 0xEE, ("i", "diaer"): 0xEF,
    ("n", "tilde"): 0xF1,
    ("o", "grave"): 0xF2, ("o", "acute"): 0xF3, ("o", "circ"): 0xF4, ("o", "tilde"): 0xF5, ("o", "diaer"): 0xF6,
    ("u", "grave"): 0xF9, ("u", "acute"): 0xFA, ("u", "circ"): 0xFB, ("u", "diaer"): 0xFC,
    ("y", "acute"): 0xFD, ("y", "diaer"): 0xFF,
}


def tables(d):
    n = struct.unpack(">H", d[4:6])[0]
    out = {}
    for i in range(n):
        tag, _, off, ln = struct.unpack(">4sIII", d[12 + 16 * i:28 + 16 * i])
        out[tag.decode("latin-1")] = bytearray(d[off:off + ln])
    return out


def read_cmap(t):
    c = bytes(t["cmap"])
    nt = struct.unpack(">H", c[2:4])[0]
    m = {}
    for i in range(nt):
        p, e, o = struct.unpack(">HHI", c[4 + 8 * i:12 + 8 * i])
        if struct.unpack(">H", c[o:o + 2])[0] != 4:
            continue
        s = o
        segx2 = struct.unpack(">H", c[s + 6:s + 8])[0]
        seg = segx2 // 2
        ends = struct.unpack(">%dH" % seg, c[s + 14:s + 14 + segx2])
        st = s + 16 + segx2
        starts = struct.unpack(">%dH" % seg, c[st:st + segx2])
        deltas = struct.unpack(">%dh" % seg, c[st + segx2:st + 2 * segx2])
        ro = st + 2 * segx2
        ranges = struct.unpack(">%dH" % seg, c[ro:ro + segx2])
        for k in range(seg):
            for ch in range(starts[k], ends[k] + 1):
                if ch == 0xFFFF:
                    continue
                if ranges[k] == 0:
                    g = (ch + deltas[k]) & 0xFFFF
                else:
                    a = ro + 2 * k + ranges[k] + 2 * (ch - starts[k])
                    g = struct.unpack(">H", c[a:a + 2])[0]
                    if g:
                        g = (g + deltas[k]) & 0xFFFF
                if g:
                    m.setdefault(ch, g)
    return m


def write_cmap(m):
    chars = sorted(m)
    segs = []
    for ch in chars:
        if segs and ch == segs[-1][1] + 1 and m[ch] - ch == m[segs[-1][0]] - segs[-1][0]:
            segs[-1][1] = ch
        else:
            segs.append([ch, ch])
    segs.append([0xFFFF, 0xFFFF])
    n = len(segs)
    ends = [e for _, e in segs]
    starts = [s for s, _ in segs]
    deltas = [((m[s] - s) & 0xFFFF) if s != 0xFFFF else 1 for s, _ in segs]
    sr = 2 * (1 << (n.bit_length() - 1))
    body = struct.pack(">HHHH", 2 * n, sr, (n.bit_length() - 1), 2 * n - sr)
    body += struct.pack(">%dH" % n, *ends) + b"\0\0" + struct.pack(">%dH" % n, *starts)
    body += struct.pack(">%dH" % n, *deltas) + struct.pack(">%dH" % n, *([0] * n))
    sub = struct.pack(">HHH", 4, 6 + len(body), 0) + body
    return struct.pack(">HHHHIHHI", 0, 2, 0, 3, 20, 3, 1, 20) + sub


def glyphs(t):
    head = t["head"]
    long_loca = struct.unpack(">h", head[50:52])[0] == 1
    ng = struct.unpack(">H", t["maxp"][4:6])[0]
    loca = t["loca"]
    if long_loca:
        offs = struct.unpack(">%dI" % (ng + 1), loca[:4 * (ng + 1)])
    else:
        offs = [2 * v for v in struct.unpack(">%dH" % (ng + 1), loca[:2 * (ng + 1)])]
    return [bytes(t["glyf"][offs[i]:offs[i + 1]]) for i in range(ng)]


def parse(g):
    """Contours of a simple glyph as lists of (x, y, on)."""
    if not g:
        return []
    nc = struct.unpack(">h", g[:2])[0]
    if nc < 0:
        return None
    ends = struct.unpack(">%dH" % nc, g[10:10 + 2 * nc])
    p = 10 + 2 * nc
    il = struct.unpack(">H", g[p:p + 2])[0]
    p += 2 + il
    npts = ends[-1] + 1 if nc else 0
    flags = []
    while len(flags) < npts:
        f = g[p]; p += 1
        flags.append(f)
        if f & 8:
            r = g[p]; p += 1
            flags += [f] * r
    def coords(short, same):
        nonlocal p
        out, v = [], 0
        for f in flags:
            if f & short:
                d = g[p]; p += 1
                v += d if f & same else -d
            elif not f & same:
                v += struct.unpack(">h", g[p:p + 2])[0]; p += 2
            out.append(v)
        return out
    xs = coords(2, 16)
    ys = coords(4, 32)
    contours, start = [], 0
    for e in ends:
        contours.append([(xs[i], ys[i], flags[i] & 1) for i in range(start, e + 1)])
        start = e + 1
    return contours


def build(contours):
    pts = [p for c in contours for p in c]
    if not pts:
        return b""
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    out = struct.pack(">hhhhh", len(contours), min(xs), min(ys), max(xs), max(ys))
    e = -1
    for c in contours:
        e += len(c)
        out += struct.pack(">H", e)
    out += struct.pack(">H", 0)
    out += bytes(p[2] & 1 for p in pts)
    px = py = 0
    for p in pts:
        out += struct.pack(">h", p[0] - px); px = p[0]
    for p in pts:
        out += struct.pack(">h", p[1] - py); py = p[1]
    if len(out) % 2:
        out += b"\0"
    return out


def bbox(contours):
    pts = [p for c in contours for p in c]
    return min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts)


def squares(rows, x0, y_top, px):
    """Pixel art as one square contour per pixel, clockwise (TrueType fill)."""
    out = []
    for j, row in enumerate(rows):
        for i, ch in enumerate(row):
            if ch == "#":
                x, y = round(x0 + i * px), round(y_top - (j + 1) * px)
                x2, y2 = round(x0 + (i + 1) * px), round(y_top - j * px)
                out.append([(x, y, 1), (x, y2, 1), (x2, y2, 1), (x2, y, 1)])
    return out


def main(src, dst):
    d = open(src, "rb").read()
    t = tables(d)
    cmap = read_cmap(t)
    gl = glyphs(t)
    ng = len(gl)
    hhea = t["hhea"]
    nhm = struct.unpack(">H", hhea[34:36])[0]
    hm = t["hmtx"]
    metrics = [struct.unpack(">Hh", hm[4 * i:4 * i + 4]) for i in range(nhm)]
    for i in range(nhm, ng):
        metrics.append((metrics[-1][0], struct.unpack(">h", hm[4 * nhm + 2 * (i - nhm):4 * nhm + 2 * (i - nhm) + 2])[0]))
    coords = [abs(v) for g in gl if g and parse(g) for c in parse(g) for p in c for v in p[:2] if v]
    px = float(sys.argv[3]) if len(sys.argv) > 3 else reduce(gcd, coords)
    def outline(ch):
        g = cmap.get(ord(ch))
        return parse(gl[g]) if g is not None else None
    xh = bbox(outline("x"))[3]
    cap = bbox(outline("H"))[3]
    def add(code, base_ch, mark_rows, below=False):
        if code in cmap:
            return
        base = outline(base_ch)
        if not base:
            return
        g = cmap[ord(base_ch)]
        upper = base_ch.isupper()
        top = cap if upper else xh
        if base_ch.lower() in "ij" and not upper:
            # The dot goes: everything wholly above the x-height.
            base = [c for c in base if min(p[1] for p in c) < xh]
        x0, y0, x1, y1 = bbox(base)
        w = max(len(r) for r in mark_rows) * px
        centre = (x0 + x1) / 2
        ax = round((centre - w / 2) / px) * px
        if below:
            new = base + squares(mark_rows, ax, 0, px)
        else:
            new = base + squares(mark_rows, ax, top + (0 if upper else px) + len(mark_rows) * px, px)
        gl.append(build(new))
        metrics.append((metrics[g][0], bbox(new)[0]))
        cmap[code] = len(gl) - 1
    for (b, m), code in UNI.items():
        add(code, b, ACCENTS[m])
        up = code - 0x20
        if b != "y" or m == "acute":
            add(up if code != 0xFF else 0x178, b.upper(), ACCENTS[m])
    add(0xE7, "c", CEDILLA, below=True)
    add(0xC7, "C", CEDILLA, below=True)
    ng2 = len(gl)
    glyf, loca, off = b"", [], 0
    for g in gl:
        loca.append(off); glyf += g; off += len(g)
    loca.append(off)
    t["glyf"] = bytearray(glyf)
    t["loca"] = bytearray(struct.pack(">%dI" % len(loca), *loca))
    t["head"][50:52] = struct.pack(">h", 1)
    t["hmtx"] = bytearray(b"".join(struct.pack(">Hh", a, l) for a, l in metrics))
    t["hhea"][34:36] = struct.pack(">H", ng2)
    mp = t["maxp"]
    mp[4:6] = struct.pack(">H", ng2)
    if len(mp) >= 10:
        pts = max((sum(len(c) for c in parse(g)) for g in gl if g and parse(g)), default=0)
        cts = max((len(parse(g)) for g in gl if g and parse(g)), default=0)
        mp[6:8] = struct.pack(">H", max(pts, struct.unpack(">H", mp[6:8])[0]))
        mp[8:10] = struct.pack(">H", max(cts, struct.unpack(">H", mp[8:10])[0]))
    t["post"] = bytearray(struct.pack(">I", 0x30000) + bytes(t["post"][4:32]))
    t["cmap"] = bytearray(write_cmap(cmap))
    for drop in ("hdmx", "LTSH", "VDMX", "DSIG"):
        t.pop(drop, None)
    tags = sorted(t)
    n = len(tags)
    es = n.bit_length() - 1
    out = struct.pack(">IHHHH", 0x10000, n, 16 << es, es, 16 * n - (16 << es))
    off = 12 + 16 * n
    body = b""
    for tag in tags:
        data = bytes(t[tag])
        pad = data + b"\0" * (-len(data) % 4)
        cs = sum(struct.unpack(">%dI" % (len(pad) // 4), pad)) & 0xFFFFFFFF
        out += struct.pack(">4sIII", tag.encode("latin-1"), cs, off + len(body), len(data))
        body += pad
    open(dst, "wb").write(out + body)
    print(src, "px", px, "x-height", xh, "cap", cap, "glyphs", ng, "->", ng2)


main(sys.argv[1], sys.argv[2])
