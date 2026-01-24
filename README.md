# gis-viewer-cli

View GIS raster images directly in the terminal. Supports Sentinel-2, GeoTIFF, COG, and any GDAL-supported format.

## Features

- **Full pixel rendering** in Kitty terminal (falls back to Unicode blocks elsewhere)
- **Read directly from zipped files** (Sentinel-2 SAFE.zip, etc.)
- **Band selection** for custom RGB composites
- **Automatic downsampling** for large rasters
- **Percentile stretch** for contrast enhancement
- **GIS metadata display**

## Installation

### Prerequisites

- Rust toolchain
- GDAL library

```bash
# macOS
brew install gdal

# Ubuntu/Debian
sudo apt install libgdal-dev

# Build
cargo build --release
```

Binary will be at `target/release/gis-view`.

## Usage

```bash
# View a GeoTIFF
gis-view satellite.tif

# Show metadata only
gis-view satellite.tif --info

# Custom band selection (NIR-Red-Green false color)
gis-view image.tif --bands 4,3,2

# Adjust contrast stretch (default: 2%)
gis-view image.tif --stretch 5

# Control output resolution
gis-view large-image.tif --max-res 2000
```

### Sentinel-2 from ZIP

```bash
# True Color Image
gis-view "SENTINEL2_L2A:/vsizip//path/to/S2A_MSIL2A_*.SAFE.zip/*/MTD_MSIL2A.xml:TCI:EPSG_32630"

# 10m bands with false color composite
gis-view "SENTINEL2_L2A:/vsizip//path/to/S2A_MSIL2A_*.SAFE.zip/*/MTD_MSIL2A.xml:10m:EPSG_32630" --bands 4,3,2
```

### Band Combinations

| Name | Bands | Description |
|------|-------|-------------|
| True color | `3,2,1` | Natural looking |
| False color | `4,3,2` | Vegetation appears red |
| NIR | `4,4,4` | Single band grayscale |

## Terminal Support

| Terminal | Quality |
|----------|---------|
| Kitty | Full pixels (best) |
| iTerm2 | Full pixels |
| WezTerm | Full pixels |
| Others | Unicode half-blocks |

For best results, use [Kitty](https://sw.kovidgoyal.net/kitty/).

## License

MIT
