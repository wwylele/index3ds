import sys
import requests

host = "http://127.0.0.1:8080"

filename = sys.argv[1]
print(filename)
f = open(filename, "rb")

header = f.read(0x200)
offset = 0

if header[0x100:0x104] == b'NCSD':
    offset = 0x4000
    f.seek(offset, 0)
    header = f.read(0x200)

while(True):
    r = requests.post(url = host + "/post_ncch", data=header)
print(r)
print(r.text)
j = r.json()
while j['status'] == 'AppendNeeded':
    f.seek(offset + j['offset'], 0)
    data = f.read(j['len'])
    r = requests.post(url = host + "/append_ncch/%d" % j['session_id'], data=data)
    print(r)
    print(r.text)
    j = r.json()


f.close()
