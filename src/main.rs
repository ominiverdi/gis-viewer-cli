use anyhow::{Context, Result};
use clap::Parser;
use gdal::Dataset;
use image::{DynamicImage, RgbImage};
use std::path::PathBuf;
use std::process::Command;
use viuer::Config;

// Maximum pixels to load (to avoid OOM on large rasters)
const MAX_PIXELS: usize = 4000 * 4000;

#[derive(Parser, Debug)]
#[command(name = "gis-view")]
#[command(about = "View GIS raster images in the terminal")]
#[command(version)]
struct Args {
    /// Path to the raster file (GeoTIFF, etc.)
    file: PathBuf,

    /// Bands to display as RGB (e.g., "4,3,2" for false color)
    #[arg(short, long, value_delimiter = ',')]
    bands: Option<Vec<usize>>,

    /// Output width in terminal columns (auto-detected if not set)
    #[arg(short, long)]
    width: Option<u32>,

    /// Output height in terminal rows (auto-detected if not set)
    #[arg(short = 'H', long)]
    height: Option<u32>,

    /// Show raster metadata only, don't display image
    #[arg(short, long)]
    info: bool,

    /// Percentile for contrast stretch (e.g., 2 for 2%-98%)
    #[arg(short, long, default_value = "2")]
    stretch: f64,

    /// Maximum output resolution (default: 4000, use 0 for full resolution)
    #[arg(short = 'r', long, default_value = "4000")]
    max_res: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let dataset = Dataset::open(&args.file)
        .with_context(|| format!("Failed to open: {}", args.file.display()))?;

    if args.info {
        print_metadata(&dataset)?;
        return Ok(());
    }

    let img = render_raster(&dataset, &args)?;
    display_image(&img, &args)?;

    Ok(())
}

fn print_metadata(dataset: &Dataset) -> Result<()> {
    let (width, height) = dataset.raster_size();
    let band_count = dataset.raster_count();
    let geo_transform = dataset.geo_transform().ok();
    let projection = dataset.projection();

    println!("Dimensions: {}x{}", width, height);
    println!("Bands: {}", band_count);

    if let Some(gt) = geo_transform {
        println!("Origin: ({:.6}, {:.6})", gt[0], gt[3]);
        println!("Pixel size: ({:.6}, {:.6})", gt[1], gt[5]);
    }

    if !projection.is_empty() {
        // Truncate long WKT strings
        let proj_display = if projection.len() > 80 {
            format!("{}...", &projection[..80])
        } else {
            projection
        };
        println!("Projection: {}", proj_display);
    }

    // Print band info
    for i in 1..=band_count {
        if let Ok(band) = dataset.rasterband(i) {
            let dtype = band.band_type();
            let nodata = band.no_data_value();
            print!("Band {}: {:?}", i, dtype);
            if let Some(nd) = nodata {
                print!(" (nodata: {})", nd);
            }
            println!();
        }
    }

    Ok(())
}

fn render_raster(dataset: &Dataset, args: &Args) -> Result<DynamicImage> {
    let (src_width, src_height) = dataset.raster_size();
    let band_count = dataset.raster_count();

    // Calculate output dimensions (downsample if needed)
    let (out_width, out_height) = if args.max_res > 0 {
        let max_dim = args.max_res;
        let scale = (max_dim as f64 / src_width.max(src_height) as f64).min(1.0);
        (
            ((src_width as f64 * scale) as usize).max(1),
            ((src_height as f64 * scale) as usize).max(1),
        )
    } else {
        // Full resolution, but cap at MAX_PIXELS
        let total = src_width * src_height;
        if total > MAX_PIXELS {
            let scale = (MAX_PIXELS as f64 / total as f64).sqrt();
            (
                ((src_width as f64 * scale) as usize).max(1),
                ((src_height as f64 * scale) as usize).max(1),
            )
        } else {
            (src_width, src_height)
        }
    };

    if out_width != src_width || out_height != src_height {
        eprintln!(
            "Downsampling {}x{} -> {}x{} for display",
            src_width, src_height, out_width, out_height
        );
    }

    // Determine which bands to use
    let bands = match &args.bands {
        Some(b) if b.len() >= 3 => vec![b[0], b[1], b[2]],
        Some(b) if b.len() == 1 => vec![b[0], b[0], b[0]], // Grayscale
        _ if band_count >= 3 => vec![1, 2, 3],             // Default RGB
        _ => vec![1, 1, 1],                                // Single band grayscale
    };

    // Read bands with GDAL-side resampling
    let red = read_band_resampled(
        dataset, bands[0], src_width, src_height, out_width, out_height,
    )?;
    let green = read_band_resampled(
        dataset, bands[1], src_width, src_height, out_width, out_height,
    )?;
    let blue = read_band_resampled(
        dataset, bands[2], src_width, src_height, out_width, out_height,
    )?;

    // Get nodata value from first band
    let nodata = dataset.rasterband(bands[0])?.no_data_value();

    // Normalize each band
    let stretch = args.stretch / 100.0;
    let red_norm = normalize_percentile(&red, nodata, stretch, 1.0 - stretch);
    let green_norm = normalize_percentile(&green, nodata, stretch, 1.0 - stretch);
    let blue_norm = normalize_percentile(&blue, nodata, stretch, 1.0 - stretch);

    // Create RGB image
    let mut img = RgbImage::new(out_width as u32, out_height as u32);
    for y in 0..out_height {
        for x in 0..out_width {
            let idx = y * out_width + x;
            img.put_pixel(
                x as u32,
                y as u32,
                image::Rgb([red_norm[idx], green_norm[idx], blue_norm[idx]]),
            );
        }
    }

    Ok(DynamicImage::ImageRgb8(img))
}

fn read_band_resampled(
    dataset: &Dataset,
    band_idx: usize,
    src_width: usize,
    src_height: usize,
    out_width: usize,
    out_height: usize,
) -> Result<Vec<f64>> {
    let band = dataset
        .rasterband(band_idx)
        .with_context(|| format!("Failed to read band {}", band_idx))?;

    // GDAL read_as with different buffer size does resampling
    let data: Vec<f64> = band
        .read_as::<f64>(
            (0, 0),
            (src_width, src_height),
            (out_width, out_height),
            None,
        )
        .with_context(|| format!("Failed to read data from band {}", band_idx))?
        .data()
        .to_vec();

    Ok(data)
}

fn normalize_percentile(
    values: &[f64],
    nodata: Option<f64>,
    low_pct: f64,
    high_pct: f64,
) -> Vec<u8> {
    // Filter out nodata values for statistics
    let mut valid: Vec<f64> = values
        .iter()
        .filter(|&&v| {
            if let Some(nd) = nodata {
                (v - nd).abs() > f64::EPSILON && v.is_finite()
            } else {
                v.is_finite()
            }
        })
        .copied()
        .collect();

    if valid.is_empty() {
        return vec![0u8; values.len()];
    }

    valid.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let low_idx = ((valid.len() as f64 * low_pct) as usize).min(valid.len() - 1);
    let high_idx = ((valid.len() as f64 * high_pct) as usize).min(valid.len() - 1);

    let min_val = valid[low_idx];
    let max_val = valid[high_idx];
    let range = max_val - min_val;

    values
        .iter()
        .map(|&v| {
            if let Some(nd) = nodata {
                if (v - nd).abs() < f64::EPSILON {
                    return 0u8;
                }
            }
            if !v.is_finite() {
                return 0u8;
            }
            if range > 0.0 {
                (((v - min_val) / range) * 255.0).clamp(0.0, 255.0) as u8
            } else {
                128u8
            }
        })
        .collect()
}

fn display_image(img: &DynamicImage, _args: &Args) -> Result<()> {
    // Check if we're in Kitty terminal
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        // Use kitten icat for guaranteed pixel-perfect rendering
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("gis-view-temp.png");

        img.save(&temp_path)
            .context("Failed to save temporary image")?;

        let status = Command::new("kitten")
            .args(["icat", "--align", "left"])
            .arg(&temp_path)
            .status()
            .context("Failed to run kitten icat")?;

        // Clean up
        let _ = std::fs::remove_file(&temp_path);

        if !status.success() {
            anyhow::bail!("kitten icat failed");
        }
    } else {
        // Fall back to viuer for other terminals
        let config = Config {
            absolute_offset: false,
            use_kitty: true,
            use_iterm: true,
            ..Default::default()
        };
        viuer::print(img, &config).context("Failed to display image")?;
    }

    Ok(())
}
