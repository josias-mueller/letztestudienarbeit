use std::{
    fs::{self, create_dir_all, File},
    io::Write,
    path::Path,
    time,
};

use csv::StringRecord;

const GEO_JSON_BASE: &'static str = "../../../data/geojson/";

fn ensure_geojson_path(p: &str) {
    create_dir_all(Path::new(&format!("{GEO_JSON_BASE}{p}"))).unwrap();
}
fn geojson_file(p: &str) -> Option<File> {
    let str = format!("{GEO_JSON_BASE}{p}");
    let p = Path::new(&str);
    return if p.exists() {
        println!("{str} preexists");
        None
    } else {
        Some(File::create(p).unwrap())
    };
}

fn timed_main() {
    let opendata_deser = |record: StringRecord| {
        assert!(record.len() == 20);
        let coords = record
            .get(record.len() - 1)
            .unwrap()
            .split_once(", ")
            .unwrap();
        let lat = coords.0.parse::<f32>().unwrap();
        let lon = coords.1.parse::<f32>().unwrap();

        Some([lon, lat])
    };
    let matthe_deser = |record: StringRecord| {
        assert!(record.len() == 10);
        let lat = match record.get(8).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        let lon = match record.get(9).unwrap().parse::<f32>() {
            Ok(data) => data,
            Err(_) => return None,
        };
        Some([lon, lat])
    };
    let simplemaps_deser = |record: StringRecord| {
        assert!(record.len() == 11);
        let lat = record.get(2).unwrap().parse::<f32>().unwrap();
        let lon = record.get(3).unwrap().parse::<f32>().unwrap();
        Some([lon, lat])
    };
    let random_deser = |record: StringRecord| {
        assert!(record.len() == 3);
        let lat = record.get(0).unwrap().parse::<f32>().unwrap();
        let lon = record.get(1).unwrap().parse::<f32>().unwrap();
        Some([lon, lat])
    };

    {
        let geojsonpath = "matthe/";
        ensure_geojson_path(geojsonpath);
        let points = read(
            b',',
            "../../../data/base/matthewproctor/worldcities-geo.csv",
            3808651,
            matthe_deser,
        );
        {
            match geojson_file(&format!("{geojsonpath}raw")) {
                None => (),
                Some(mut rawfile) => {
                    rawfile
                        .write_all(
                            make_feature_collection(&[points_to_feature(&points, None)]).as_bytes(),
                        )
                        .unwrap();
                }
            }
        }
    }
    {
        let geojsonpath = "opendata/";
        ensure_geojson_path(geojsonpath);
        let points = read(
            b';',
            "../../../data/base/opendatasoft/geonames-all-cities-with-a-population-1000.csv",
            140974,
            opendata_deser,
        );
        {
            match geojson_file(&format!("{geojsonpath}raw")) {
                None => (),
                Some(mut rawfile) => {
                    rawfile
                        .write_all(
                            make_feature_collection(&[points_to_feature(&points, None)]).as_bytes(),
                        )
                        .unwrap();
                }
            }
        }
    }
    {
        let geojsonpath = "simplem/";
        ensure_geojson_path(geojsonpath);
        let points = read(
            b',',
            "../../../data/base/simplemaps/worldcities.csv",
            44692,
            simplemaps_deser,
        );
        {
            match geojson_file(&format!("{geojsonpath}raw")) {
                None => (),
                Some(mut rawfile) => {
                    rawfile
                        .write_all(
                            make_feature_collection(&[points_to_feature(&points, None)]).as_bytes(),
                        )
                        .unwrap();
                }
            }
        }
    }
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
            let geojsonpath = format!("clustered/{filename_wo_ext}/");
            ensure_geojson_path(&geojsonpath);
            let points = read(b',', strpath, count, random_deser);
            {
                match geojson_file(&format!("{geojsonpath}raw")) {
                    None => (),
                    Some(mut rawfile) => {
                        rawfile
                            .write_all(
                                make_feature_collection(&[points_to_feature(&points, None)])
                                    .as_bytes(),
                            )
                            .unwrap();
                    }
                }
            }
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
            let geojsonpath = format!("uniform/{filename_wo_ext}/");
            ensure_geojson_path(&geojsonpath);
            let points = read(b',', strpath, count, random_deser);
            {
                match geojson_file(&format!("{geojsonpath}raw")) {
                    None => (),
                    Some(mut rawfile) => {
                        rawfile
                            .write_all(
                                make_feature_collection(&[points_to_feature(&points, None)])
                                    .as_bytes(),
                            )
                            .unwrap();
                    }
                }
            }
        }
    }
    {
        for mult in [1, 4, 16, 64, 256] {
            let mut points = Vec::with_capacity(180 * 90 * mult as usize);
            let submult = (mult as f32).sqrt();
            let d = 2f32 / submult;
            let mut x = -180f32;
            for _ in 0..(180 * submult as u32) {
                let mut y = -90f32;
                for _ in 0..(90 * submult as u32) {
                    points.push([x, y]);
                    y += d;
                }
                x += d;
            }
            assert!(points.len() == 180 * 90 * mult as usize);
            let geojsonpath = format!("ordered/{mult}/");
            ensure_geojson_path(&geojsonpath);
            let points = points;
            {
                match geojson_file(&format!("{geojsonpath}raw")) {
                    None => (),
                    Some(mut rawfile) => {
                        rawfile
                            .write_all(
                                make_feature_collection(&[points_to_feature(&points, None)])
                                    .as_bytes(),
                            )
                            .unwrap();
                    }
                }
            }
        }
    }
}
fn main() {
    let start = time::Instant::now();
    timed_main();
    let end = time::Instant::now();
    let diff = end - start;
    println!("done in {diff:?}");
}

fn read<T>(
    delimiter: u8,
    path: &str,
    count: usize,
    deser: fn(StringRecord) -> Option<T>,
) -> Vec<T> {
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

fn point_to_feature(point: &[f32; 2], col: Option<&str>) -> String {
    let col = col.or_else(|| Some("#00FF00")).unwrap();
    format!(
        "{{\"type\": \"Feature\",
        \"geometry\": {{\"type\": \"Point\",\"coordinates\": {point:?}}},
        \"properties\": {{\"marker-size\": \"small\",\"marker-color\": \"{col}\"}}}}"
    )
}
fn points_to_feature(points: &[[f32; 2]], col: Option<&str>) -> String {
    let col = col.or_else(|| Some("#00FF00")).unwrap();
    format!(
        "{{\"type\": \"Feature\",
        \"geometry\": {{\"type\": \"MultiPoint\",\"coordinates\": {points:?}}},
        \"properties\": {{\"marker-size\": \"small\",\"marker-color\": \"{col}\"}}}}"
    )
}
fn bbox_to_feature(minx: f32, maxx: f32, miny: f32, maxy: f32, col: Option<&str>) -> String {
    let col = col.or_else(|| Some("#FF0000")).unwrap();
    let arr: &[[f32; 2]; 5] = &[
        [minx, miny],
        [minx, maxy],
        [maxx, maxy],
        [maxx, miny],
        [minx, miny],
    ];
    format!(
        "{{\"type\": \"Feature\",
        \"geometry\": {{\"type\": \"Polygon\",\"coordinates\": {arr:?}}},
        \"properties\": {{\"stroke\": \"{col}\",\"fill-opacity\": 0}}}}"
    )
}
fn make_feature_collection(features: &[String]) -> String {
    let mut str = String::new();

    for (i, f) in features.iter().enumerate() {
        str.push_str(f.as_str());
        if i < features.len() - 1 {
            str.push(',');
        }
    }
    format!("{{\"type\": \"FeatureCollection\",\"features\": [{str}]}}")
}
