#!/usr/bin/env python3
"""Rebase the schema-stable registries of TARGET onto a CORRECT lower-version
bundle.

The 1.21.5+ bundles were filtered from the 1.21.11 codec, so dimension_type /
worldgen/biome / enchantment / jukebox_song carry future-schema data the
target client rejects. Those registries don't change schema across the 1.21.x
patch line (no RegistryDataRewriter addHandler between consecutive versions),
so we copy the correct bodies from an already-fixed lower-version bundle and
keep everything else in TARGET (new registries like the mob variants / dialog).

    python3 rebase_registries.py <correct_src.bin> <target.bin>

`correct_src` must already carry the right schema (e.g. registries_1_21_5.bin
after fix_1_21_5.py). Biome music is copied as-is — make sure the source is
already in the 1.21.4+ weighted-list form.
"""
import struct, sys

REPLACE = {
    "minecraft:dimension_type",
    "minecraft:worldgen/biome",
    "minecraft:enchantment",
    "minecraft:jukebox_song",
}

def rvarint(b, i):
    v = s = 0
    while True:
        x = b[i]; i += 1; v |= (x & 0x7F) << s
        if not x & 0x80: break
        s += 7
    return v, i

def rstr(b, i):
    l, i = rvarint(b, i); return b[i:i+l].decode(), i+l

def split_bundle(path):
    d = open(path, "rb").read(); off = 0
    n = struct.unpack(">I", d[off:off+4])[0]; off += 4
    regs = []
    for _ in range(n):
        ln = struct.unpack(">I", d[off:off+4])[0]; off += 4
        body = d[off:off+ln]; off += ln
        rid, _ = rstr(body, 0)
        regs.append((rid, body))
    return regs

correct = dict(split_bundle(sys.argv[1]))
target = split_bundle(sys.argv[2])

out = []
for rid, body in target:
    if rid in REPLACE:
        if rid not in correct:
            sys.exit("source bundle missing %s" % rid)
        out.append((rid, correct[rid]))
        print("rebased", rid, "->", len(correct[rid]), "bytes")
    else:
        out.append((rid, body))

bundle = bytearray(); bundle += struct.pack(">I", len(out))
for _, body in out:
    bundle += struct.pack(">I", len(body)); bundle += body
open(sys.argv[2], "wb").write(bundle)
print("wrote", sys.argv[2], len(bundle), "bytes,", len(out), "registries")
print("order:", [r for r, _ in out])
