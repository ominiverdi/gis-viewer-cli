# Installation

## Quick Start

### Option 1: Install from crates.io (Recommended)

Builds from source on your machine, automatically linking to your installed GDAL version. No version mismatch issues.

```bash
# Install GDAL first (see Prerequisites below)
cargo install gis-viewer-cli
```

### Option 2: Download Pre-built Binary

Pre-built binaries are available from [Releases](https://github.com/ominiverdi/gis-viewer-cli/releases).

**Linux:** binaries are dynamically linked to GDAL. Download the one matching your system:

| Your System | GDAL Version | Binary to Download |
|-------------|--------------|-------------------|
| Ubuntu 22.04 LTS | 3.4.x | `gis-view-...-ubuntu22.04-gdal3.4.tar.gz` |
| Ubuntu 24.04 LTS | 3.8.x | `gis-view-...-ubuntu24.04-gdal3.8.tar.gz` |
| Debian 12 (Bookworm) | 3.6.x | Try ubuntu22.04 build or use `cargo install` |
| Other/Custom GDAL | varies | Use `cargo install` (recommended) |

```bash
tar -xzf gis-view-*.tar.gz
sudo mv gis-view /usr/local/bin/
```

**macOS / Windows:**
- macOS Intel: `gis-view-x86_64-apple-darwin.tar.gz`
- macOS Apple Silicon: `gis-view-aarch64-apple-darwin.tar.gz`
- Windows: `gis-view-x86_64-pc-windows-msvc.zip`

---

## Prerequisites

### Required: GDAL Library

The CLI requires libgdal for raster format support. The binary dynamically links
against GDAL, so **your system's GDAL version must match the version used to
compile the binary**.

**macOS**
```bash
brew install gdal
```

**Ubuntu/Debian**
```bash
sudo apt update
sudo apt install libgdal-dev
```

**Fedora/RHEL**
```bash
sudo dnf install gdal-devel
```

**Arch Linux**
```bash
sudo pacman -S gdal
```

### Required for Building: Rust Toolchain

Only needed if building from source:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

---

## Building from Source

```bash
git clone https://github.com/ominiverdi/gis-viewer-cli.git
cd gis-viewer-cli
cargo build --release
```

Binary will be at `target/release/gis-view`.

### Install to PATH

```bash
cargo install --path .
```

Or manually:
```bash
cp target/release/gis-view ~/.local/bin/
```

---

## Verify Installation

```bash
# Check CLI works
gis-view --version

# Check GDAL binding
gis-view --info sample.tif
```

---

## Cloud Server Setup

### Ubuntu 22.04+ (AWS, GCP, Azure)

```bash
# Install dependencies
sudo apt update
sudo apt install -y libgdal-dev build-essential curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Install gis-view
cargo install gis-viewer-cli
```

### Amazon Linux 2023

```bash
sudo dnf install -y gdal-devel gcc
# Then follow Rust installation above
```

### SSH Usage

When viewing images on a remote server via SSH, terminal auto-detection may fail.
Use the `--protocol` flag to force the display protocol:

```bash
# From your Kitty terminal, SSH into the remote server:
ssh myserver "gis-view -p kitty /path/to/image.tif"

# Or in an interactive SSH session:
gis-view -p kitty image.tif --max-res 500
```

Available protocols:
- `kitty` - full pixel rendering (use when SSHed from Kitty terminal)
- `iterm` - full pixel rendering (use when SSHed from iTerm2)
- `blocks` - Unicode half-blocks (works everywhere)

### Docker

```dockerfile
FROM rust:1.83 as builder

RUN apt-get update && apt-get install -y libgdal-dev

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libgdal34t64 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/gis-view /usr/local/bin/
ENTRYPOINT ["gis-view"]
```

---

## Troubleshooting

### GDAL Version Mismatch Error

If you see an error like:
```
error while loading shared libraries: libgdal.so.34: cannot open shared object file
```

This means the binary was compiled against a different GDAL version than what's
installed on your system.

**Solution**: Either:
1. Install from source (recommended): `cargo install gis-viewer-cli`
2. Download the binary matching your GDAL version (check with `gdal-config --version`)

### Check GDAL Library Version

```bash
# Check installed GDAL version
gdal-config --version

# Check which shared library is available
ldconfig -p | grep libgdal

# Example output:
#   libgdal.so.34 (libc6,x86-64) => /usr/lib/x86_64-linux-gnu/libgdal.so.34
```

### `gdal-sys` build fails

GDAL development headers not found. Check:
```bash
gdal-config --version  # Should print version
pkg-config --libs gdal # Should print linker flags
```

If not found, install the development package:
```bash
# Ubuntu/Debian
sudo apt install libgdal-dev

# Fedora/RHEL
sudo dnf install gdal-devel
```

### Missing `libgdal.so` at runtime

```bash
# Find where GDAL is installed
ldconfig -p | grep gdal

# If not found, update library cache
sudo ldconfig

# Or add custom path
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

### Homebrew GDAL not detected (macOS)

```bash
export GDAL_HOME=$(brew --prefix gdal)
export PKG_CONFIG_PATH="$GDAL_HOME/lib/pkgconfig:$PKG_CONFIG_PATH"
cargo build --release
```
