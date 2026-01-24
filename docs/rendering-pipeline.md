# Rendering Pipeline

## Overview

```
GeoTIFF → Read Bands → Normalize → RGB Image → Terminal Output
```

## Stage 1: Read Raster Data

```rust
// Using GDAL
let dataset = Dataset::open(path)?;
let band = dataset.rasterband(1)?;
let data: Vec<f64> = band.read_as((0, 0), band.size(), band.size())?;
```

**Considerations:**
- Handle NoData values
- Read only required bands
- For large files, read at reduced resolution

## Stage 2: Band Selection

User specifies which bands map to RGB:

| Input | Interpretation |
|-------|----------------|
| `--bands 1` | Single band → grayscale |
| `--bands 4,3,2` | Bands 4→R, 3→G, 2→B |
| (default) | First 3 bands or grayscale |

## Stage 3: Value Normalization

Raw raster values must be scaled to 0-255.

### Linear Stretch

```rust
fn normalize_linear(values: &[f64], nodata: Option<f64>) -> Vec<u8> {
    let valid: Vec<f64> = values.iter()
        .filter(|v| nodata.map_or(true, |nd| *v != &nd))
        .copied()
        .collect();
    
    let min = valid.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = valid.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let range = max - min;
    
    values.iter().map(|v| {
        if range > 0.0 {
            ((v - min) / range * 255.0).clamp(0.0, 255.0) as u8
        } else {
            128u8
        }
    }).collect()
}
```

### Percentile Clip (Recommended)

Clips outliers for better contrast:

```rust
fn normalize_percentile(values: &[f64], low: f64, high: f64) -> Vec<u8> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let low_idx = (sorted.len() as f64 * low) as usize;
    let high_idx = (sorted.len() as f64 * high) as usize;
    
    let min = sorted[low_idx];
    let max = sorted[high_idx];
    // ... normalize using min/max
}
```

## Stage 4: Create RGB Image

```rust
use image::{RgbImage, Rgb};

fn create_rgb_image(
    red: &[u8], 
    green: &[u8], 
    blue: &[u8],
    width: u32,
    height: u32
) -> RgbImage {
    let mut img = RgbImage::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            img.put_pixel(x, y, Rgb([red[idx], green[idx], blue[idx]]));
        }
    }
    
    img
}
```

## Stage 5: Terminal Rendering

```rust
use viuer::{Config, print};

fn display_image(img: &DynamicImage) -> Result<()> {
    let config = Config {
        width: Some(80),  // or terminal width
        height: Some(24),
        ..Default::default()
    };
    
    print(img, &config)?;
    Ok(())
}
```

`viuer` automatically:
1. Detects terminal capabilities
2. Resizes image to fit
3. Selects best protocol
4. Renders output

## Resolution Handling

For large rasters, downsample before display:

```rust
// GDAL overview reading
let overview_count = band.overview_count()?;
if overview_count > 0 {
    let overview = band.overview(overview_count - 1)?;  // smallest
    // read from overview
}

// Or resize with image crate
let thumbnail = img.thumbnail(terminal_width * 2, terminal_height * 4);
```

## Pipeline Configuration

```rust
struct RenderConfig {
    bands: Vec<usize>,
    stretch: StretchMethod,
    width: Option<u32>,
    height: Option<u32>,
    colormap: Option<Colormap>,
}

enum StretchMethod {
    Linear,
    Percentile(f64, f64),
    Histogram,
    None,
}
```
