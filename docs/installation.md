# Installation

## Quick Start

### Option 1: Download Pre-built Binary (Linux)

Pre-built binaries are available for specific Ubuntu/GDAL version combinations.
**You must download the binary matching your system's GDAL version.**

1. Check your GDAL version:
   ```bash
   gdal-config --version
   ```

2. Download the matching binary from [Releases](https://github.com/ominiverdi/gis-viewer-cli/releases):

   | Your System | GDAL Version | Binary to Download |
   |-------------|--------------|-------------------|
   | Ubuntu 22.04 LTS | 3.4.x | `gis-view-x86_64-unknown-linux-gnu-ubuntu22.04-gdal3.4.tar.gz` |
   | Ubuntu 24.04 LTS | 3.8.x | `gis-view-x86_64-unknown-linux-gnu-ubuntu24.04-gdal3.8.tar.gz` |
   | Debian 12 (Bookworm) | 3.6.x | Try ubuntu22.04 build or build from source |
   | Other/Custom GDAL | varies | Build from source (recommended) |

3. Extract and install:
   ```bash
   tar -xzf gis-view-*.tar.gz
   sudo mv gis-view /usr/local/bin/
   ```

### Option 2: Build from Source (Recommended for Non-standard GDAL)

Building from source ensures the binary matches your exact GDAL version.

```bash
# Install GDAL first (see Prerequisites below)
cargo install --git https://github.com/ominiverdi/gis-viewer-cli
```

### Option 3: macOS / Windows

Download the appropriate binary from [Releases](https://github.com/ominiverdi/gis-viewer-cli/releases):
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

# Build
git clone https://github.com/ominiverdi/gis-viewer-cli.git
cd gis-viewer-cli
cargo build --release
sudo cp target/release/gis-view /usr/local/bin/
```

### Amazon Linux 2023

```bash
sudo dnf install -y gdal-devel gcc
# Then follow Rust installation above
```

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
1. Download the binary matching your GDAL version (check with `gdal-config --version`)
2. Build from source: `cargo install --git https://github.com/ominiverdi/gis-viewer-cli`

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
