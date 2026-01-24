# GIS Data Handling

## Supported Formats

### Primary: GeoTIFF

Industry standard for georeferenced raster data.

**Characteristics:**
- Embeds coordinate reference system (CRS)
- Supports multiple bands
- Various compression options (LZW, DEFLATE, JPEG)
- Tiled or stripped organization

### Cloud Optimized GeoTIFF (COG)

GeoTIFF optimized for HTTP range requests.

**Advantages for CLI:**
- Can read subsets without downloading entire file
- Internal overviews for quick previews
- Standard format, growing adoption

### Other Formats

- JPEG/PNG (via `image` crate, no georeferencing)
- NetCDF, HDF5 (requires GDAL)

## GDAL vs Pure-Rust

### With GDAL Bindings

**Pros:**
- 200+ format support
- Full projection handling
- Industry-proven reliability
- Handles edge cases

**Cons:**
- System dependency (libgdal)
- Harder to distribute as single binary
- ~50MB+ dependency

**Installation:**
```bash
# macOS
brew install gdal

# Ubuntu/Debian
apt install libgdal-dev

# Fedora
dnf install gdal-devel
```

### Pure-Rust Approach

**Crates:**
- `tiff` - Basic TIFF reading
- `geotiff` - GeoTIFF metadata (limited)

**Pros:**
- Single binary distribution
- No system dependencies
- Faster compilation

**Cons:**
- Limited format support
- May not handle all GeoTIFF variants
- No projection transformations

## Recommended Approach

Start with GDAL for full compatibility. Consider pure-Rust for a "lite" version later.

## Band Handling

GIS rasters often have multiple bands:

| Use Case | Typical Bands |
|----------|---------------|
| Natural color | Red, Green, Blue |
| False color | NIR, Red, Green |
| Single band | Elevation, temperature |

### Display Strategy

1. **RGB image**: Use bands directly
2. **Multi-band**: Allow user to select 3 bands for RGB composite
3. **Single band**: Apply colormap (grayscale or pseudocolor)

## Value Normalization

Raster values need scaling to 0-255 for display:

```
normalized = (value - min) / (max - min) * 255
```

**Stretch options:**
- Linear (min-max)
- Histogram equalization
- Standard deviation stretch
- Percentile clip (2%-98%)
