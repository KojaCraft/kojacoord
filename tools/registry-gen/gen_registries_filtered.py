import json, struct, sys

TAGID = {'end':0,'byte':1,'short':2,'int':3,'long':4,'float':5,'double':6,
         'byteArray':7,'string':8,'list':9,'compound':10,'intArray':11,'longArray':12}
def w_varint(out, v):
    v &= 0xFFFFFFFF
    while True:
        b = v & 0x7F; v >>= 7
        if v: out.append(b|0x80)
        else: out.append(b); break
def w_mcstr(out, s):
    b=s.encode(); w_varint(out,len(b)); out+=b
def long_i64(v): return (v[0]<<32)|(v[1]&0xFFFFFFFF) if isinstance(v,list) else v
def w_payload(out,t,val):
    if t=='byte': out+=struct.pack('>b',val)
    elif t=='short': out+=struct.pack('>h',val)
    elif t=='int': out+=struct.pack('>i',val)
    elif t=='long': out+=struct.pack('>q',long_i64(val))
    elif t=='float': out+=struct.pack('>f',val)
    elif t=='double': out+=struct.pack('>d',val)
    elif t=='string':
        b=val.encode(); out+=struct.pack('>H',len(b)); out+=b
    elif t=='byteArray':
        out+=struct.pack('>i',len(val))
        for x in val: out+=struct.pack('>b',x)
    elif t=='intArray':
        out+=struct.pack('>i',len(val))
        for x in val: out+=struct.pack('>i',x)
    elif t=='longArray':
        out+=struct.pack('>i',len(val))
        for x in val: out+=struct.pack('>q',long_i64(x))
    elif t=='list':
        et=val['type']; items=val['value']
        out+=struct.pack('>B',TAGID[et]); out+=struct.pack('>i',len(items))
        for it in items: w_payload(out,et,it)
    elif t=='compound':
        for k,tv in val.items():
            out+=struct.pack('>B',TAGID[tv['type']])
            kb=k.encode(); out+=struct.pack('>H',len(kb)); out+=kb
            w_payload(out,tv['type'],tv['value'])
        out+=b'\x00'
def w_nameless(out,tag):
    out+=struct.pack('>B',TAGID[tag['type']]); w_payload(out,tag['type'],tag['value'])

codec=json.load(open(sys.argv[1]))['dimensionCodec']
include=set(sys.argv[3].split(',')) if len(sys.argv)>3 and sys.argv[3] else None
packets=[]
for reg_id,reg in codec.items():
    if include is not None and reg_id not in include: continue
    body=bytearray(); w_mcstr(body,reg_id)
    entries=reg['entries']; w_varint(body,len(entries))
    for e in entries:
        w_mcstr(body,e['key']); v=e.get('value')
        if v is None: body.append(0)
        else: body.append(1); w_nameless(body,v)
    packets.append(bytes(body))
bundle=bytearray(); bundle+=struct.pack('>I',len(packets))
for p in packets: bundle+=struct.pack('>I',len(p)); bundle+=p
open(sys.argv[2],'wb').write(bundle)
print('wrote',sys.argv[2],len(bundle),'bytes,',len(packets),'registries')
