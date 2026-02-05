use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Select};
use gdal::{Dataset, Metadata};
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
    #[arg(long)]
    info: bool,

    /// Percentile for contrast stretch (e.g., 2 for 2%-98%)
    #[arg(short, long, default_value = "2")]
    stretch: f64,

    /// Maximum output resolution (default: 4000, use 0 for full resolution)
    #[arg(short = 'r', long, default_value = "4000")]
    max_res: usize,

    /// Interactive mode - select subdataset and bands interactively
    #[arg(short, long)]
    interactive: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle interactive mode
    if args.interactive {
        return run_interactive(&args);
    }

    // Build the path - handle ZIP files with /vsizip/ prefix
    let path_str = args.file.to_string_lossy();
    let gdal_path = if path_str.ends_with(".zip") || path_str.ends_with(".ZIP") {
        format!("/vsizip/{}", path_str)
    } else {
        path_str.to_string()
    };

    let dataset = Dataset::open(&gdal_path)
        .with_context(|| format!("Failed to open: {}", args.file.display()))?;

    // Check if this is a container file with subdatasets but no direct bands
    let band_count = dataset.raster_count();
    let subdatasets = get_subdatasets(&dataset);

    if band_count == 0 && !subdatasets.is_empty() {
        // This is a container file (e.g., Sentinel-2 ZIP, HDF, NetCDF)
        if args.info {
            // Show subdataset info
            print_container_info(&dataset, &subdatasets)?;
            return Ok(());
        }
        // Auto-switch to interactive mode for rendering
        eprintln!(
            "Detected container file with {} subdatasets. Switching to interactive mode...\n",
            subdatasets.len()
        );
        return run_interactive(&args);
    }

    if band_count == 0 {
        anyhow::bail!(
            "File has no raster bands. If this is a multi-dataset format, try: gis-view -i {:?}",
            args.file
        );
    }

    if args.info {
        print_metadata(&dataset)?;
        return Ok(());
    }

    let img = render_raster(&dataset, &args)?;
    display_image(&img, &args)?;

    Ok(())
}

fn run_interactive(args: &Args) -> Result<()> {
    let path = &args.file;
    let path_str = path.to_string_lossy();

    // Build the vsizip path if it's a zip file
    let gdal_path = if path_str.ends_with(".zip") {
        format!("/vsizip/{}", path_str)
    } else {
        path_str.to_string()
    };

    // Try to open and get subdatasets
    let dataset =
        Dataset::open(&gdal_path).with_context(|| format!("Failed to open: {}", gdal_path))?;

    let subdatasets = get_subdatasets(&dataset);

    let selected_path = if subdatasets.is_empty() {
        // No subdatasets, use the file directly
        gdal_path
    } else {
        // Let user select a subdataset
        println!("Available subdatasets:\n");
        let descriptions: Vec<String> = subdatasets.iter().map(|(_, desc)| desc.clone()).collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select subdataset")
            .items(&descriptions)
            .default(0)
            .interact()?;

        subdatasets[selection].0.clone()
    };

    // Open the selected dataset
    let dataset = Dataset::open(&selected_path)
        .with_context(|| format!("Failed to open: {}", selected_path))?;

    let band_count = dataset.raster_count();

    // Select band combination
    let bands = if band_count >= 3 {
        let band_options = vec![
            format!("True color (3,2,1) - Red, Green, Blue"),
            format!("False color (4,3,2) - NIR, Red, Green"),
            format!("Color infrared (4,2,1) - NIR, Green, Blue"),
            format!("Agriculture (4,3,1) - NIR, Red, Blue"),
            format!("Single band grayscale"),
            format!("Custom bands"),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Select band combination ({} bands available)",
                band_count
            ))
            .items(&band_options)
            .default(0)
            .interact()?;

        match selection {
            0 => vec![3, 2, 1],
            1 => vec![4, 3, 2],
            2 => vec![4, 2, 1],
            3 => vec![4, 3, 1],
            4 => {
                // Single band selection
                let band_items: Vec<String> =
                    (1..=band_count).map(|i| format!("Band {}", i)).collect();
                let band_sel = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select band for grayscale")
                    .items(&band_items)
                    .default(0)
                    .interact()?;
                vec![band_sel + 1, band_sel + 1, band_sel + 1]
            }
            5 => {
                // Custom bands
                let band_items: Vec<String> =
                    (1..=band_count).map(|i| format!("Band {}", i)).collect();

                let r = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select RED band")
                    .items(&band_items)
                    .default(0)
                    .interact()?
                    + 1;

                let g = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select GREEN band")
                    .items(&band_items)
                    .default(1.min(band_count - 1))
                    .interact()?
                    + 1;

                let b = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select BLUE band")
                    .items(&band_items)
                    .default(2.min(band_count - 1))
                    .interact()?
                    + 1;

                vec![r, g, b]
            }
            _ => vec![1, 2, 3],
        }
    } else {
        vec![1, 1, 1]
    };

    // Print the equivalent command
    println!("\nEquivalent command:");
    println!(
        "  gis-view \"{}\" --bands {},{},{}\n",
        selected_path, bands[0], bands[1], bands[2]
    );

    // Create modified args with selected bands
    let modified_args = Args {
        file: PathBuf::from(&selected_path),
        bands: Some(bands),
        width: args.width,
        height: args.height,
        info: false,
        stretch: args.stretch,
        max_res: args.max_res,
        interactive: false,
    };

    let img = render_raster(&dataset, &modified_args)?;
    display_image(&img, &modified_args)?;

    Ok(())
}

fn get_subdatasets(dataset: &Dataset) -> Vec<(String, String)> {
    let mut subdatasets = Vec::new();

    // GDAL stores subdatasets as metadata - use metadata_item to get individual entries
    let mut i = 1;
    loop {
        let name_key = format!("SUBDATASET_{}_NAME", i);
        let desc_key = format!("SUBDATASET_{}_DESC", i);

        let name = dataset.metadata_item(&name_key, "SUBDATASETS");
        let desc = dataset.metadata_item(&desc_key, "SUBDATASETS");

        match (name, desc) {
            (Some(n), Some(d)) => {
                subdatasets.push((n, d));
                i += 1;
            }
            _ => break,
        }
    }

    subdatasets
}

fn print_container_info(dataset: &Dataset, subdatasets: &[(String, String)]) -> Result<()> {
    let driver = dataset.driver().short_name();
    println!("Container format: {}", driver);
    println!("Subdatasets: {}\n", subdatasets.len());

    for (i, (name, desc)) in subdatasets.iter().enumerate() {
        println!("  [{}] {}", i + 1, desc);
        println!("      Path: {}", name);
    }

    println!("\nTo view a subdataset, use interactive mode:");
    println!("  gis-view -i <file>");
    println!("\nOr specify the subdataset path directly:");
    println!("  gis-view \"<subdataset_path>\" --bands 4,3,2");

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

/// Get terminal pixel dimensions (width, height) if available
fn get_terminal_pixel_size() -> Option<(usize, usize)> {
    // Try Kitty's method
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        if let Ok(output) = Command::new("kitten")
            .args(["icat", "--print-window-size"])
            .output()
        {
            if output.status.success() {
                let size_str = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = size_str.trim().split('x').collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse()) {
                        return Some((w, h));
                    }
                }
            }
        }
    }

    // Fallback: estimate from terminal character dimensions
    // Most terminals are roughly 80 columns, assume ~10 pixels per character
    if let Ok(output) = Command::new("tput").arg("cols").output() {
        if output.status.success() {
            if let Ok(cols) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<usize>()
            {
                // Estimate pixel width (most fonts are ~8-10 pixels wide)
                let estimated_width = cols * 9;
                let estimated_height = estimated_width * 3 / 4; // Assume 4:3 aspect
                return Some((estimated_width, estimated_height));
            }
        }
    }

    // Final fallback: assume reasonable defaults
    Some((1920, 1080))
}

fn render_raster(dataset: &Dataset, args: &Args) -> Result<DynamicImage> {
    let (src_width, src_height) = dataset.raster_size();
    let band_count = dataset.raster_count();

    // Get terminal size for smart scaling
    let terminal_size = get_terminal_pixel_size();

    // Calculate output dimensions (downsample if needed)
    let (out_width, out_height) = if args.max_res > 0 {
        let max_dim = args.max_res;
        let scale = (max_dim as f64 / src_width.max(src_height) as f64).min(1.0);
        let mut out_w = ((src_width as f64 * scale) as usize).max(1);
        let mut out_h = ((src_height as f64 * scale) as usize).max(1);

        // If output would be smaller than terminal, scale up to terminal size
        // This avoids display issues with some terminal graphics protocols
        if let Some((term_w, term_h)) = terminal_size {
            if out_w < term_w && out_h < term_h && src_width > term_w {
                // Image was downsampled below terminal size - use terminal size instead
                let term_scale = (term_w as f64 / src_width as f64)
                    .min(term_h as f64 / src_height as f64)
                    .min(1.0);
                out_w = ((src_width as f64 * term_scale) as usize).max(1);
                out_h = ((src_height as f64 * term_scale) as usize).max(1);
            }
        }
        (out_w, out_h)
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
    // Use viuer for all terminals - it handles Kitty, iTerm2, and falls back to blocks
    let config = Config {
        absolute_offset: false,
        use_kitty: true,
        use_iterm: true,
        ..Default::default()
    };
    viuer::print(img, &config).context("Failed to display image")?;

    Ok(())
}
