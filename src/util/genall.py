import os

sizes = [16_200, 44_692, 64_800, 140_974, 259_200, 1_036_800, 3_808_651, 4_147_200]

num_clusters = [2, 4, 8, 16, 32]
cluster_dist = 0.1
cluster_diststr = str(cluster_dist)

basecmd = "python rndgen.py "
basemv = "mv "

repeat = 3

def gen_uniform():
    for sz in sizes:
        szstr = str(sz)
        for i in range(repeat):
            oldname = szstr+".csv "
            newname = szstr+"_"+str(i)+".csv"
            os.system(basecmd+szstr)
            os.system(basemv+oldname+newname)

def gen_clustered():
    for clusters in num_clusters:
        clusterstr = str(clusters)
        for sz in sizes:
            sz_per_cluster = sz // clusters
            sz_per_clusterstr = str(sz_per_cluster)
            oldname = sz_per_clusterstr+"_"+clusterstr+"_"+cluster_diststr+".csv"
            for i in range(repeat):
                newname = oldname[:-4]+"_"+str(i)+".csv"
                #print("call: " + basecmd+sz_per_clusterstr+" "+clusterstr+" "+cluster_diststr)
                #print("move: "+basemv+oldname+" "+newname)
                os.system(basecmd+sz_per_clusterstr+" "+clusterstr+" "+cluster_diststr)
                os.system(basemv+oldname+" "+newname)





gen_uniform()
gen_clustered()