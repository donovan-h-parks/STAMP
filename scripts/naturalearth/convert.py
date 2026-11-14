#!/usr/bin/env python3
"""Convert Natural Earth GeoJSON into the compact outlines STAMP's map plot
draws (crates/stamp-plot/data/*.bin). Standard library only.

    python3 -I scripts/naturalearth/convert.py --download SOURCE_DIR
    python3 -I scripts/naturalearth/convert.py SOURCE_DIR crates/stamp-plot/data

The first form downloads the GeoJSON files of the pinned Natural Earth
release into SOURCE_DIR and checks their SHA-256 sums; the second writes the
.bin files. See README.md beside this script for the format.
"""

import hashlib
import json
import os
import sys
import urllib.request

RELEASE = "v5.1.2"
URL = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/{}/geojson/{}.geojson"

# Source file, SHA-256 of the release's file, output file, Douglas-Peucker
# tolerance (degrees; 0 keeps every vertex), smallest ring area kept
# (square degrees).
SOURCES = [
    ("ne_110m_admin_0_countries",
     "6866c877d39cba9c357620878839b336d569f8c662d3cfab4cb1dbe2d39c977f",
     "ne_110m_countries.bin", 0.0, 0.0),
    ("ne_50m_admin_0_countries",
     "3e458fc036ad0a66411f2c1e6cac49c5d7bfb81cb1123bc513b22511a2b7fdeb",
     "ne_50m_countries.bin", 0.02, 0.0004),
    ("ne_110m_lakes",
     "eb02ecc86c82004fccbf979058bfabbbd6c2d07968c7844d38eb1c9152d2ffc9",
     "ne_110m_lakes.bin", 0.0, 0.0),
    ("ne_50m_lakes",
     "d350b75978b26fe839b797c2c529b2fb8f47fb3983c03f4964e36d5df9378a52",
     "ne_50m_lakes.bin", 0.02, 0.2),
]

MAGIC = b"STMAP1"
# Longitude and latitude are stored as integers: lon / 180 * SCALE and
# lat / 90 * SCALE, so one step is about 0.0055 degrees of longitude.
SCALE = 32767


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def download(directory):
    os.makedirs(directory, exist_ok=True)
    for name, digest, _, _, _ in SOURCES:
        path = os.path.join(directory, name + ".geojson")
        urllib.request.urlretrieve(URL.format(RELEASE, name), path)
        if sha256(path) != digest:
            sys.exit(f"{path}: unexpected SHA-256; not the {RELEASE} file")
        print(f"downloaded {path}")


def outer_rings(path):
    """The outer ring of every polygon. Holes are left out: in the country
    files they are enclaves (Lesotho in South Africa, say), drawn as
    countries of their own in the same colour."""
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    rings = []
    for feature in data["features"]:
        geometry = feature["geometry"]
        if geometry is None:
            continue
        if geometry["type"] == "Polygon":
            rings.append(geometry["coordinates"][0])
        elif geometry["type"] == "MultiPolygon":
            rings.extend(polygon[0] for polygon in geometry["coordinates"])
    return rings


def douglas_peucker(points, tolerance):
    if tolerance <= 0 or len(points) < 3:
        return points
    keep = [False] * len(points)
    keep[0] = keep[-1] = True
    stack = [(0, len(points) - 1)]
    limit = tolerance * tolerance
    while stack:
        a, b = stack.pop()
        (ax, ay), (bx, by) = points[a], points[b]
        dx, dy = bx - ax, by - ay
        length = dx * dx + dy * dy
        best, best_index = -1.0, -1
        for i in range(a + 1, b):
            px, py = points[i]
            if length == 0:
                d = (px - ax) ** 2 + (py - ay) ** 2
            else:
                t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length))
                d = (px - ax - t * dx) ** 2 + (py - ay - t * dy) ** 2
            if d > best:
                best, best_index = d, i
        if best > limit:
            keep[best_index] = True
            stack += [(a, best_index), (best_index, b)]
    return [p for p, k in zip(points, keep) if k]


def area(ring):
    return abs(sum(ring[i - 1][0] * ring[i][1] - ring[i][0] * ring[i - 1][1]
                   for i in range(len(ring)))) / 2


def quantise(lon, lat):
    x = max(-SCALE, min(SCALE, round(lon / 180 * SCALE)))
    y = max(-SCALE, min(SCALE, round(lat / 90 * SCALE)))
    return x, y


def varint(value, out):
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)


def zigzag(value):
    return (value << 1) if value >= 0 else ((-value) << 1) - 1


def encode(rings, tolerance, min_area):
    kept = []
    for ring in rings:
        simplified = douglas_peucker(ring, tolerance)
        # Open the ring (GeoJSON repeats the first vertex at the end) and
        # drop vertices that quantise onto the one before.
        points = []
        for lon, lat in simplified[:-1] if simplified[0] == simplified[-1] else simplified:
            q = quantise(lon, lat)
            if not points or points[-1] != q:
                points.append(q)
        while len(points) > 1 and points[0] == points[-1]:
            points.pop()
        if len(points) < 3 or area(points) * (180 / SCALE) * (90 / SCALE) < min_area:
            continue
        kept.append(points)
    out = bytearray(MAGIC)
    varint(len(kept), out)
    for points in kept:
        varint(len(points), out)
        px = py = 0
        for x, y in points:
            varint(zigzag(x - px), out)
            varint(zigzag(y - py), out)
            px, py = x, y
    return bytes(out), len(kept), sum(len(p) for p in kept)


def convert(source, target):
    os.makedirs(target, exist_ok=True)
    for name, digest, output, tolerance, min_area in SOURCES:
        path = os.path.join(source, name + ".geojson")
        if sha256(path) != digest:
            print(f"warning: {path} is not the {RELEASE} file", file=sys.stderr)
        data, rings, vertices = encode(outer_rings(path), tolerance, min_area)
        with open(os.path.join(target, output), "wb") as f:
            f.write(data)
        print(f"{output}: {rings} rings, {vertices} vertices, {len(data)} bytes")


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--download":
        download(sys.argv[2])
    elif len(sys.argv) == 3:
        convert(sys.argv[1], sys.argv[2])
    else:
        sys.exit(__doc__)
