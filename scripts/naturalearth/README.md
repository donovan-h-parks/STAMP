# Natural Earth outlines for the map plot

The map plot (`crates/stamp-plot/src/map_plot.rs`) draws countries and lakes
from [Natural Earth](https://www.naturalearthdata.com/), which is in the
public domain ("Made with Natural Earth"). `convert.py` turns the GeoJSON
files of one Natural Earth release into the small binary files in
`crates/stamp-plot/data/`, which are compiled into STAMP with
`include_bytes!` and read by `crates/stamp-plot/src/geo.rs`.

## Reproducing the files

```bash
python3 -I scripts/naturalearth/convert.py --download /some/new/dir
python3 -I scripts/naturalearth/convert.py /some/new/dir crates/stamp-plot/data
```

Only the Python standard library is used. The download is checked against
the SHA-256 sums in the script.

## Sources

Release **v5.1.2** of the
[natural-earth-vector](https://github.com/nvkelso/natural-earth-vector)
repository (the same geometry as the 5.1.x downloads from naciscdn.org):

| Source (`geojson/`)                | SHA-256 (first 16)  | Output                  | Simplification          | Rings / vertices | Bytes   |
|------------------------------------|---------------------|-------------------------|-------------------------|------------------|---------|
| `ne_110m_admin_0_countries.geojson` | `6866c877d39cba9c` | `ne_110m_countries.bin` | none                    | 287 / 10,293     | 35,314  |
| `ne_50m_admin_0_countries.geojson`  | `3e458fc036ad0a66` | `ne_50m_countries.bin`  | Douglas-Peucker 0.02°, rings under 0.0004 deg² dropped | 1,591 / 50,305 | 122,656 |
| `ne_110m_lakes.geojson`             | `eb02ecc86c82004f` | `ne_110m_lakes.bin`     | none                    | 24 / 427         | 1,365   |
| `ne_50m_lakes.geojson`              | `d350b75978b26fe8` | `ne_50m_lakes.bin`      | Douglas-Peucker 0.02°, lakes under 0.2 deg² dropped | 110 / 4,709 | 10,752 |

URL pattern:
`https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/<name>.geojson`.

Only the outer ring of each polygon is kept. In the country files the holes
are enclaves (Lesotho, San Marino and the like), which are countries of
their own drawn in the same colour; names and other attributes are not
kept.

## Format

All integers are unsigned LEB128 varints unless stated.

```
"STMAP1"                      6 bytes
ring count
for each ring:
    vertex count              the ring is open: the first vertex is not repeated
    for each vertex:
        zigzag(dx), zigzag(dy)  differences from the previous vertex of the
                                ring (from 0, 0 for the first)
```

`x = round(lon / 180 * 32767)` and `y = round(lat / 90 * 32767)`, so a step
is 0.0055° of longitude and 0.0027° of latitude (about 600 m and 300 m),
well below what the simplified outlines can show. `zigzag(v)` is `2v` for
`v >= 0` and `-2v - 1` for `v < 0`.

The files are stored uncompressed (151 KB for all four once gzipped), so
no decompression code is needed.

## Caveats

Natural Earth shows *de facto* boundaries (Kashmir, Crimea, Western Sahara
and others are drawn as they are administered), which the documentation
notes. The polygons are already split at ±180° longitude; the map plot only
supports a central meridian of 0°, so no further cutting is needed.
