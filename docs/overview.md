# Project Overview

## Goal

Display high-resolution GIS raster images directly in terminal environments, with first-class support for headless/SSH workflows on cloud servers.

## Scope

### In Scope

- Read common GIS raster formats (GeoTIFF, COG)
- Render images using terminal graphics protocols
- Fallback to Unicode block characters for basic terminals
- Display basic metadata (CRS, bounds, dimensions)
- Band selection and basic stretching

### Out of Scope (v1)

- Vector data support
- Interactive pan/zoom (TUI)
- Tile server functionality
- Writing/editing raster data

## High-Level Architecture

```
┌─────────────────────────────────────────────────────┐
│                   CLI Interface                     │
│                   (clap)                            │
├─────────────────────────────────────────────────────┤
│                   Core Library                      │
├──────────────────┬──────────────────────────────────┤
│  Raster Reader   │  Terminal Renderer              │
│  (gdal/tiff)     │  (viuer)                        │
├──────────────────┼──────────────────────────────────┤
│  GeoTIFF         │  Kitty Protocol                 │
│  COG             │  iTerm2 Protocol                │
│  PNG/JPEG        │  Sixel                          │
│                  │  Unicode Blocks (fallback)      │
└──────────────────┴──────────────────────────────────┘
```

## Target Usage

```bash
# Basic view
gis-view satellite.tif

# RGB composite with band selection
gis-view satellite.tif --bands 4,3,2

# With histogram stretch
gis-view satellite.tif --bands 4,3,2 --stretch histogram

# Show metadata only
gis-view satellite.tif --info
```
