# Changelog

## [0.2.3] - 2026-02-06

### Added
- Direct Kitty graphics protocol implementation for SSH support
- `--protocol` flag to force display mode (kitty, iterm, or blocks)
- Published to [crates.io](https://crates.io/crates/gis-viewer-cli)

### Fixed
- Color rendering: global percentile stretch across all bands preserves color relationships (per-band stretch was producing grayscale)
- ZIP opening: try direct open before `/vsizip/` prefix (fixes Sentinel-2C products on newer GDAL)
- File existence check no longer blocks GDAL subdataset paths

### Changed
- `cargo install gis-viewer-cli` is now the recommended installation method
- Updated installation docs with SSH usage instructions

## [0.2.2] - 2026-02-05

### Added
- Vector file support: GeoJSON, Shapefile, GeoPackage, and any OGR-supported format
- `--layer` flag for selecting layers in multi-layer vector files
- `--info` shows vector layer details (features, geometry type, extent, CRS, fields)
- Interactive layer selection for vector files with `-i`
- Custom vector rendering with Bresenham line drawing algorithm

### Fixed
- File existence check with clear error message instead of cryptic GDAL error
- Improved error message for corrupted ZIP files
- Display workaround for small images removed (viuer handles them correctly)
- Windows CI: `gdal_i.lib` compatibility with vcpkg

## [0.2.1] - 2026-02-05

### Fixed
- Auto-detect container files (Sentinel-2 ZIP, HDF, NetCDF) and switch to interactive mode
- macOS Intel build: use macos-15-intel runner (macos-13 retired)

## [0.2.0] - 2026-01-24

### Added
- Multi-version Linux release builds (Ubuntu 22.04/GDAL 3.4, Ubuntu 24.04/GDAL 3.8)
- Windows x64 build in release workflow

## [0.1.0] - 2026-01-24

### Added
- Initial release
- GIS raster viewing in the terminal (GeoTIFF, COG, Sentinel-2, HDF, NetCDF)
- Interactive mode for subdataset and band selection
- Full pixel rendering in Kitty terminal (Unicode half-blocks fallback)
- Direct reading from zipped Sentinel-2 SAFE files
- Band selection for custom RGB composites
- Automatic downsampling for large rasters
- Percentile-based contrast stretching
- GIS metadata display with `--info`
- macOS (Intel + Apple Silicon) and Linux builds
