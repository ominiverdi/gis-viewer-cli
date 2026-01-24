# Rust Crate Selection

## Core Dependencies

### CLI Framework

**Selected: `clap`**

```toml
clap = { version = "4", features = ["derive"] }
```

Industry standard. Derive macros for clean argument parsing.

### Image Processing

**Selected: `image`**

```toml
image = "0.25"
```

Pure-Rust image handling. Converts between formats, basic operations.

### Terminal Rendering

**Selected: `viuer`**

```toml
viuer = "0.7"
```

Auto-detects terminal capabilities and renders appropriately.

**Supports:**
- Kitty graphics protocol
- iTerm2 inline images
- Sixel
- Unicode block fallback

### GIS Raster Reading

**Option A: `gdal` (Recommended)**

```toml
gdal = "0.16"
```

Full GDAL bindings. Requires libgdal system installation.

**Option B: Pure-Rust stack**

```toml
tiff = "0.9"
# geotiff support is limited in pure-Rust ecosystem
```

## Optional Dependencies

### Error Handling

```toml
anyhow = "1.0"      # Application errors
thiserror = "1.0"   # Library errors
```

### Progress Indication

```toml
indicatif = "0.17"  # Progress bars for large files
```

### Async (for COG HTTP reads)

```toml
tokio = { version = "1", features = ["rt-multi-thread"] }
reqwest = "0.11"
```

## Dependency Graph

```
gis-viewer-cli
├── clap (CLI parsing)
├── gdal (raster reading)
│   └── libgdal (system)
├── image (image conversion)
├── viuer (terminal display)
│   ├── kitty protocol
│   ├── iterm2 protocol
│   ├── sixel
│   └── crossterm (fallback)
└── anyhow (errors)
```

## Build Considerations

### Static Linking GDAL

Possible but complex. Not recommended for initial development.

### Feature Flags

Consider splitting functionality:

```toml
[features]
default = ["gdal-backend"]
gdal-backend = ["gdal"]
pure-rust = ["tiff"]
```

## Minimal Cargo.toml

```toml
[package]
name = "gis-viewer-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
gdal = "0.16"
image = "0.25"
viuer = "0.7"
anyhow = "1.0"
```
