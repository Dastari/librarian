import hashlib, os, sys, random
def benc(x):
    if isinstance(x,int): return b"i%de"%x
    if isinstance(x,str): x=x.encode()
    if isinstance(x,bytes): return b"%d:%s"%(len(x),x)
    if isinstance(x,list): return b"l"+b"".join(benc(i) for i in x)+b"e"
    if isinstance(x,dict):
        return b"d"+b"".join(benc(k)+benc(x[k]) for k in sorted(x, key=lambda k: k.encode() if isinstance(k,str) else k))+b"e"
def make(name, files, announce, private, out, seed):
    rnd=random.Random(seed)
    plen=16384
    blob=b""; flist=[]
    for path,size in files:
        data=bytes(rnd.getrandbits(8) for _ in range(size))
        blob+=data; flist.append({"length":size,"path":path.split("/")})
    pieces=b"".join(hashlib.sha1(blob[i:i+plen]).digest() for i in range(0,len(blob),plen))
    info={"name":name,"piece length":plen,"pieces":pieces,"files":flist}
    if private: info["private"]=1
    t={"info":info,"created by":"librarian test fixture","creation date":1762646400}
    if announce: t["announce"]=announce
    open(out,"wb").write(benc(t))
    print(out, hashlib.sha1(benc(info)).hexdigest())
d=sys.argv[1]
make("Babylon.5.S01.1080p.BluRay.x264-GRP",[
 ("Babylon.5.S01E01.Midnight.on.the.Firing.Line.1080p.BluRay.x264-GRP.mkv",40000),
 ("Babylon.5.S01E02.Soul.Hunter.1080p.BluRay.x264-GRP.mkv",36000),
 ("Babylon.5.S01E03.Born.to.the.Purple.1080p.BluRay.x264-GRP.mkv",38000),
 ("Babylon.5.S01E03.Born.to.the.Purple.1080p.BluRay.x264-GRP.srt",900),
 ("Sample/babylon.5.s01e01.sample.mkv",5000),
 ("Babylon.5.S01.1080p.BluRay.x264-GRP.nfo",700),
],"https://tracker.example/a/0123456789abcdefPASSKEY/announce",True,os.path.join(d,"babylon5_s01_private.torrent"),1)
make("Queen - A Night at the Opera (1975) [FLAC]",[
 ("CD1/01 - Death on Two Legs (Dedicated to....).flac",30000),
 ("CD1/02 - Lazing on a Sunday Afternoon.flac",20000),
 ("CD1/11 - Bohemian Rhapsody.flac",45000),
 ("cover.jpg",3000),
],"udp://tracker.opentrackr.org:1337/announce",False,os.path.join(d,"queen_opera_public.torrent"),2)
