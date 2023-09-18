// #![feature(offset_of)]
use std::{
    fs::{self, create_dir_all, File, OpenOptions},
    io::{self, stdout, Read, Write},
    ops::Shl,
    path::Path,
    thread::sleep,
    time::{self, Duration},
};

use csv::StringRecord;
use hprtree::{BBox, HPRTree, HPRTreeBuilder, Point};
use rstar::{ParentNode, RTree, RTreeObject, AABB};

const ENV_SIZES: [usize; 5] = [16, 64, 256, 1024, 4096];
const ENV_COUNT: usize = 16;

const BUILD_COUNT_LIMIT: usize = 5_000_000;
const BUILD_TIME_LIMIT: Duration = Duration::from_secs(30);

const QUERYALL_LIMIT: usize = 5_000_000;
const QUERYALL_TIME_LIMIT: Duration = Duration::from_secs(30);

const QUERYPRE_LIMIT: usize = 5_000_000;
const QUERYPRE_TIME_LIMIT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
struct Element {
    pub lat: f32,
    pub lon: f32,
    pub id: u32,
}

impl RTreeObject for Element {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lon, self.lat])
    }
}

#[derive(Clone, Debug)]
struct BiggerElement {
    pub lat: f32,
    pub lon: f32,
    pub id: u32,
    pub negid: i32,
    pub compid: u64,
}

impl RTreeObject for BiggerElement {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lon, self.lat])
    }
}

#[derive(Clone, Debug)]
struct BigElement {
    pub lat: f32,
    pub lon: f32,
    pub data: [u64; 31],
}

impl RTreeObject for BigElement {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lon, self.lat])
    }
}

#[derive(Clone, Debug)]
struct VeryBigElement {
    pub lat: f32,
    pub lon: f32,
    pub data: [u64; 63],
}

impl RTreeObject for VeryBigElement {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lon, self.lat])
    }
}

#[derive(Clone, Debug)]
struct VeryVeryBigElement {
    pub lat: f32,
    pub lon: f32,
    pub data: [u64; 127],
}

impl RTreeObject for VeryVeryBigElement {
    type Envelope = AABB<[f32; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.lon, self.lat])
    }
}

fn read<T>(
    delimiter: u8,
    path: &str,
    count: usize,
    deser: fn(StringRecord) -> Option<(T, Point)>,
) -> Vec<(T, Point)> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_path(path)
        .unwrap();

    let mut arr = Vec::with_capacity(count);

    for result in reader.records() {
        match result {
            Ok(record) => match deser(record) {
                Some(element) => arr.push(element),
                None => {
                    //eprintln!("deser failed");
                }
            },
            Err(e) => eprintln!("error: {}", e),
        }
    }
    arr
}

fn bench_build_hprtree<T: Clone>(data: Vec<(T, Point)>, name: &str) -> HPRTree<T> {
    let stime = time::Instant::now();

    let mut timings = Vec::with_capacity(BUILD_COUNT_LIMIT);
    let mut d_timings = Vec::with_capacity(BUILD_COUNT_LIMIT);
    let mut size = 0;
    let mut total = Duration::ZERO;
    for c in 0..BUILD_COUNT_LIMIT {
        let data = data.clone();
        let start = time::Instant::now();
        let mut treebuilder = HPRTreeBuilder::new(data.len());
        for e in data {
            treebuilder.insert(e.0, e.1);
        }
        let treebuilder = treebuilder.build();
        let end = time::Instant::now();
        let diff = end - start;
        total += diff;
        assert!(treebuilder.len() != 0);
        size = treebuilder.current_size_in_bytes();
        timings.push(diff);
        {
            let d_start = time::Instant::now();
            drop(treebuilder);
            let d_end = time::Instant::now();
            let d_diff = d_end - d_start;
            total += d_diff;
            d_timings.push(d_diff);
        }
        if total > BUILD_TIME_LIMIT {
            eprintln!("exceeded time limit with iteration {c}!");
            break;
        }
    }
    {
        let mut file = File::create(format!("result/build/hprtree/{name}")).unwrap();
        for t in timings {
            file.write_all((t.as_nanos().to_string() + "\n").as_bytes())
                .unwrap();
        }
    }
    {
        let mut d_file = File::create(format!("result/build/d_hprtree/{name}")).unwrap();
        for t in d_timings {
            d_file
                .write_all((t.as_nanos().to_string() + "\n").as_bytes())
                .unwrap();
        }
    }
    {
        let mut szfile = OpenOptions::new()
            .append(true)
            .create(true)
            .open("result/szfiles/hprtree")
            .unwrap();
        szfile
            .write_all(format!("{name}: {size}\n").as_bytes())
            .unwrap();
    }
    let etime = time::Instant::now();
    println!("{name} done in {:?} ({total:?})", etime - stime);

    let data = data.clone();
    let mut treebuilder = HPRTreeBuilder::new(data.len());
    for e in data {
        treebuilder.insert(e.0, e.1);
    }
    treebuilder.build()
}

fn bench_build_rstar<T: Clone>(data: Vec<T>, name: &str) -> RTree<T>
where
    T: RTreeObject,
{
    let stime = time::Instant::now();

    let mut size = 0;
    let mut timings = Vec::with_capacity(BUILD_COUNT_LIMIT);
    let mut d_timings = Vec::with_capacity(BUILD_COUNT_LIMIT);
    let mut total = Duration::ZERO;
    for c in 0..BUILD_COUNT_LIMIT {
        let data = data.clone();
        let start = time::Instant::now();
        let tree = RTree::bulk_load(data);
        let end = time::Instant::now();
        let diff = end - start;
        total += diff;
        size = tree.size_in_bytes();
        assert!(tree.size() != 0);
        timings.push(diff);
        {
            let d_start = time::Instant::now();
            drop(tree);
            let d_end = time::Instant::now();
            let d_diff = d_end - d_start;
            total += d_diff;
            d_timings.push(d_diff);
        }
        if total > BUILD_TIME_LIMIT {
            eprintln!("exceeded time limit with iteration {c}!");
            break;
        }
    }
    {
        let mut file = File::create(format!("result/build/rstar/{name}")).unwrap();
        for t in timings {
            file.write_all((t.as_nanos().to_string() + "\n").as_bytes())
                .unwrap();
        }
    }
    {
        let mut d_file = File::create(format!("result/build/d_rstar/{name}")).unwrap();
        for t in d_timings {
            d_file
                .write_all((t.as_nanos().to_string() + "\n").as_bytes())
                .unwrap();
        }
    }
    {
        let mut szfile = OpenOptions::new()
            .append(true)
            .create(true)
            .open("result/szfiles/rstar")
            .unwrap();
        szfile
            .write_all(format!("{name}: {size}\n").as_bytes())
            .unwrap();
    }
    let etime = time::Instant::now();
    println!("{name} done in {:?} ({total:?})", etime - stime);

    RTree::bulk_load(data.clone())
}

fn bench_queryall_hprtree<T>(filename: String, tree: &HPRTree<T>)
where
    T: Clone,
{
    let stime = time::Instant::now();

    let mut timings = Vec::with_capacity(QUERYALL_LIMIT);
    let mut total = Duration::ZERO;
    for c in 0..QUERYALL_LIMIT {
        let start = time::Instant::now();
        //
        let queryres = tree.query(&tree.extent());
        //
        let end = time::Instant::now();
        assert!(queryres.len() == tree.len());
        let diff = end - start;
        total += diff;
        timings.push(diff);
        if total > QUERYALL_TIME_LIMIT {
            eprintln!("exceeded time limit with iteration {c}!");
            break;
        }
    }
    let mut file = File::create(format!("result/queryall/hprtree/{filename}")).unwrap();
    for t in timings {
        file.write_all((t.as_nanos().to_string() + "\n").as_bytes())
            .unwrap();
    }

    let etime = time::Instant::now();
    println!("queryall done in {:?} ({total:?})", etime - stime);
}

fn bench_queryall_rtree<T>(filename: String, tree: &RTree<T>)
where
    T: Clone,
    T: RTreeObject,
{
    let stime = time::Instant::now();

    let mut timings = Vec::with_capacity(QUERYALL_LIMIT);
    let mut total = Duration::ZERO;
    for c in 0..QUERYALL_LIMIT {
        let start = time::Instant::now();
        // this thing unfortunately gives back references, idk how to properly make that into actual data, but this is how it will be for now...
        let queryiter = tree.locate_in_envelope(&tree.root().envelope());
        let mut queryres = Vec::with_capacity(tree.size());
        for elem in queryiter {
            queryres.push(elem.clone());
        }
        //
        let end = time::Instant::now();
        assert!(queryres.len() == tree.size());
        let diff = end - start;
        total += diff;
        timings.push(diff);
        if total > QUERYALL_TIME_LIMIT {
            eprintln!("exceeded time limit with iteration {c}!");
            break;
        }
    }
    let mut file = File::create(format!("result/queryall/rstar/{filename}")).unwrap();
    for t in timings {
        file.write_all((t.as_nanos().to_string() + "\n").as_bytes())
            .unwrap();
    }

    let etime = time::Instant::now();
    println!("queryall done in {:?} ({total:?})", etime - stime);
}

// filename is name of env file - .size
fn bench_querypre_hprtree<T>(filename: String, tree: &HPRTree<T>)
where
    T: Clone,
{
    let stime = time::Instant::now();

    // load the respective bboxes
    let bboxes = {
        let mut bboxes = [
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
        ];
        for (i, size) in ENV_SIZES.iter().enumerate() {
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .delimiter(b',')
                .from_path(format!("{}.{}", filename, size))
                .unwrap();

            for result in reader.records() {
                let result = result.unwrap();
                let env = BBox {
                    minx: result.get(0).unwrap().parse::<f32>().unwrap(),
                    maxx: result.get(1).unwrap().parse::<f32>().unwrap(),
                    miny: result.get(2).unwrap().parse::<f32>().unwrap(),
                    maxy: result.get(3).unwrap().parse::<f32>().unwrap(),
                };
                bboxes[i].push(env);
            }
        }
        bboxes
    };
    // n bboxes timings
    let mut timings = [
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
    ];
    assert!(timings.len() == ENV_SIZES.len());
    let mut total = Duration::ZERO;
    for c in 0..QUERYPRE_LIMIT {
        // for all bboxes
        for i in 0..ENV_COUNT {
            for n in 0..ENV_SIZES.len() {
                //start time
                let start = time::Instant::now();
                //do query
                let mut res = Vec::with_capacity(ENV_SIZES[n]);
                tree.query_with_list(&bboxes[n][i], &mut res);
                //save to respective timing
                let end = time::Instant::now();
                assert!(res.len() == ENV_SIZES[n]);
                let diff = end - start;
                total += diff;
                timings[n].push(diff);
            }
        }
        if total > QUERYPRE_TIME_LIMIT {
            eprintln!("exceeded time limit with iteration {c}!");
            break;
        }
    }
    // save timings
    let path = Path::new(&filename);
    let pname = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let fname = path.file_name().unwrap().to_str().unwrap();
    let fname = &fname[..(fname.len() - 4)];
    let tn = &std::any::type_name::<T>()[6..];
    for (i, size) in ENV_SIZES.iter().enumerate() {
        let mut file = File::create(format!(
            "result/querypre/hprtree/{pname}_{fname}_{tn}.{size}"
        ))
        .unwrap();
        for t in &timings[i] {
            file.write_all((t.as_nanos().to_string() + "\n").as_bytes())
                .unwrap();
        }
    }

    let etime = time::Instant::now();
    println!("querypre done in {:?} ({total:?})", etime - stime);
}

// filename is name of env file - .size
fn bench_querypre_rstar<T>(filename: String, tree: &RTree<T>)
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    let stime = time::Instant::now();

    // load the respective bboxes
    let bboxes = {
        let mut bboxes = [
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
            Vec::with_capacity(ENV_COUNT),
        ];
        for (i, size) in ENV_SIZES.iter().enumerate() {
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .delimiter(b',')
                .from_path(format!("{}.{}", filename, size))
                .unwrap();

            for result in reader.records() {
                let result = result.unwrap();
                let env = BBox {
                    minx: result.get(0).unwrap().parse::<f32>().unwrap(),
                    maxx: result.get(1).unwrap().parse::<f32>().unwrap(),
                    miny: result.get(2).unwrap().parse::<f32>().unwrap(),
                    maxy: result.get(3).unwrap().parse::<f32>().unwrap(),
                };
                bboxes[i].push(env);
            }
        }
        bboxes
    };
    // n bboxes timings
    let mut timings = [
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
        Vec::with_capacity(QUERYPRE_LIMIT),
    ];
    assert!(timings.len() == ENV_SIZES.len());
    let mut total = Duration::ZERO;
    for c in 0..QUERYPRE_LIMIT {
        //  for all bboxes
        for i in 0..ENV_COUNT {
            for n in 0..ENV_SIZES.len() {
                //start time
                let start = time::Instant::now();
                //do query
                let e = &bboxes[n][i];
                let queryiter = tree
                    .locate_in_envelope(&AABB::from_corners([e.minx, e.miny], [e.maxx, e.maxy]));
                let mut res = Vec::with_capacity(tree.size());
                for elem in queryiter {
                    res.push(elem.clone());
                }
                //save to respective timing
                let end = time::Instant::now();
                assert!(res.len() == ENV_SIZES[n]);
                let diff = end - start;
                total += diff;
                timings[n].push(diff);
            }
        }
        if total > QUERYPRE_TIME_LIMIT {
            eprintln!("exceeded time limit with iteration {c}!");
            break;
        }
    }
    // save timings
    let path = Path::new(&filename);
    let pname = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let fname = path.file_name().unwrap().to_str().unwrap();
    let tn = &std::any::type_name::<T>()[6..];
    for (i, size) in ENV_SIZES.iter().enumerate() {
        let mut file =
            File::create(format!("result/querypre/rstar/{pname}_{fname}_{tn}.{size}")).unwrap();
        for t in &timings[i] {
            file.write_all((t.as_nanos().to_string() + "\n").as_bytes())
                .unwrap();
        }
    }

    let etime = time::Instant::now();
    println!("querypre done in {:?} ({total:?})", etime - stime);
}

trait TestSize {
    fn size_in_bytes(&self) -> usize;
}

fn size_in_bytes_helper<T>(node: &ParentNode<T>) -> usize
where
    T: RTreeObject,
{
    let mut sum = 0;
    for child in node.children() {
        sum += std::mem::size_of_val(child);
        match child {
            rstar::RTreeNode::Leaf(_) => (),
            rstar::RTreeNode::Parent(subparent) => sum += size_in_bytes_helper(subparent),
        }
    }
    sum
}

impl<T> TestSize for RTree<T>
where
    T: RTreeObject,
{
    fn size_in_bytes(&self) -> usize {
        let internal_sz = size_in_bytes_helper(self.root());
        std::mem::size_of_val(self) + internal_sz
    }
}

fn bench_hprtree_opendata<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
{
    println!("starting build...");
    let count = 140974;
    let tn = &std::any::type_name::<T>()[6..];

    let data = read(
        b';',
        "../../data/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv",
        count,
        deser,
    );

    let tree = bench_build_hprtree(data, &format!("bench_build_hprtree_opendata_{tn}"));
    bench_queryall_hprtree(format!("bench_queryall_hprtree_opendata_{tn}"), &tree);
    bench_querypre_hprtree(
        "../../data/envelopes/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv"
            .to_string(),
        &tree,
    );
}

fn bench_rstar_opendata<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    println!("starting build...");
    let tn = &std::any::type_name::<T>()[6..];
    let count = 140974;

    let data = read(
        b';',
        "../../data/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv",
        count,
        deser,
    )
    .into_iter()
    .map(|e| e.0)
    .collect();

    let tree = bench_build_rstar(data, &format!("bench_build_rstar_opendata_{tn}"));
    bench_queryall_rtree(format!("bench_queryall_rstar_opendata_{tn}"), &tree);
    bench_querypre_rstar(
        "../../data/envelopes/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv"
            .to_string(),
        &tree,
    );
}

fn bench_hprtree_random_uniform<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
{
    let tn = &std::any::type_name::<T>()[6..];
    let paths = fs::read_dir("../../data/new/uniform/").unwrap();
    for path in paths {
        let path = path.unwrap();
        if !path.metadata().unwrap().is_file() {
            continue;
        }
        let filename = path.file_name();
        let filename_str = filename.to_str().unwrap();
        let filename_wo_ext = &filename_str[..(filename_str.len() - 4)];

        let count = filename_wo_ext
            .split_once('_')
            .unwrap()
            .0
            .parse::<usize>()
            .unwrap();
        let p = path.path();
        let pathstr = p.to_str().unwrap();
        let data = read(b',', pathstr, count, deser);

        let tree = bench_build_hprtree(
            data,
            &format!("bench_build_hprtree_uniform_{filename_wo_ext}_{tn}",),
        );
        bench_queryall_hprtree(
            format!("bench_queryall_hprtree_uniform_{filename_wo_ext}_{tn}"),
            &tree,
        );
        bench_querypre_hprtree(pathstr.replace("/data", "/data/envelopes"), &tree);
    }
}

fn bench_rstar_random_uniform<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    let tn = &std::any::type_name::<T>()[6..];
    let paths = fs::read_dir("../../data/new/uniform/").unwrap();
    for path in paths {
        let path = path.unwrap();
        if !path.metadata().unwrap().is_file() {
            continue;
        }
        let filename = path.file_name();
        let filename_str = filename.to_str().unwrap();
        let filename_wo_ext = &filename_str[..(filename_str.len() - 4)];

        let count = filename_wo_ext
            .split_once('_')
            .unwrap()
            .0
            .parse::<usize>()
            .unwrap();

        let p = path.path();
        let pathstr = p.to_str().unwrap();
        let data = read(b',', pathstr, count, deser)
            .into_iter()
            .map(|e| e.0)
            .collect();

        let tree = bench_build_rstar(
            data,
            &format!("bench_build_rstar_uniform_{filename_wo_ext}_{tn}"),
        );
        bench_queryall_rtree(
            format!("bench_queryall_rstar_uniform_{filename_wo_ext}_{tn}"),
            &tree,
        );
        bench_querypre_rstar(pathstr.replace("/data", "/data/envelopes"), &tree);
    }
}

fn bench_hprtree_random_clustered<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
{
    let tn = &std::any::type_name::<T>()[6..];
    let paths = fs::read_dir("../../data/new/clustered/").unwrap();
    for path in paths {
        let path = path.unwrap();
        if !path.metadata().unwrap().is_file() {
            continue;
        }
        let filename = path.file_name();
        let filename_str = filename.to_str().unwrap();
        let filename_wo_ext = &filename_str[..(filename_str.len() - 4)];

        let mut split = filename_wo_ext.split('_');
        let numelem = split.next().unwrap().parse::<usize>().unwrap();
        let numclust = split.next().unwrap().parse::<usize>().unwrap();

        let count = numelem * numclust;

        let p = path.path();
        let pathstr = p.to_str().unwrap();
        let data = read(b',', pathstr, count, deser);

        let tree = bench_build_hprtree(
            data,
            &format!("bench_build_hprtree_clustered_{filename_wo_ext}_{tn}"),
        );
        bench_queryall_hprtree(
            format!("bench_queryall_hprtree_clustered_{filename_wo_ext}_{tn}"),
            &tree,
        );
        bench_querypre_hprtree(pathstr.replace("/data", "/data/envelopes"), &tree);
    }
}

fn bench_rstar_random_clustered<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    let tn = &std::any::type_name::<T>()[6..];
    let paths = fs::read_dir("../../data/new/clustered/").unwrap();
    for path in paths {
        let path = path.unwrap();
        if !path.metadata().unwrap().is_file() {
            continue;
        }
        let filename = path.file_name();
        let filename_str = filename.to_str().unwrap();
        let filename_wo_ext = &filename_str[..(filename_str.len() - 4)];

        let mut split = filename_wo_ext.split('_');
        let numelem = split.next().unwrap().parse::<usize>().unwrap();
        let numclust = split.next().unwrap().parse::<usize>().unwrap();

        let count = numelem * numclust;

        let p = path.path();
        let pathstr = p.to_str().unwrap();

        let data = read(b',', pathstr, count, deser)
            .into_iter()
            .map(|e| e.0)
            .collect();

        let tree = bench_build_rstar(
            data,
            &format!("bench_build_rstar_clustered_{filename_wo_ext}_{tn}"),
        );
        bench_queryall_rtree(
            format!("bench_queryall_rstar_clustered_{filename_wo_ext}_{tn}"),
            &tree,
        );
        bench_querypre_rstar(pathstr.replace("/data", "/data/envelopes"), &tree);
    }
}

fn bench_hprtree_simplemaps<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
{
    let tn = &std::any::type_name::<T>()[6..];
    let count = 44692;
    let data = read(
        b',',
        "../../data/base/simplemaps/worldcities.csv",
        count,
        deser,
    );

    let tree = bench_build_hprtree(data, &format!("bench_build_hprtree_simplemaps_{tn}"));
    bench_queryall_hprtree(format!("bench_queryall_hprtree_simplemaps_{tn}"), &tree);
    bench_querypre_hprtree(
        "../../data/envelopes/base/simplemaps/worldcities.csv".to_string(),
        &tree,
    );
}

fn bench_rstar_simplemaps<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    let tn = &std::any::type_name::<T>()[6..];
    let count = 44692;
    let data = read(
        b',',
        "../../data/base/simplemaps/worldcities.csv",
        count,
        deser,
    )
    .into_iter()
    .map(|e| e.0)
    .collect();

    let tree = bench_build_rstar(data, &format!("bench_build_rstar_simplemaps_{tn}"));
    bench_queryall_rtree(format!("bench_queryall_rstar_simplemaps_{tn}"), &tree);
    bench_querypre_rstar(
        "../../data/envelopes/base/simplemaps/worldcities.csv".to_string(),
        &tree,
    );
}

fn bench_hprtree_synthetic_180x90x_x<T>(mult: u32, gen: fn(f32, f32, u32) -> (T, Point))
where
    T: Clone,
{
    let tn = &std::any::type_name::<T>()[6..];
    let submult = (mult as f32).sqrt();
    let d = 2f32 / submult;
    let data = {
        let mut data = Vec::with_capacity(180 * 90 * mult as usize);
        let mut x = -180f32;
        for i in 0..(180 * submult as u32) {
            let mut y = -90f32;
            for j in 0..(90 * submult as u32) {
                data.push(gen(x, y, i * 10000u32 + j));
                y += d;
            }
            x += d;
        }
        data
    };
    assert!(data.len() == 180 * 90 * mult as usize);

    let tree = bench_build_hprtree(
        data,
        &format!("bench_build_hprtree_synthetic_180x90x{mult}_{tn}"),
    );
    bench_queryall_hprtree(
        format!("bench_queryall_hprtree_synthetic_180x90x{mult}_{tn}"),
        &tree,
    );
    bench_querypre_hprtree(format!("../../data/envelopes/ordered/{mult}"), &tree);
}

fn bench_rstar_synthetic_180x90x_x<T>(mult: u32, gen: fn(f32, f32, u32) -> (T, Point))
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    let tn = &std::any::type_name::<T>()[6..];
    let submult = (mult as f32).sqrt();
    let d = 2f32 / submult;
    let data: Vec<T> = {
        let mut data = Vec::with_capacity(180 * 90 * mult as usize);
        let mut x = -180f32;
        for i in 0..(180 * submult as u32) {
            let mut y = -90f32;
            for j in 0..(90 * submult as u32) {
                data.push(gen(x, y, i * 10000u32 + j));
                y += d;
            }
            x += d;
        }
        data
    }
    .into_iter()
    .map(|e| e.0)
    .collect();
    assert!(data.len() == 180 * 90 * mult as usize);

    let tree = bench_build_rstar(
        data,
        &format!("bench_build_rstar_synthetic_180x90x{mult}_{tn}"),
    );
    bench_queryall_rtree(
        format!("bench_queryall_rstar_synthetic_180x90x{mult}_{tn}"),
        &tree,
    );
    bench_querypre_rstar(format!("../../data/envelopes/ordered/{mult}"), &tree);
}

fn bench_hprtree_matthe<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
{
    let tn = &std::any::type_name::<T>()[6..];
    let count = 3808651;
    let data = read(
        b',',
        "../../data/base/matthewproctor/worldcities-geo.csv",
        count,
        deser,
    );

    let tree = bench_build_hprtree(data, &format!("bench_build_hprtree_matthe_{tn}"));
    bench_queryall_hprtree(format!("bench_queryall_hprtree_matthe_{tn}"), &tree);
    bench_querypre_hprtree(
        "../../data/envelopes/base/matthewproctor/worldcities-geo.csv".to_string(),
        &tree,
    );
}

fn bench_rstar_matthe<T>(deser: fn(StringRecord) -> Option<(T, Point)>)
where
    T: Clone,
    T: RTreeObject<Envelope = AABB<[f32; 2]>>,
{
    let tn = &std::any::type_name::<T>()[6..];
    let count = 3808651;
    let data = read(
        b',',
        "../../data/base/matthewproctor/worldcities-geo.csv",
        count,
        deser,
    )
    .into_iter()
    .map(|e| e.0)
    .collect();

    let tree = bench_build_rstar(data, &format!("bench_build_rstar_matthe_{tn}"));
    bench_queryall_rtree(format!("bench_queryall_rstar_matthe_{tn}"), &tree);
    bench_querypre_rstar(
        "../../data/envelopes/base/matthewproctor/worldcities-geo.csv".to_string(),
        &tree,
    );
}

fn main() {
    {
        let mut stdin = io::stdin();
        print!("Press enter to start...");
        stdout().flush().unwrap();
        stdin.read(&mut [0u8]).unwrap();
    }

    let program_start = time::Instant::now();

    create_dir_all(Path::new("result/querypre/rstar/")).unwrap();
    create_dir_all(Path::new("result/querypre/hprtree/")).unwrap();
    create_dir_all(Path::new("result/queryall/rstar/")).unwrap();
    create_dir_all(Path::new("result/queryall/hprtree/")).unwrap();
    create_dir_all(Path::new("result/build/rstar/")).unwrap();
    create_dir_all(Path::new("result/build/d_rstar/")).unwrap();
    create_dir_all(Path::new("result/build/hprtree/")).unwrap();
    create_dir_all(Path::new("result/build/d_hprtree/")).unwrap();
    create_dir_all(Path::new("result/szfiles/")).unwrap();

    // println!("u64: {}", std::mem::size_of_val(&64u64));
    // println!("u32: {}", std::mem::size_of_val(&64u32));
    // println!("[u32]: {}", std::mem::size_of_val(&[0u32]));
    // println!("[u32, u32, u32]: {}", std::mem::size_of_val(&[0u32, 0u32, 0u32]));

    // println!("\n\n");
    // let elements: Vec<[f32; 2]> = vec![[0.0f32, 0.0f32], [0.0f32, 0.0f32]];//[0.0f32, 0.0f32], [0.0f32, 0.0f32]
    // let tree = RTree::bulk_load(elements);

    // println!("env size: {}", std::mem::size_of_val(&tree.root().envelope()));
    // println!("tree size: {}", std::mem::size_of_val(&tree));
    // println!("pnode size: {}", std::mem::size_of_val(tree.root()));
    // println!("arr size: {}", std::mem::size_of_val(&tree.root().children()));
    // println!("tree size: {}, in bytes: {}", tree.size(), tree.size_in_bytes());
    // return;

    let opendata_to_element = |record: StringRecord| {
        assert!(record.len() == 20);
        let geoid = record.get(0).unwrap().parse::<u32>().unwrap();
        let coords = record
            .get(record.len() - 1)
            .unwrap()
            .split_once(", ")
            .unwrap();
        let lat = coords.0.parse::<f32>().unwrap();
        let lon = coords.1.parse::<f32>().unwrap();

        Some((
            Element {
                lat,
                lon,
                id: geoid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let opendata_to_biggerelement = |record: StringRecord| {
        assert!(record.len() == 20);
        let geoid = record.get(0).unwrap().parse::<u32>().unwrap();
        let coords = record
            .get(record.len() - 1)
            .unwrap()
            .split_once(", ")
            .unwrap();
        let lat = coords.0.parse::<f32>().unwrap();
        let lon = coords.1.parse::<f32>().unwrap();

        let negid = -(geoid as i32);
        let compid = geoid as u64 | ((negid as u64 & 0xFFFFFFFF00000000).shl(8));
        Some((
            BiggerElement {
                lat,
                lon,
                id: geoid,
                negid,
                compid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let opendata_to_bigelement = |record: StringRecord| {
        assert!(record.len() == 20);
        let geoid = record.get(0).unwrap().parse::<u32>().unwrap();
        let coords = record
            .get(record.len() - 1)
            .unwrap()
            .split_once(", ")
            .unwrap();
        let lat = coords.0.parse::<f32>().unwrap();
        let lon = coords.1.parse::<f32>().unwrap();

        let data = [geoid as u64; 31];
        Some((
            BigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let opendata_to_verybigelement = |record: StringRecord| {
        assert!(record.len() == 20);
        let geoid = record.get(0).unwrap().parse::<u32>().unwrap();
        let coords = record
            .get(record.len() - 1)
            .unwrap()
            .split_once(", ")
            .unwrap();
        let lat = coords.0.parse::<f32>().unwrap();
        let lon = coords.1.parse::<f32>().unwrap();

        let data = [geoid as u64; 63];
        Some((
            VeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let opendata_to_veryverybigelement = |record: StringRecord| {
        assert!(record.len() == 20);
        let geoid = record.get(0).unwrap().parse::<u32>().unwrap();
        let coords = record
            .get(record.len() - 1)
            .unwrap()
            .split_once(", ")
            .unwrap();
        let lat = coords.0.parse::<f32>().unwrap();
        let lon = coords.1.parse::<f32>().unwrap();

        let data = [geoid as u64; 127];
        Some((
            VeryVeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };

    let matthe_to_element = |record: StringRecord| {
        assert!(record.len() == 10);

        let geoid = match record.get(0).unwrap().parse::<u32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lat = match record.get(8).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lon = match record.get(9).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };

        Some((
            Element {
                lat,
                lon,
                id: geoid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let matthe_to_biggerelement = |record: StringRecord| {
        assert!(record.len() == 10);

        let geoid = match record.get(0).unwrap().parse::<u32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lat = match record.get(8).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lon = match record.get(9).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };

        let negid = -(geoid as i32);
        let compid = geoid as u64 | ((negid as u64 & 0xFFFFFFFF00000000).shl(8));
        Some((
            BiggerElement {
                lat,
                lon,
                id: geoid,
                negid,
                compid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let matthe_to_bigelement = |record: StringRecord| {
        assert!(record.len() == 10);

        let geoid = match record.get(0).unwrap().parse::<u32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lat = match record.get(8).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lon = match record.get(9).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };

        let data = [geoid as u64; 31];
        Some((
            BigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let matthe_to_verybigelement = |record: StringRecord| {
        assert!(record.len() == 10);

        let geoid = match record.get(0).unwrap().parse::<u32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lat = match record.get(8).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lon = match record.get(9).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };

        let data = [geoid as u64; 63];
        Some((
            VeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let matthe_to_veryverybigelement = |record: StringRecord| {
        assert!(record.len() == 10);

        let geoid = match record.get(0).unwrap().parse::<u32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lat = match record.get(8).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lon = match record.get(9).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };

        let data = [geoid as u64; 127];
        Some((
            VeryVeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };

    let simplemaps_to_element = |record: StringRecord| {
        assert!(record.len() == 11);
        let lat = record.get(2).unwrap().parse::<f32>().unwrap();
        let lon = record.get(3).unwrap().parse::<f32>().unwrap();
        let geoid = record.get(10).unwrap().parse::<u32>().unwrap();
        Some((
            Element {
                lat,
                lon,
                id: geoid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let simplemaps_to_biggerelement = |record: StringRecord| {
        assert!(record.len() == 11);
        let lat = record.get(2).unwrap().parse::<f32>().unwrap();
        let lon = record.get(3).unwrap().parse::<f32>().unwrap();
        let geoid = record.get(10).unwrap().parse::<u32>().unwrap();
        let negid = -(geoid as i32);
        let compid = geoid as u64 | ((negid as u64 & 0xFFFFFFFF00000000).shl(8));
        Some((
            BiggerElement {
                lat,
                lon,
                id: geoid,
                negid,
                compid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let simplemaps_to_bigelement = |record: StringRecord| {
        assert!(record.len() == 11);
        let lat = record.get(2).unwrap().parse::<f32>().unwrap();
        let lon = record.get(3).unwrap().parse::<f32>().unwrap();
        let geoid = record.get(10).unwrap().parse::<u32>().unwrap();
        let data = [geoid as u64; 31];
        Some((
            BigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let simplemaps_to_verybigelement = |record: StringRecord| {
        assert!(record.len() == 11);
        let lat = record.get(2).unwrap().parse::<f32>().unwrap();
        let lon = record.get(3).unwrap().parse::<f32>().unwrap();
        let geoid = record.get(10).unwrap().parse::<u32>().unwrap();
        let data = [geoid as u64; 63];
        Some((
            VeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let simplemaps_to_veryverybigelement = |record: StringRecord| {
        assert!(record.len() == 11);
        let lat = record.get(2).unwrap().parse::<f32>().unwrap();
        let lon = record.get(3).unwrap().parse::<f32>().unwrap();
        let geoid = record.get(10).unwrap().parse::<u32>().unwrap();
        let data = [geoid as u64; 127];
        Some((
            VeryVeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };

    let synthetic_to_element =
        |x: f32, y: f32, id: u32| (Element { lat: y, lon: x, id }, Point { x, y });
    let synthetic_to_biggerelement = |x: f32, y: f32, id: u32| {
        let negid = -(id as i32);
        let compid = id as u64 | ((negid as u64 & 0xFFFFFFFF00000000).shl(8));
        (
            BiggerElement {
                lat: y,
                lon: x,
                id,
                negid,
                compid,
            },
            Point { x, y },
        )
    };
    let synthetic_to_bigelement = |x: f32, y: f32, id: u32| {
        let data = [id as u64; 31];
        (
            BigElement {
                lat: y,
                lon: x,
                data,
            },
            Point { x, y },
        )
    };
    let synthetic_to_verybigelement = |x: f32, y: f32, id: u32| {
        let data = [id as u64; 63];
        (
            VeryBigElement {
                lat: y,
                lon: x,
                data,
            },
            Point { x, y },
        )
    };
    let synthetic_to_veryverybigelement = |x: f32, y: f32, id: u32| {
        let data = [id as u64; 127];
        (
            VeryVeryBigElement {
                lat: y,
                lon: x,
                data,
            },
            Point { x, y },
        )
    };

    let random_to_element = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        let id = record.get(2).unwrap().parse::<u32>().unwrap();
        Some((Element { lat, lon, id }, hprtree::Point { x: lon, y: lat }))
    };
    let random_to_biggerelement = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        let id = record.get(2).unwrap().parse::<u32>().unwrap();
        let negid = -(id as i32);
        let compid = id as u64 | ((negid as u64 & 0xFFFFFFFF00000000).shl(8));
        Some((
            BiggerElement {
                lat,
                lon,
                id,
                negid,
                compid,
            },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let random_to_bigelement = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        let id = record.get(2).unwrap().parse::<u32>().unwrap();
        let data = [id as u64; 31];
        Some((
            BigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let random_to_verybigelement = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        let id = record.get(2).unwrap().parse::<u32>().unwrap();
        let data = [id as u64; 63];
        Some((
            VeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };
    let random_to_veryverybigelement = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        let id = record.get(2).unwrap().parse::<u32>().unwrap();
        let data = [id as u64; 127];
        Some((
            VeryVeryBigElement { lat, lon, data },
            hprtree::Point { x: lon, y: lat },
        ))
    };

    println!("{}", std::mem::size_of::<Element>());
    println!("{}", std::mem::size_of::<BiggerElement>());
    println!("{}", std::mem::size_of::<BigElement>());
    println!("{}", std::mem::size_of::<VeryBigElement>());
    println!("{}", std::mem::size_of::<VeryVeryBigElement>());
    /////
    // println!("{}",std::mem::offset_of!(BiggerElement, lat));
    // println!("{}",std::mem::offset_of!(BiggerElement, lon));
    // println!("{}",std::mem::offset_of!(BiggerElement, id));
    // println!("{}",std::mem::offset_of!(BiggerElement, negid));
    // println!("{}",std::mem::offset_of!(BiggerElement, compid));

    {
        // hprtree build
        ///// hprtree element:
        bench_hprtree_opendata(opendata_to_element);
        bench_hprtree_matthe(matthe_to_element);
        bench_hprtree_simplemaps(simplemaps_to_element);
        bench_hprtree_synthetic_180x90x_x(1, synthetic_to_element);
        bench_hprtree_synthetic_180x90x_x(4, synthetic_to_element);
        bench_hprtree_synthetic_180x90x_x(16, synthetic_to_element);
        bench_hprtree_synthetic_180x90x_x(64, synthetic_to_element);
        bench_hprtree_synthetic_180x90x_x(256, synthetic_to_element);
        bench_hprtree_random_uniform(random_to_element);
        bench_hprtree_random_clustered(random_to_element);
        println!("hprtree element done\n");
        ///// hprtree biggerelement:
        bench_hprtree_opendata(opendata_to_biggerelement);
        bench_hprtree_matthe(matthe_to_biggerelement);
        bench_hprtree_simplemaps(simplemaps_to_biggerelement);
        bench_hprtree_synthetic_180x90x_x(1, synthetic_to_biggerelement);
        bench_hprtree_synthetic_180x90x_x(4, synthetic_to_biggerelement);
        bench_hprtree_synthetic_180x90x_x(16, synthetic_to_biggerelement);
        bench_hprtree_synthetic_180x90x_x(64, synthetic_to_biggerelement);
        bench_hprtree_synthetic_180x90x_x(256, synthetic_to_biggerelement);
        bench_hprtree_random_uniform(random_to_biggerelement);
        bench_hprtree_random_clustered(random_to_biggerelement);
        println!("hprtree biggerelement done\n");
        ///// hprtree bigelement:
        bench_hprtree_opendata(opendata_to_bigelement);
        bench_hprtree_matthe(matthe_to_bigelement);
        bench_hprtree_simplemaps(simplemaps_to_bigelement);
        bench_hprtree_synthetic_180x90x_x(1, synthetic_to_bigelement);
        bench_hprtree_synthetic_180x90x_x(4, synthetic_to_bigelement);
        bench_hprtree_synthetic_180x90x_x(16, synthetic_to_bigelement);
        bench_hprtree_synthetic_180x90x_x(64, synthetic_to_bigelement);
        bench_hprtree_synthetic_180x90x_x(256, synthetic_to_bigelement);
        bench_hprtree_random_uniform(random_to_bigelement);
        bench_hprtree_random_clustered(random_to_bigelement);
        println!("hprtree bigelement done\n");
        ///// hprtree verybigelement:
        bench_hprtree_opendata(opendata_to_verybigelement);
        bench_hprtree_matthe(matthe_to_verybigelement);
        bench_hprtree_simplemaps(simplemaps_to_verybigelement);
        bench_hprtree_synthetic_180x90x_x(1, synthetic_to_verybigelement);
        bench_hprtree_synthetic_180x90x_x(4, synthetic_to_verybigelement);
        bench_hprtree_synthetic_180x90x_x(16, synthetic_to_verybigelement);
        bench_hprtree_synthetic_180x90x_x(64, synthetic_to_verybigelement);
        bench_hprtree_synthetic_180x90x_x(256, synthetic_to_verybigelement);
        bench_hprtree_random_uniform(random_to_verybigelement);
        bench_hprtree_random_clustered(random_to_verybigelement);
        println!("hprtree verybigelement done\n");
        ///// hprtree veryverybigelement:
        bench_hprtree_opendata(opendata_to_veryverybigelement);
        bench_hprtree_matthe(matthe_to_veryverybigelement);
        bench_hprtree_simplemaps(simplemaps_to_veryverybigelement);
        bench_hprtree_synthetic_180x90x_x(1, synthetic_to_veryverybigelement);
        bench_hprtree_synthetic_180x90x_x(4, synthetic_to_veryverybigelement);
        bench_hprtree_synthetic_180x90x_x(16, synthetic_to_veryverybigelement);
        bench_hprtree_synthetic_180x90x_x(64, synthetic_to_veryverybigelement);
        bench_hprtree_synthetic_180x90x_x(256, synthetic_to_veryverybigelement);
        bench_hprtree_random_uniform(random_to_veryverybigelement);
        bench_hprtree_random_clustered(random_to_veryverybigelement);
        println!("hprtree veryveryelement done\n");
    }

    {
        // rstar build
        ///// rstar element:
        bench_rstar_opendata(opendata_to_element);
        bench_rstar_matthe(matthe_to_element);
        bench_rstar_simplemaps(simplemaps_to_element);
        bench_rstar_synthetic_180x90x_x(1, synthetic_to_element);
        bench_rstar_synthetic_180x90x_x(4, synthetic_to_element);
        bench_rstar_synthetic_180x90x_x(16, synthetic_to_element);
        bench_rstar_synthetic_180x90x_x(64, synthetic_to_element);
        bench_rstar_synthetic_180x90x_x(256, synthetic_to_element);
        bench_rstar_random_uniform(random_to_element);
        bench_rstar_random_clustered(random_to_element);
        println!("rstar element done\n");
        ///// rstar biggerelement:
        bench_rstar_opendata(opendata_to_biggerelement);
        bench_rstar_matthe(matthe_to_biggerelement);
        bench_rstar_simplemaps(simplemaps_to_biggerelement);
        bench_rstar_synthetic_180x90x_x(1, synthetic_to_biggerelement);
        bench_rstar_synthetic_180x90x_x(4, synthetic_to_biggerelement);
        bench_rstar_synthetic_180x90x_x(16, synthetic_to_biggerelement);
        bench_rstar_synthetic_180x90x_x(64, synthetic_to_biggerelement);
        bench_rstar_synthetic_180x90x_x(256, synthetic_to_biggerelement);
        bench_rstar_random_uniform(random_to_biggerelement);
        bench_rstar_random_clustered(random_to_biggerelement);
        println!("rstar biggerelement done\n");
        ///// rstar bigelement:
        bench_rstar_opendata(opendata_to_bigelement);
        bench_rstar_matthe(matthe_to_bigelement);
        bench_rstar_simplemaps(simplemaps_to_bigelement);
        bench_rstar_synthetic_180x90x_x(1, synthetic_to_bigelement);
        bench_rstar_synthetic_180x90x_x(4, synthetic_to_bigelement);
        bench_rstar_synthetic_180x90x_x(16, synthetic_to_bigelement);
        bench_rstar_synthetic_180x90x_x(64, synthetic_to_bigelement);
        //bench_rstar_synthetic_180x90x_x(256, synthetic_to_bigelement); dies
        bench_rstar_random_uniform(random_to_bigelement);
        bench_rstar_random_clustered(random_to_bigelement);
        println!("rstar bigelement done\n");
        ///// rstar verybigelement:
        bench_rstar_opendata(opendata_to_verybigelement);
        bench_rstar_matthe(matthe_to_verybigelement);
        bench_rstar_simplemaps(simplemaps_to_verybigelement);
        bench_rstar_synthetic_180x90x_x(1, synthetic_to_verybigelement);
        bench_rstar_synthetic_180x90x_x(4, synthetic_to_verybigelement);
        bench_rstar_synthetic_180x90x_x(16, synthetic_to_verybigelement);
        bench_rstar_synthetic_180x90x_x(64, synthetic_to_verybigelement);
        //bench_rstar_synthetic_180x90x_x(256, synthetic_to_verybigelement); dies
        bench_rstar_random_uniform(random_to_verybigelement);
        bench_rstar_random_clustered(random_to_verybigelement);
        println!("rstar verybigelement done\n");
        ///// rstar veryverybigelement:
        bench_rstar_opendata(opendata_to_veryverybigelement);
        bench_rstar_matthe(matthe_to_veryverybigelement);
        bench_rstar_simplemaps(simplemaps_to_veryverybigelement);
        bench_rstar_synthetic_180x90x_x(1, synthetic_to_veryverybigelement);
        bench_rstar_synthetic_180x90x_x(4, synthetic_to_veryverybigelement);
        bench_rstar_synthetic_180x90x_x(16, synthetic_to_veryverybigelement);
        //bench_rstar_synthetic_180x90x_x(64, synthetic_to_veryverybigelement); // dies
        //bench_build_rstar_synthetic_180x90x_x(256, synthetic_to_veryverybigelement); // dies
        bench_rstar_random_uniform(random_to_veryverybigelement);
        bench_rstar_random_clustered(random_to_veryverybigelement);
        println!("rstar veryverybigelement done\n");
    }

    let program_end = time::Instant::now();
    let diff = program_end - program_start;

    println!("everything done in {diff:?}");
}
