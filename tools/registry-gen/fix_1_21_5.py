#!/usr/bin/env python3
"""Regenerate registries_1_21_5.bin with correct 1.21.5-schema content.

The original bundle was filtered from the 1.21.11 codec, so dimension_type,
worldgen/biome, enchantment and jukebox_song carried 1.21.11-schema data that
a 1.21.5 client rejects (missing has_raids/sky_color/…, unknown
post_piercing_attack, etc.). 1.21.5 keeps the 1.21.4 schema for those (no
RegistryDataRewriter addHandler between 1.21.4 and 1.21.5), and only ADDS the
six mob-variant registries — which already validate fine.

So: take the broken bundle, replace those 4 registry bodies with the 1.21.4
versions (registries_1_21_3.bin), wrapping biome effects.music into the
1.21.4+ weighted-list form, and keep everything else (incl. the mob-variant
registries) untouched.
"""
import struct, sys, os

HERE = os.path.dirname(__file__)
DATA = os.path.join(HERE, "..", "..", "crates", "protocol", "data")
SRC_1214 = os.path.join(DATA, "registries_1_21_3.bin")   # 1.21.4 (correct schema)
SRC_1215 = os.path.join(DATA, "registries_1_21_5.bin")   # current (1.21.11-filtered)
OUT      = os.path.join(DATA, "registries_1_21_5.bin")

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

def wvarint(out, v):
    v &= 0xFFFFFFFF
    while True:
        x = v & 0x7F; v >>= 7
        out.append(x | 0x80 if v else x)
        if not v: break

def rstr(b, i):
    l, i = rvarint(b, i); return b[i:i+l].decode(), i+l

def wstr(out, s):
    bs = s.encode(); wvarint(out, len(bs)); out += bs

def split_bundle(path):
    """Return ordered list of (reg_id, body_bytes)."""
    d = open(path, "rb").read(); off = 0
    n = struct.unpack(">I", d[off:off+4])[0]; off += 4
    regs = []
    for _ in range(n):
        ln = struct.unpack(">I", d[off:off+4])[0]; off += 4
        body = d[off:off+ln]; off += ln
        rid, _ = rstr(body, 0)
        regs.append((rid, body))
    return regs

# --- nameless NBT parse / encode -------------------------------------------
TAGID = {'end':0,'byte':1,'short':2,'int':3,'long':4,'float':5,'double':6,
         'byteArray':7,'string':8,'list':9,'compound':10,'intArray':11,'longArray':12}

def ppayload(t, b, i):
    if t==1: return ('byte', struct.unpack('>b',b[i:i+1])[0]), i+1
    if t==2: return ('short', struct.unpack('>h',b[i:i+2])[0]), i+2
    if t==3: return ('int', struct.unpack('>i',b[i:i+4])[0]), i+4
    if t==4: return ('long', struct.unpack('>q',b[i:i+8])[0]), i+8
    if t==5: return ('float', struct.unpack('>f',b[i:i+4])[0]), i+4
    if t==6: return ('double', struct.unpack('>d',b[i:i+8])[0]), i+8
    if t==7:
        n=struct.unpack('>i',b[i:i+4])[0]; i+=4
        return ('byteArray', list(struct.unpack('>%db'%n, b[i:i+n]))), i+n
    if t==8:
        l=struct.unpack('>H',b[i:i+2])[0]; i+=2
        return ('string', b[i:i+l].decode()), i+l
    if t==9:
        et=b[i]; i+=1; n=struct.unpack('>i',b[i:i+4])[0]; i+=4
        items=[]
        for _ in range(n):
            v,i=ppayload(et,b,i); items.append(v)
        etname=[k for k,vv in TAGID.items() if vv==et][0]
        return ('list', {'etype':etname,'items':items}), i
    if t==10:
        m={}
        while True:
            tt=b[i]; i+=1
            if tt==0: break
            l=struct.unpack('>H',b[i:i+2])[0]; i+=2; k=b[i:i+l].decode(); i+=l
            v,i=ppayload(tt,b,i); m[k]=v
        return ('compound', m), i
    if t==11:
        n=struct.unpack('>i',b[i:i+4])[0]; i+=4
        return ('intArray', list(struct.unpack('>%di'%n,b[i:i+4*n]))), i+4*n
    if t==12:
        n=struct.unpack('>i',b[i:i+4])[0]; i+=4
        return ('longArray', list(struct.unpack('>%dq'%n,b[i:i+8*n]))), i+8*n
    raise ValueError("tag %d" % t)

def wpayload(out, node):
    t, val = node
    if t=='byte': out+=struct.pack('>b',val)
    elif t=='short': out+=struct.pack('>h',val)
    elif t=='int': out+=struct.pack('>i',val)
    elif t=='long': out+=struct.pack('>q',val)
    elif t=='float': out+=struct.pack('>f',val)
    elif t=='double': out+=struct.pack('>d',val)
    elif t=='string':
        bs=val.encode(); out+=struct.pack('>H',len(bs)); out+=bs
    elif t=='byteArray':
        out+=struct.pack('>i',len(val))
        for x in val: out+=struct.pack('>b',x)
    elif t=='intArray':
        out+=struct.pack('>i',len(val))
        for x in val: out+=struct.pack('>i',x)
    elif t=='longArray':
        out+=struct.pack('>i',len(val))
        for x in val: out+=struct.pack('>q',x)
    elif t=='list':
        et=val['etype']; items=val['items']
        out+=struct.pack('>B',TAGID[et]); out+=struct.pack('>i',len(items))
        for it in items: wpayload(out, it)
    elif t=='compound':
        for k, node2 in val.items():
            out+=struct.pack('>B',TAGID[node2[0]])
            kb=k.encode(); out+=struct.pack('>H',len(kb)); out+=kb
            wpayload(out, node2)
        out+=b'\x00'
    else:
        raise ValueError(t)

def wrap_music(elem):
    """elem is ('compound', {...}); wrap effects.music object -> weighted list."""
    if elem[0] != 'compound': return
    eff = elem[1].get('effects')
    if not eff or eff[0] != 'compound': return
    music = eff[1].get('music')
    if music and music[0] == 'compound':
        weighted = ('compound', {'data': music, 'weight': ('int', 1)})
        eff[1]['music'] = ('list', {'etype':'compound','items':[weighted]})

def transform_biome_body(body):
    """Rebuild a biome RegistryData body wrapping each music object."""
    i = 0
    reg, i = rstr(body, i)
    out = bytearray(); wstr(out, reg)
    cnt, i = rvarint(body, i)
    wvarint(out, cnt)
    for _ in range(cnt):
        key, i = rstr(body, i); wstr(out, key)
        has = body[i]; i += 1; out.append(has)
        if not has: continue
        assert body[i] == 0x0a, "expected nameless compound"
        i += 1
        elem, i = ppayload(10, body, i)
        wrap_music(elem)
        out.append(0x0a); wpayload(out, elem)
    return bytes(out)

# --- main ------------------------------------------------------------------
cur = split_bundle(SRC_1215)
v1214 = dict(split_bundle(SRC_1214))

new_regs = []
for rid, body in cur:
    if rid in REPLACE:
        src = v1214[rid]
        if rid == "minecraft:worldgen/biome":
            src = transform_biome_body(src)
        new_regs.append((rid, src))
        print("replaced", rid, "->", len(src), "bytes (from 1.21.4)")
    else:
        new_regs.append((rid, body))

bundle = bytearray(); bundle += struct.pack(">I", len(new_regs))
for _, body in new_regs:
    bundle += struct.pack(">I", len(body)); bundle += body
open(OUT, "wb").write(bundle)
print("wrote", OUT, len(bundle), "bytes,", len(new_regs), "registries")
print("registry order:", [r for r,_ in new_regs])
