#!/usr/bin/env python3
"""Build registries_26_1.bin from registries_1_21_11.bin by injecting the
registry fields 26.1 (proto 775) made required.

26.1 schema deltas (minecraft.wiki, verified against the client's registry
load error):
  * dimension_type        + has_ender_dragon_fight (byte; the_end only)
  * cat/chicken/cow/pig_variant + baby_asset_id (string = asset_id)
  * wolf_variant          + baby_assets (copy of assets)
  * wolf_sound_variant    flat *_sound fields -> adult_sounds + baby_sounds
  * timeline              + clock (string world-clock id, minecraft:overworld)

    python3 build_26_1.py <src_1_21_11.bin> <out_26_1.bin>
"""
import struct, sys, json

# ── 26.1 entity sound-variant registries ───────────────────────────────────
# cat/chicken/cow/pig_sound_variant are NEW, empty-by-default registries in
# 26.1; the client's RegistryValidator rejects them unless non-empty
# ("Registry must be non-empty: minecraft:cat_sound_variant"). ViaVersion ships
# their entries in `sound-variant-registries-26.1.nbt` and sends them in its
# FINISH_CONFIGURATION handler. Captured verbatim from that file
# (ViaVersion/common/.../assets/viaversion/data). NOTE: cat/chicken/pig wrap
# their sounds in adult_sounds/baby_sounds; cow uses flat fields.
SOUND_VARIANT_JSON = r'''
{"minecraft:cat_sound_variant":{"minecraft:royal":{"adult_sounds":{"hiss_sound":"minecraft:entity.cat_royal.hiss","eat_sound":"minecraft:entity.cat_royal.eat","purreow_sound":"minecraft:entity.cat_royal.purreow","ambient_sound":"minecraft:entity.cat_royal.ambient","hurt_sound":"minecraft:entity.cat_royal.hurt","death_sound":"minecraft:entity.cat_royal.death","beg_for_food_sound":"minecraft:entity.cat_royal.beg_for_food","purr_sound":"minecraft:entity.cat_royal.purr","stray_ambient_sound":"minecraft:entity.cat_royal.ambient"},"baby_sounds":{"hiss_sound":"minecraft:entity.baby_cat.hiss","eat_sound":"minecraft:entity.baby_cat.eat","purreow_sound":"minecraft:entity.baby_cat.purreow","ambient_sound":"minecraft:entity.baby_cat.ambient","hurt_sound":"minecraft:entity.baby_cat.hurt","death_sound":"minecraft:entity.baby_cat.death","beg_for_food_sound":"minecraft:entity.baby_cat.beg_for_food","purr_sound":"minecraft:entity.baby_cat.purr","stray_ambient_sound":"minecraft:entity.baby_cat.ambient"}},"minecraft:classic":{"adult_sounds":{"hiss_sound":"minecraft:entity.cat.hiss","eat_sound":"minecraft:entity.cat.eat","purreow_sound":"minecraft:entity.cat.purreow","ambient_sound":"minecraft:entity.cat.ambient","hurt_sound":"minecraft:entity.cat.hurt","death_sound":"minecraft:entity.cat.death","beg_for_food_sound":"minecraft:entity.cat.beg_for_food","purr_sound":"minecraft:entity.cat.purr","stray_ambient_sound":"minecraft:entity.cat.ambient"},"baby_sounds":{"hiss_sound":"minecraft:entity.baby_cat.hiss","eat_sound":"minecraft:entity.baby_cat.eat","purreow_sound":"minecraft:entity.baby_cat.purreow","ambient_sound":"minecraft:entity.baby_cat.ambient","hurt_sound":"minecraft:entity.baby_cat.hurt","death_sound":"minecraft:entity.baby_cat.death","beg_for_food_sound":"minecraft:entity.baby_cat.beg_for_food","purr_sound":"minecraft:entity.baby_cat.purr","stray_ambient_sound":"minecraft:entity.baby_cat.ambient"}}},"minecraft:chicken_sound_variant":{"minecraft:picky":{"adult_sounds":{"ambient_sound":"minecraft:entity.chicken_picky.ambient","hurt_sound":"minecraft:entity.chicken_picky.hurt","death_sound":"minecraft:entity.chicken_picky.death","step_sound":"minecraft:entity.chicken.step"},"baby_sounds":{"ambient_sound":"minecraft:entity.baby_chicken.ambient","hurt_sound":"minecraft:entity.baby_chicken.hurt","death_sound":"minecraft:entity.baby_chicken.death","step_sound":"minecraft:entity.baby_chicken.step"}},"minecraft:classic":{"adult_sounds":{"ambient_sound":"minecraft:entity.chicken.ambient","hurt_sound":"minecraft:entity.chicken.hurt","death_sound":"minecraft:entity.chicken.death","step_sound":"minecraft:entity.chicken.step"},"baby_sounds":{"ambient_sound":"minecraft:entity.baby_chicken.ambient","hurt_sound":"minecraft:entity.baby_chicken.hurt","death_sound":"minecraft:entity.baby_chicken.death","step_sound":"minecraft:entity.baby_chicken.step"}}},"minecraft:cow_sound_variant":{"minecraft:moody":{"ambient_sound":"minecraft:entity.cow_moody.ambient","hurt_sound":"minecraft:entity.cow_moody.hurt","death_sound":"minecraft:entity.cow_moody.death","step_sound":"minecraft:entity.cow_moody.step"},"minecraft:classic":{"ambient_sound":"minecraft:entity.cow.ambient","hurt_sound":"minecraft:entity.cow.hurt","death_sound":"minecraft:entity.cow.death","step_sound":"minecraft:entity.cow.step"}},"minecraft:pig_sound_variant":{"minecraft:mini":{"adult_sounds":{"ambient_sound":"minecraft:entity.pig_mini.ambient","hurt_sound":"minecraft:entity.pig_mini.hurt","death_sound":"minecraft:entity.pig_mini.death","step_sound":"minecraft:entity.pig.step","eat_sound":"minecraft:entity.pig.eat"},"baby_sounds":{"ambient_sound":"minecraft:entity.baby_pig.ambient","hurt_sound":"minecraft:entity.baby_pig.hurt","death_sound":"minecraft:entity.baby_pig.death","step_sound":"minecraft:entity.baby_pig.step","eat_sound":"minecraft:entity.baby_pig.eat"}},"minecraft:big":{"adult_sounds":{"ambient_sound":"minecraft:entity.pig_big.ambient","hurt_sound":"minecraft:entity.pig_big.hurt","death_sound":"minecraft:entity.pig_big.death","step_sound":"minecraft:entity.pig.step","eat_sound":"minecraft:entity.pig.eat"},"baby_sounds":{"ambient_sound":"minecraft:entity.baby_pig.ambient","hurt_sound":"minecraft:entity.baby_pig.hurt","death_sound":"minecraft:entity.baby_pig.death","step_sound":"minecraft:entity.baby_pig.step","eat_sound":"minecraft:entity.baby_pig.eat"}},"minecraft:classic":{"adult_sounds":{"ambient_sound":"minecraft:entity.pig.ambient","hurt_sound":"minecraft:entity.pig.hurt","death_sound":"minecraft:entity.pig.death","step_sound":"minecraft:entity.pig.step","eat_sound":"minecraft:entity.pig.eat"},"baby_sounds":{"ambient_sound":"minecraft:entity.baby_pig.ambient","hurt_sound":"minecraft:entity.baby_pig.hurt","death_sound":"minecraft:entity.baby_pig.death","step_sound":"minecraft:entity.baby_pig.step","eat_sound":"minecraft:entity.baby_pig.eat"}}}}
'''

def plain_to_node(v):
    """Convert a plain JSON value (str / dict-of-str) into NBT node form."""
    if isinstance(v, str):
        return ('string', v)
    if isinstance(v, dict):
        return ('compound', {k: plain_to_node(x) for k, x in v.items()})
    raise SystemExit('unsupported sound-variant value: %r' % (v,))

def sound_variant_registries():
    data = json.loads(SOUND_VARIANT_JSON)
    regs = []
    for rid, entries in data.items():
        regs.append((rid, [[key, 1, plain_to_node(comp)] for key, comp in entries.items()]))
    return regs

# ── bundle framing (mc-string varint) ──────────────────────────────────────
def rvarint(b, i):
    v = s = 0
    while True:
        x = b[i]; i += 1; v |= (x & 0x7F) << s
        if not x & 0x80: break
        s += 7
    return v, i

def wvarint(out, v):
    while True:
        b = v & 0x7F; v >>= 7
        out.append(b | 0x80 if v else b)
        if not v: break

def rstr_mc(b, i):
    l, i = rvarint(b, i); return b[i:i+l].decode(), i + l

def wstr_mc(out, s):
    bb = s.encode(); wvarint(out, len(bb)); out += bb

# ── java NBT payloads (2-byte string lengths) ──────────────────────────────
def rstr(b, i):
    l = struct.unpack('>H', b[i:i+2])[0]; i += 2
    return b[i:i+l].decode('utf-8'), i + l

def wstr(out, s):
    bb = s.encode('utf-8'); out += struct.pack('>H', len(bb)); out += bb

def rpayload(b, i, t):
    if t == 1: return ('byte', b[i] - 256 if b[i] > 127 else b[i]), i + 1
    if t == 2: return ('short', struct.unpack('>h', b[i:i+2])[0]), i + 2
    if t == 3: return ('int', struct.unpack('>i', b[i:i+4])[0]), i + 4
    if t == 4: return ('long', struct.unpack('>q', b[i:i+8])[0]), i + 8
    if t == 5: return ('float', struct.unpack('>f', b[i:i+4])[0]), i + 4
    if t == 6: return ('double', struct.unpack('>d', b[i:i+8])[0]), i + 8
    if t == 7:
        n = struct.unpack('>i', b[i:i+4])[0]; i += 4
        return ('byteArray', list(b[i:i+n])), i + n
    if t == 8:
        s, i = rstr(b, i); return ('string', s), i
    if t == 9:
        et = b[i]; i += 1; n = struct.unpack('>i', b[i:i+4])[0]; i += 4
        items = []
        for _ in range(n):
            v, i = rpayload(b, i, et); items.append(v)
        return ('list', (et, items)), i
    if t == 10:
        d = {}
        while True:
            tt = b[i]; i += 1
            if tt == 0: break
            nm, i = rstr(b, i); v, i = rpayload(b, i, tt); d[nm] = v
        return ('compound', d), i
    if t == 11:
        n = struct.unpack('>i', b[i:i+4])[0]; i += 4
        return ('intArray', [struct.unpack('>i', b[i+4*k:i+4*k+4])[0] for k in range(n)]), i + 4*n
    if t == 12:
        n = struct.unpack('>i', b[i:i+4])[0]; i += 4
        return ('longArray', [struct.unpack('>q', b[i+8*k:i+8*k+8])[0] for k in range(n)]), i + 8*n
    raise SystemExit('bad tag %d' % t)

TAGID = {'byte':1,'short':2,'int':3,'long':4,'float':5,'double':6,
         'byteArray':7,'string':8,'list':9,'compound':10,'intArray':11,'longArray':12}

def wpayload(out, node):
    t, v = node
    tid = TAGID[t]
    if t == 'byte': out += struct.pack('>b', v)
    elif t == 'short': out += struct.pack('>h', v)
    elif t == 'int': out += struct.pack('>i', v)
    elif t == 'long': out += struct.pack('>q', v)
    elif t == 'float': out += struct.pack('>f', v)
    elif t == 'double': out += struct.pack('>d', v)
    elif t == 'byteArray':
        out += struct.pack('>i', len(v))
        for x in v: out += struct.pack('>b', x - 256 if x > 127 else x)
    elif t == 'string': wstr(out, v)
    elif t == 'list':
        et, items = v
        out += struct.pack('>B', et); out += struct.pack('>i', len(items))
        for it in items: wpayload(out, (INV[et], it) if et != 10 else ('compound', it[1]) if isinstance(it, tuple) else it)
    elif t == 'compound':
        for k, node2 in v.items():
            out += struct.pack('>B', TAGID[node2[0]]); wstr(out, k); wpayload(out, node2)
        out += b'\x00'
    elif t == 'intArray':
        out += struct.pack('>i', len(v))
        for x in v: out += struct.pack('>i', x)
    elif t == 'longArray':
        out += struct.pack('>i', len(v))
        for x in v: out += struct.pack('>q', x)
    else:
        raise SystemExit('unknown ' + t)

INV = {v: k for k, v in TAGID.items()}

# list payloads store raw element payloads; for compound element lists the
# items are the dict (second tuple field). Re-wrap so wpayload sees nodes.
def fix_list_items(node):
    """Recursively normalise list elements parsed by rpayload into node form."""
    t, v = node
    if t == 'compound':
        return ('compound', {k: fix_list_items(x) for k, x in v.items()})
    if t == 'list':
        et, items = v
        et_name = INV[et]
        return ('list', (et, [fix_list_items((et_name, it))[1] if et == 10 else it for it in items]))
    return node

# Simpler: re-implement list writing by carrying element nodes directly.
# To avoid the ambiguity above we re-parse with element nodes preserved.

def rpayload2(b, i, t):
    if t == 9:
        et = b[i]; i += 1; n = struct.unpack('>i', b[i:i+4])[0]; i += 4
        items = []
        for _ in range(n):
            node, i = rpayload2(b, i, et); items.append(node)
        return ('list', ('end' if et == 0 else INV[et], items)), i
    if t == 10:
        d = {}
        while True:
            tt = b[i]; i += 1
            if tt == 0: break
            nm, i = rstr(b, i); node, i = rpayload2(b, i, tt); d[nm] = node
        return ('compound', d), i
    return rpayload(b, i, t)

def wpayload2(out, node):
    t, v = node
    if t == 'list':
        et_name, items = v
        et = TAGID[et_name] if et_name in TAGID else 0
        if not items: et = 0
        out += struct.pack('>B', et); out += struct.pack('>i', len(items))
        for it in items: wpayload2(out, (et_name, it[1]) if et_name == 'compound' else it)
        return
    if t == 'compound':
        for k, node2 in v.items():
            out += struct.pack('>B', TAGID[node2[0]]); wstr(out, k); wpayload2(out, node2)
        out += b'\x00'
        return
    wpayload(out, node)

# ── bundle split / entry parse ─────────────────────────────────────────────
def split_bundle(path):
    d = open(path, 'rb').read(); off = 0
    n = struct.unpack('>I', d[off:off+4])[0]; off += 4
    regs = []
    for _ in range(n):
        ln = struct.unpack('>I', d[off:off+4])[0]; off += 4
        regs.append(d[off:off+ln]); off += ln
    return regs

def parse_entries(body):
    rid, i = rstr_mc(body, 0)
    cnt, i = rvarint(body, i)
    entries = []
    for _ in range(cnt):
        key, i = rstr_mc(body, i)
        has = body[i]; i += 1
        node = None
        if has:
            assert body[i] == 0x0a
            node, i = rpayload2(body, i + 1, 10)  # nameless compound
        entries.append([key, has, node])
    return rid, entries

def build_body(rid, entries):
    out = bytearray()
    wstr_mc(out, rid)
    wvarint(out, len(entries))
    for key, has, node in entries:
        wstr_mc(out, key)
        out.append(has)
        if has:
            out.append(0x0a)            # nameless network NBT compound
            tmp = bytearray()
            wpayload2(tmp, node)        # compound payload (no leading id/name)
            out += tmp
    return bytes(out)

# ── 26.1 transforms ────────────────────────────────────────────────────────
def t_dimension_type(key, comp):
    comp[1]['has_ender_dragon_fight'] = ('byte', 1 if key == 'minecraft:the_end' else 0)

def t_baby_asset_id(key, comp):
    aid = comp[1].get('asset_id')
    if aid: comp[1]['baby_asset_id'] = ('string', aid[1])

def t_wolf_variant(key, comp):
    # ViaVersion: baby_assets = each asset value + "_baby" (NOT a verbatim copy).
    assets = comp[1].get('assets')
    if assets:
        baby = {}
        for k, node in assets[1].items():
            if node[0] == 'string':
                baby[k] = ('string', node[1] + '_baby')
            else:
                baby[k] = node
        comp[1]['baby_assets'] = ('compound', baby)

def t_wolf_sound_variant(key, comp):
    # ViaVersion Protocol1_21_11To26_1: move ALL existing fields into a `sounds`
    # compound, add the constant `step_sound = entity.wolf.step` (the only
    # wolf step sound that actually exists in the client's sound_event
    # registry — per-variant ".step" ids like entity.wolf_angry.step do NOT
    # exist), then set adult_sounds + baby_sounds to copies of it.
    d = comp[1]
    sounds = dict(d)
    sounds['step_sound'] = ('string', 'entity.wolf.step')
    d.clear()
    d['adult_sounds'] = ('compound', dict(sounds))
    d['baby_sounds'] = ('compound', dict(sounds))

def t_timeline(key, comp):
    comp[1]['clock'] = ('string', 'minecraft:overworld')

# ── 26.2 entity-predicate migration ────────────────────────────────────────
# 26.2 reworked the entity predicate (minecraft.wiki Java_Edition_26.2): the
# field `type` (entity-type matcher) was renamed to `minecraft:entity_type`,
# and unrecognized sub-predicate keys are now rejected. Our enchantment data
# carries the old `predicate:{type:"..."}` form, which a 26.2 client rejects
# with "Unknown registry key entity_sub_predicate_type: minecraft:type". Walk
# every registry entry, find `entity_properties` conditions, and rename the
# `type` key inside their entity predicate (and nested entity predicates).
ENTITY_PRED_NEST = ('looking_at', 'vehicle', 'passenger', 'targeted_entity')

def rename_entity_predicate(pred):
    d = pred[1]
    if 'type' in d:
        d['minecraft:entity_type'] = d.pop('type')
    for k in ENTITY_PRED_NEST:
        sub = d.get(k)
        if sub and sub[0] == 'compound':
            rename_entity_predicate(sub)

def walk_26_2(node):
    t, v = node
    if t == 'compound':
        cond = v.get('condition')
        if cond and cond[0] == 'string' and cond[1] == 'minecraft:entity_properties':
            pred = v.get('predicate')
            if pred and pred[0] == 'compound':
                rename_entity_predicate(pred)
        for child in v.values():
            walk_26_2(child)
    elif t == 'list':
        et_name, items = v
        if et_name in ('compound', 'list'):
            for it in items:
                walk_26_2(it)

TRANSFORMS = {
    'minecraft:dimension_type': t_dimension_type,
    'minecraft:cat_variant': t_baby_asset_id,
    'minecraft:chicken_variant': t_baby_asset_id,
    'minecraft:cow_variant': t_baby_asset_id,
    'minecraft:pig_variant': t_baby_asset_id,
    'minecraft:wolf_variant': t_wolf_variant,
    'minecraft:wolf_sound_variant': t_wolf_sound_variant,
    'minecraft:timeline': t_timeline,
}

def main():
    src, out_path = sys.argv[1], sys.argv[2]
    target = sys.argv[3] if len(sys.argv) > 3 else '26.1'
    regs = [parse_entries(b) for b in split_bundle(src)]

    # 26.1 added the minecraft:world_clock registry; timeline entries reference
    # minecraft:overworld/the_end, so they must be bound. Entries are empty
    # compounds. ViaVersion sends this as the LAST registry (in its
    # FINISH_CONFIGURATION handler, after every mapped registry incl. timeline);
    # append at the end to match that order exactly.
    world_clock = ('minecraft:world_clock', [
        ['minecraft:overworld', 1, ('compound', {})],
        ['minecraft:the_end', 1, ('compound', {})],
    ])
    regs.append(world_clock)
    print('added minecraft:world_clock (2 entries, appended last)')

    # 26.1's four new entity sound-variant registries (cat/chicken/cow/pig);
    # the client rejects them unless non-empty.
    for rid, entries in sound_variant_registries():
        regs.append((rid, entries))
        print('added %s (%d entries)' % (rid, len(entries)))

    # 26.2 added the "Bounce" music disc (minecraft.wiki Java_Edition_26.2);
    # the client builds its default jukebox_playable component referencing
    # jukebox_song minecraft:bounce, which the 1.21.11 base lacks ("Missing
    # element jukebox_song / minecraft:bounce"). Append it (comparator_output 8
    # per the wiki; same entry shape as the existing songs).
    if target == '26.2':
        bounce = ['minecraft:bounce', 1, ('compound', {
            'sound_event': ('string', 'minecraft:music_disc.bounce'),
            # Must be strictly positive (client validates "Value must be
            # positive"); exact length is cosmetic in limbo.
            'length_in_seconds': ('float', 120.0),
            'description': ('compound', {
                'translate': ('string', 'jukebox_song.minecraft.bounce'),
            }),
            'comparator_output': ('int', 8),
        })]
        for rid, entries in regs:
            if rid == 'minecraft:jukebox_song':
                entries.append(bounce)
                print('added jukebox_song minecraft:bounce')
                break

    out_bodies = []
    for rid, entries in regs:
        fn = TRANSFORMS.get(rid)
        if fn:
            for e in entries:
                if e[1] and e[2]:
                    fn(e[0], e[2])
            print('patched', rid, '(%d entries)' % len(entries))
        # 26.2: migrate entity predicates (type -> minecraft:entity_type).
        if target == '26.2':
            for e in entries:
                if e[1] and e[2]:
                    walk_26_2(e[2])
        out_bodies.append(build_body(rid, entries))
    if target == '26.2':
        print('applied 26.2 entity-predicate migration')

    bundle = bytearray(); bundle += struct.pack('>I', len(out_bodies))
    for b in out_bodies:
        bundle += struct.pack('>I', len(b)); bundle += b
    open(out_path, 'wb').write(bundle)
    print('wrote', out_path, len(bundle), 'bytes,', len(out_bodies), 'registries')

if __name__ == '__main__':
    main()
