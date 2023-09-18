#![feature(float_next_up_down)]

use std::{
    fs::{self, create_dir_all, File},
    io::Write,
    path::Path,
    thread::sleep,
    time::{self, Duration},
};

use rand::prelude::*;

use csv::StringRecord;
use hprtree::{BBox, HPRTree, HPRTreeBuilder, Point};

const ENV_SIZES: [usize; 5] = [16, 64, 256, 1024, 4096];
const ENV_COUNT: usize = 16;
const MAX_ENV: BBox = BBox {
    minx: -180f32,
    maxx: 180f32,
    miny: -90f32,
    maxy: 90f32,
};

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

#[derive(Clone, Debug)]
struct Element {
    pub lat: f32,
    pub lon: f32,
    pub id: u32,
}

fn build_hprtree<T>(
    delimiter: u8,
    deser: fn(StringRecord) -> Option<(T, Point)>,
    count: usize,
    path: &str,
) -> HPRTree<T>
where
    T: Clone,
{
    let data = read(delimiter, path, count, deser);

    let mut treebuilder = hprtree::HPRTreeBuilder::new(data.len());
    for e in data {
        treebuilder.insert(e.0, e.1);
    }
    treebuilder.build()
}

#[derive(PartialEq, Eq)]
enum BBoxPart {
    MINX,
    MAXX,
    MINY,
    MAXY,
}
use BBoxPart::*;

fn gen_envelopes(tree: HPRTree<Element>, path: &str) {
    let mut rng = thread_rng();
    for size in ENV_SIZES {
        let path = format!("{}.{}", path, size);
        let p = Path::new(&path);

        if p.exists() {
            println!("skipping {p:?}, preexists");
            continue;
        }

        let mut envelopes = Vec::with_capacity(ENV_COUNT);
        println!("target: {size}");

        for i in 0..ENV_COUNT {
            let mut env = tree.extent();
            let mut too_small = tree.query(&env).len() < size;

            let mut loop_count = 0;
            let mut info = false;
            let start = time::Instant::now();
            loop {
                if info {
                    println!(
                    "\tattempt {i}: Envelope{{minx: {}, maxx: {}, miny: {}, maxy: {}}} ({}, {})",
                    env.minx,
                    env.maxx,
                    env.miny,
                    env.maxy,
                    tree.query(&env).len(),
                    if too_small { "too small" } else { "too big" }
                );
                }
                let mut parts = vec![MINX, MAXX, MINY, MAXY];

                if too_small {
                    if env.minx <= MAX_ENV.minx {
                        parts.retain(|e| *e != MINX);
                    }
                    if env.miny <= MAX_ENV.miny {
                        parts.retain(|e| *e != MINY);
                    }
                    if env.maxx >= MAX_ENV.maxx {
                        parts.retain(|e| *e != MAXX);
                    }
                    if env.maxy >= MAX_ENV.maxy {
                        parts.retain(|e| *e != MAXY);
                    }
                } else {
                    if env.minx >= env.maxx {
                        parts.retain(|e| *e != MINX);
                    }
                    if env.miny >= env.maxy {
                        parts.retain(|e| *e != MINY);
                    }
                    if env.maxx <= env.minx {
                        parts.retain(|e| *e != MAXX);
                    }
                    if env.maxy <= env.miny {
                        parts.retain(|e| *e != MAXY);
                    }
                }
                assert!(parts.len() != 0);

                match parts.choose(&mut rng).unwrap() {
                    MINX => {
                        let max_delta = if too_small { MAX_ENV.minx } else { env.maxx } - env.minx;
                        if info {
                            println!("\tmax delta minx: {max_delta}");
                        }
                        let delta = rng.gen_range(
                            0f32.next_up().min(max_delta)..max_delta.max(0f32.next_up()),
                        );
                        env.minx += delta;
                    }
                    MINY => {
                        let max_delta = if too_small { MAX_ENV.miny } else { env.maxy } - env.miny;
                        if info {
                            println!("\tmax delta miny: {max_delta}");
                        }
                        let delta = rng.gen_range(
                            0f32.next_up().min(max_delta)..max_delta.max(0f32.next_up()),
                        );
                        env.miny += delta;
                    }
                    MAXX => {
                        let max_delta = if too_small { MAX_ENV.maxx } else { env.minx } - env.maxx;
                        if info {
                            println!("\tmax delta maxx: {max_delta}");
                        }
                        let delta = rng.gen_range(
                            0f32.next_up().min(max_delta)..max_delta.max(0f32.next_up()),
                        );
                        env.maxx += delta;
                    }
                    MAXY => {
                        let max_delta = if too_small { MAX_ENV.maxy } else { env.miny } - env.maxy;
                        if info {
                            println!("\tmax delta maxy: {max_delta}");
                        }
                        let delta = rng.gen_range(
                            0f32.next_up().min(max_delta)..max_delta.max(0f32.next_up()),
                        );
                        env.maxy += delta;
                    }
                }

                let count = tree.query(&env).len();
                too_small = match count.cmp(&size) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Greater => false,
                    std::cmp::Ordering::Equal => break,
                };
                loop_count += 1;
                if info {
                    info = false;
                } else if loop_count % 100000 == 0 {
                    println!("\titeration {loop_count}");
                    info = true;
                }
            }
            let end = time::Instant::now();
            let diff = end - start;
            println!(
                "\tfinal {i}: Envelope{{minx: {}, maxx: {}, miny: {}, maxy: {}}} ({loop_count} iter in {diff:?})",
                env.minx, env.maxx, env.miny, env.maxy
            );
            envelopes.push(env);
        }

        create_dir_all(p.parent().unwrap()).unwrap();
        let mut file = File::create(p).unwrap();
        for env in envelopes {
            file.write_fmt(format_args!(
                "{},{},{},{}\n",
                env.minx, env.maxx, env.miny, env.maxy
            ))
            .unwrap();
        }
    }
}

fn main() {
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
    let synthetic_to_element =
        |x: f32, y: f32, id: u32| (Element { lat: y, lon: x, id }, Point { x, y });
    let random_to_element = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        let id = record.get(2).unwrap().parse::<u32>().unwrap();
        Some((Element { lat, lon, id }, hprtree::Point { x: lon, y: lat }))
    };

    gen_envelopes(
        build_hprtree(
            b',',
            matthe_to_element,
            3808651,
            "../../../data/base/matthewproctor/worldcities-geo.csv",
        ),
        "../../../data/envelopes/base/matthewproctor/worldcities-geo.csv",
    );
    gen_envelopes(
        build_hprtree(
            b';',
            opendata_to_element,
            140974,
            "../../../data/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv",
        ),
        "../../../data/envelopes/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv",
    );
    gen_envelopes(
        build_hprtree(
            b',',
            simplemaps_to_element,
            44692,
            "../../../data/base/simplemaps/worldcities.csv",
        ),
        "../../../data/envelopes/base/simplemaps/worldcities.csv",
    );
    {
        let paths = fs::read_dir("../../../data/new/clustered/").unwrap();
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
            let strpath = p.to_str().unwrap();
            gen_envelopes(
                build_hprtree(b',', random_to_element, count, strpath),
                strpath.replace("/data", "/data/envelopes").as_str(),
            );
        }
    }
    {
        let paths = fs::read_dir("../../../data/new/uniform/").unwrap();
        for path in paths {
            let path = path.unwrap();
            if !path.metadata().unwrap().is_file() {
                continue;
            }
            let filename = path.file_name();
            let filename_str = filename.to_str().unwrap();
            let filename_wo_ext = &filename_str[..(filename_str.len() - 4)];
            let p = path.path();
            let strpath = p.to_str().unwrap();

            let count = filename_wo_ext
                .split_once('_')
                .unwrap()
                .0
                .parse::<usize>()
                .unwrap();
            gen_envelopes(
                build_hprtree(b',', random_to_element, count, strpath),
                strpath.replace("/data", "/data/envelopes").as_str(),
            );
        }
    }
    {
        for mult in [1, 4, 16, 64, 256] {
            let submult = (mult as f32).sqrt();
            let d = 2f32 / submult;
            let mut builder = HPRTreeBuilder::new(180 * 90 * mult);
            let mut x = -180f32;
            for i in 0..(180 * submult as u32) {
                let mut y = -90f32;
                for j in 0..(90 * submult as u32) {
                    let elem = synthetic_to_element(x, y, i * 10000u32 + j);
                    builder.insert(elem.0, elem.1);
                    y += d;
                }
                x += d;
            }
            assert!(builder.len() == 180 * 90 * mult);
            gen_envelopes(
                builder.build(),
                format!("../../../data/envelopes/ordered/{}", mult).as_str(),
            );
        }
    }
}
