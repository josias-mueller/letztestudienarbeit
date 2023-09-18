from random import uniform
import sys

argvlen = len(sys.argv)

if (argvlen != 2 and argvlen != 4):
    exit()

if (argvlen == 2):# fully random
    number = int(sys.argv[1])
    fn = str(number)


clusters = 1
dist = 1
if (argvlen == 4):# clustered
    number = int(sys.argv[1])
    clusters = int(sys.argv[2])
    dist = float(sys.argv[3])
    fn = str(number)+'_'+str(clusters)+'_'+str(dist)
assert(clusters > 0)
assert(dist <= 1 and dist > 0)



with open(fn+".csv", "w") as csv:
    csv.write("lat,lon,id\n")
    id = 0
    for c in range(clusters):
        if (clusters == 1):
            baselat = 0
            baselon = 0
        else:
            baselat = uniform(-90, 90)
            baselon = uniform(-180, 180)
        for i in range(number):
            lat = baselat+uniform(-90+(-baselat), 90+(-baselat))*dist
            lon = baselon+uniform(-180+(-baselon), 180+(-baselon))*dist
            csv.write((str(lat)+','+str(lon)+','+str(id)+'\n'))
            id += 1