# de oder en?

-------------------------------

| Structure   |      Rust      |  Java |
|----------|:-------------:|------:|
| HPRTree |  https://crates.io/crates/hprtree | https://github.com/locationtech/jts |
| R*Tree |    https://crates.io/crates/rstar   |   https://github.com/davidmoten/rtree |


die dinger wollen nicht so wie ich will:

| KDBush | https://crates.io/crates/kdbush |    https://github.com/imvladikon/kdbush |

-------------------------------

- [ ] Why do this
- [ ] Why are these used for this
- [~] Datastructures
    - [x] Which ones
    - [ ] Why
    - [ ] Compare
- [x] Datasets
    - [x] Find Real ones
    - [x] Make Synthetic ones
- [x] Build
    - [x] How to Build
    - [x] Bench Build
- [x] Size
    - [x] How to measure
    - [x] Bench
- [~] Queries
    - [x] How to query
    - [~] Bench
- [ ] Make Comparisons


-------------------------------

Why do this?

interesting problem, 


-------------------------------


- Synthetic Datasets & Build done bis 20.07.

- Size done bis 23.07.

- Queries done bis 04.08.

- Vergleiche / Datenarbeit fertig bis 13.08.

- Schreiben done bis  31.08.


-------------------------------
# Elements


- Element - 12 Byte (lat, lon, 32 Bit ID)

- BiggerElement - 24 Byte (lat, lon, 2x32 Bit ID,64bit ID derivat)

- BigElement - 256 Byte (lat, lon, 31x64 Bit Daten)

- VeryBigElement - 512 Byte (lat, lon, 63x64 Bit Daten)

- VeryVeryBigElement - 1024 Byte (lat, lon, 127x64 Bit Daten)

-------------------------------

# Datasets

## Real

### matthewproctor
https://www.matthewproctor.com/worldwide_cities
(theoretisch 4,384,909) 3,808,651

### opendatasoft
https://public.opendatasoft.com/explore/dataset/geonames-all-cities-with-a-population-1000
140,974

### simplemaps
https://simplemaps.com/data/world-cities
44,692

## Synthetic

### Symmetric, Ordered

x (lon) = [-180; 180) ; step = 2 / (mult/2)

y (lat) = [-90; 90) ; step = 2 / (mult/2)

- ### 180x90x1

    - 16,200

- ### 180x90x4

    - 64,800

- ### 180x90x16

    - 259,200

- ### 180x90x64

    - 1,036,800

- ### 180x90x256

    - 4,147,200

### Random

mehrere von selben typ

#### Fully Random
size of real & symmetric data

#### Random Clustered
~size of real & symmetric data

2, 4, 8, 16, 32 clusters


-------------------------------
# Queries

- fullsize queries - problem: nicht wirklich der usecase
- pro datenset iwi queries festsetzen? - yes
- ?