use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Select};
use gdal::vector::LayerAccess;
use gdal::{Dataset, Metadata};
use image::{DynamicImage, Rgb, RgbImage};
use std::io::Write;
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

    /// Layer index for vector files (0-based, default: 0)
    #[arg(short = 'l', long)]
    layer: Option<usize>,

    /// Force display protocol: kitty, iterm, or blocks (auto-detected by default)
    #[arg(short = 'p', long)]
    protocol: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Check if file exists before trying to open with GDAL
    // Skip check for subdataset paths (contain ':' like SENTINEL2_L2A:/vsizip/...)
    let path_str_check = args.file.to_string_lossy();
    let is_subdataset = path_str_check.contains(":/vsi") || path_str_check.contains(":EPSG");
    if !is_subdataset && !args.file.exists() {
        anyhow::bail!("File not found: {}", args.file.display());
    }

    // NOTE: Don't handle interactive mode here - check file type first
    // Interactive mode is handled below based on whether it's raster or vector

    // Open the file with GDAL
    // For ZIP files, try direct open first (newer GDAL supports it),
    // then fall back to /vsizip/ prefix
    let path_str = args.file.to_string_lossy();
    let is_zip = path_str.ends_with(".zip") || path_str.ends_with(".ZIP");

    let dataset = if is_zip {
        Dataset::open(path_str.as_ref())
            .or_else(|_| Dataset::open(&format!("/vsizip/{}", path_str)))
            .with_context(|| {
                format!(
                    "Failed to open: {}\nThe file exists but GDAL cannot read it. The ZIP may be corrupted or incomplete.\nTry: python3 -c \"import zipfile; zipfile.ZipFile('{}')\"",
                    args.file.display(),
                    args.file.display()
                )
            })?
    } else {
        Dataset::open(path_str.as_ref()).with_context(|| {
            format!(
                "Failed to open: {}\nGDAL cannot read this file. It may be corrupted or in an unsupported format.",
                args.file.display()
            )
        })?
    };

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

    // Check if this is a vector file
    let layer_count = dataset.layer_count();
    if band_count == 0 && layer_count > 0 {
        // This is a vector file
        if args.info {
            print_vector_info(&dataset)?;
            return Ok(());
        }

        // Determine which layer to render
        let layer_idx = if let Some(idx) = args.layer {
            if idx >= layer_count {
                anyhow::bail!(
                    "Layer index {} out of range. File has {} layers (0-{})",
                    idx,
                    layer_count,
                    layer_count - 1
                );
            }
            idx
        } else if layer_count > 1 && args.interactive {
            // Interactive layer selection
            select_vector_layer(&dataset)?
        } else if layer_count > 1 {
            // Multiple layers, no selection - show info and prompt
            eprintln!(
                "Vector file has {} layers. Use --layer N or -i for interactive selection.\n",
                layer_count
            );
            print_vector_info(&dataset)?;
            return Ok(());
        } else {
            0 // Single layer, use it
        };

        // Render vector to image
        let img = render_vector(&dataset, layer_idx, &args)?;
        display_image(&img, &args)?;
        return Ok(());
    }

    if band_count == 0 {
        anyhow::bail!(
            "File has no raster bands or vector layers. If this is a multi-dataset format, try: gis-view -i {:?}",
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
        layer: None,
        protocol: args.protocol.clone(),
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

    // Calculate output dimensions (downsample if needed)
    let (out_width, out_height) = if args.max_res > 0 {
        let max_dim = args.max_res;
        let scale = (max_dim as f64 / src_width.max(src_height) as f64).min(1.0);
        let out_w = ((src_width as f64 * scale) as usize).max(1);
        let out_h = ((src_height as f64 * scale) as usize).max(1);
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

    // Normalize bands using shared min/max to preserve color relationships
    let stretch = args.stretch / 100.0;
    let (global_min, global_max) =
        compute_global_percentiles(&red, &green, &blue, nodata, stretch, 1.0 - stretch);
    let red_norm = normalize_with_range(&red, nodata, global_min, global_max);
    let green_norm = normalize_with_range(&green, nodata, global_min, global_max);
    let blue_norm = normalize_with_range(&blue, nodata, global_min, global_max);

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

/// Compute global min/max percentiles across all three bands.
/// Using the same range for all bands preserves color relationships.
fn compute_global_percentiles(
    r: &[f64],
    g: &[f64],
    b: &[f64],
    nodata: Option<f64>,
    low_pct: f64,
    high_pct: f64,
) -> (f64, f64) {
    let is_valid = |v: &f64| -> bool {
        if let Some(nd) = nodata {
            (v - nd).abs() > f64::EPSILON && v.is_finite()
        } else {
            v.is_finite()
        }
    };

    let mut all_valid: Vec<f64> = r
        .iter()
        .chain(g.iter())
        .chain(b.iter())
        .filter(|v| is_valid(v))
        .copied()
        .collect();

    if all_valid.is_empty() {
        return (0.0, 1.0);
    }

    all_valid.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let low_idx = ((all_valid.len() as f64 * low_pct) as usize).min(all_valid.len() - 1);
    let high_idx = ((all_valid.len() as f64 * high_pct) as usize).min(all_valid.len() - 1);

    (all_valid[low_idx], all_valid[high_idx])
}

/// Normalize values to 0-255 using a pre-computed min/max range.
fn normalize_with_range(
    values: &[f64],
    nodata: Option<f64>,
    min_val: f64,
    max_val: f64,
) -> Vec<u8> {
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

fn display_image(img: &DynamicImage, args: &Args) -> Result<()> {
    match args.protocol.as_deref() {
        Some("kitty") => display_kitty_direct(img),
        Some("iterm") => {
            let config = Config {
                absolute_offset: false,
                use_kitty: false,
                use_iterm: true,
                ..Default::default()
            };
            viuer::print(img, &config).context("Failed to display image")?;
            Ok(())
        }
        Some("blocks") => {
            let config = Config {
                absolute_offset: false,
                use_kitty: false,
                use_iterm: false,
                ..Default::default()
            };
            viuer::print(img, &config).context("Failed to display image")?;
            Ok(())
        }
        Some(other) => anyhow::bail!("Unknown protocol '{}'. Use: kitty, iterm, or blocks", other),
        None => {
            // Auto-detect
            let config = Config {
                absolute_offset: false,
                use_kitty: true,
                use_iterm: true,
                ..Default::default()
            };
            viuer::print(img, &config).context("Failed to display image")?;
            Ok(())
        }
    }
}

/// Send image directly using Kitty graphics protocol escape sequences.
/// Bypasses viuer's terminal detection which fails over SSH.
fn display_kitty_direct(img: &DynamicImage) -> Result<()> {
    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let raw = rgba.as_raw();

    let encoded = general_purpose::STANDARD.encode(raw);
    let mut stdout = std::io::stdout().lock();

    // Send in chunks (Kitty protocol limit is 4096 bytes per chunk)
    let chunk_size = 4096;
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        if i == 0 {
            // First chunk: include image metadata
            write!(
                stdout,
                "\x1b_Ga=T,f=32,s={},v={},m={};{}\x1b\\",
                width,
                height,
                if is_last { 0 } else { 1 },
                chunk
            )?;
        } else {
            // Continuation chunks
            write!(
                stdout,
                "\x1b_Gm={};{}\x1b\\",
                if is_last { 0 } else { 1 },
                chunk
            )?;
        }
    }

    writeln!(stdout)?;
    stdout.flush()?;

    Ok(())
}

fn print_vector_info(dataset: &Dataset) -> Result<()> {
    let layer_count = dataset.layer_count();
    let driver = dataset.driver().short_name();

    println!("Vector file: {}", driver);
    println!("Layers: {}", layer_count);
    println!();

    for i in 0..layer_count {
        let layer = dataset.layer(i)?;
        let name = layer.name();
        let feature_count = layer.feature_count();
        let geom_type = layer
            .defn()
            .geom_fields()
            .next()
            .map(|f| format!("{:?}", f.field_type()))
            .unwrap_or_else(|| "Unknown".to_string());

        println!("Layer [{}]: {}", i, name);
        println!("  Features: {}", feature_count);
        println!("  Geometry: {}", geom_type);

        // Get extent
        if let Ok(extent) = layer.get_extent() {
            println!(
                "  Extent: ({:.6}, {:.6}) - ({:.6}, {:.6})",
                extent.MinX, extent.MinY, extent.MaxX, extent.MaxY
            );
        }

        // Get spatial reference
        if let Some(srs) = layer.spatial_ref() {
            if let Some(name) = srs.name() {
                println!("  CRS: {}", name);
            }
        }

        // List attribute fields
        let defn = layer.defn();
        let fields: Vec<String> = defn.fields().map(|f| f.name()).collect();
        if !fields.is_empty() {
            println!("  Fields: {}", fields.join(", "));
        }
        println!();
    }

    Ok(())
}

fn select_vector_layer(dataset: &Dataset) -> Result<usize> {
    let layer_count = dataset.layer_count();
    let mut layer_items = Vec::new();

    for i in 0..layer_count {
        let layer = dataset.layer(i)?;
        let name = layer.name();
        let feature_count = layer.feature_count();
        let geom_type = layer
            .defn()
            .geom_fields()
            .next()
            .map(|f| format!("{:?}", f.field_type()))
            .unwrap_or_else(|| "Unknown".to_string());
        layer_items.push(format!(
            "{} ({} features, {})",
            name, feature_count, geom_type
        ));
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select layer to display")
        .items(&layer_items)
        .default(0)
        .interact()?;

    Ok(selection)
}

fn render_vector(dataset: &Dataset, layer_idx: usize, args: &Args) -> Result<DynamicImage> {
    // Get terminal size for output dimensions
    let terminal_size = get_terminal_pixel_size();
    let (img_width, _) = terminal_size.unwrap_or((1024, 768));

    // Use specified max_res or terminal size
    let max_dim = if args.max_res > 0 && args.max_res < img_width {
        args.max_res
    } else {
        img_width
    };

    // Get the selected layer
    let layer = dataset.layer(layer_idx)?;
    let extent = layer
        .get_extent()
        .context("Cannot get layer extent for rendering")?;

    // Calculate output dimensions maintaining aspect ratio
    let extent_width = extent.MaxX - extent.MinX;
    let extent_height = extent.MaxY - extent.MinY;
    let aspect = extent_width / extent_height;

    let (out_width, out_height) = if aspect > 1.0 {
        (max_dim, (max_dim as f64 / aspect) as usize)
    } else {
        ((max_dim as f64 * aspect) as usize, max_dim)
    };

    eprintln!(
        "Rendering {} features to {}x{} image",
        layer.feature_count(),
        out_width,
        out_height
    );

    // Create image with dark background
    let mut img = RgbImage::from_pixel(out_width as u32, out_height as u32, Rgb([20, 20, 30]));

    // Transform coordinates to pixel space
    let scale_x = out_width as f64 / extent_width;
    let scale_y = out_height as f64 / extent_height;

    // Draw each feature
    let mut layer = dataset.layer(layer_idx)?; // Re-get layer to reset iterator
    for feature in layer.features() {
        if let Some(geom) = feature.geometry() {
            draw_geometry(&mut img, &geom, &extent, scale_x, scale_y, out_height);
        }
    }

    Ok(DynamicImage::ImageRgb8(img))
}

fn draw_geometry(
    img: &mut RgbImage,
    geom: &gdal::vector::Geometry,
    extent: &gdal::vector::Envelope,
    scale_x: f64,
    scale_y: f64,
    img_height: usize,
) {
    let color = Rgb([100, 200, 255]); // Cyan-ish color for vectors

    // Get geometry type name
    let geom_type = geom.geometry_type();

    match geom_type {
        // Points
        gdal::vector::OGRwkbGeometryType::wkbPoint
        | gdal::vector::OGRwkbGeometryType::wkbPoint25D
        | gdal::vector::OGRwkbGeometryType::wkbPointM
        | gdal::vector::OGRwkbGeometryType::wkbPointZM => {
            let (x, y, _) = geom.get_point(0);
            let px = ((x - extent.MinX) * scale_x) as i32;
            let py = (img_height as f64 - (y - extent.MinY) * scale_y) as i32;
            draw_point(img, px, py, color);
        }
        // LineStrings
        gdal::vector::OGRwkbGeometryType::wkbLineString
        | gdal::vector::OGRwkbGeometryType::wkbLineString25D => {
            draw_linestring(img, geom, extent, scale_x, scale_y, img_height, color);
        }
        // Polygons
        gdal::vector::OGRwkbGeometryType::wkbPolygon
        | gdal::vector::OGRwkbGeometryType::wkbPolygon25D => {
            // Draw exterior ring
            let ring = geom.get_geometry(0);
            draw_linestring(img, &ring, extent, scale_x, scale_y, img_height, color);
        }
        // Multi geometries - recurse
        gdal::vector::OGRwkbGeometryType::wkbMultiPoint
        | gdal::vector::OGRwkbGeometryType::wkbMultiLineString
        | gdal::vector::OGRwkbGeometryType::wkbMultiPolygon
        | gdal::vector::OGRwkbGeometryType::wkbGeometryCollection => {
            for i in 0..geom.geometry_count() {
                let sub_geom = geom.get_geometry(i);
                draw_geometry(img, &sub_geom, extent, scale_x, scale_y, img_height);
            }
        }
        _ => {
            // Try to handle as collection
            for i in 0..geom.geometry_count() {
                let sub_geom = geom.get_geometry(i);
                draw_geometry(img, &sub_geom, extent, scale_x, scale_y, img_height);
            }
        }
    }
}

fn draw_point(img: &mut RgbImage, x: i32, y: i32, color: Rgb<u8>) {
    // Draw a small cross for points
    for dx in -1..=1 {
        for dy in -1..=1 {
            let px = x + dx;
            let py = y + dy;
            if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

fn draw_linestring(
    img: &mut RgbImage,
    geom: &gdal::vector::Geometry,
    extent: &gdal::vector::Envelope,
    scale_x: f64,
    scale_y: f64,
    img_height: usize,
    color: Rgb<u8>,
) {
    let point_count = geom.point_count();
    if point_count < 2 {
        return;
    }

    for i in 0..(point_count - 1) as i32 {
        let (x1, y1, _) = geom.get_point(i);
        let (x2, y2, _) = geom.get_point(i + 1);
        let px1 = ((x1 - extent.MinX) * scale_x) as i32;
        let py1 = (img_height as f64 - (y1 - extent.MinY) * scale_y) as i32;
        let px2 = ((x2 - extent.MinX) * scale_x) as i32;
        let py2 = (img_height as f64 - (y2 - extent.MinY) * scale_y) as i32;
        draw_line(img, px1, py1, px2, py2, color);
    }
}

fn draw_line(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb<u8>) {
    // Bresenham's line algorithm
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
            img.put_pixel(x as u32, y as u32, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
