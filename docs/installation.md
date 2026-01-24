# Installation

## Prerequisites

### Required: GDAL Library

The CLI requires libgdal for raster format support.

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

### Required: Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Building from Source

```bash
git clone https://github.com/user/gis-viewer-cli.git
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

## Verify Installation

```bash
# Check CLI works
gis-view --version

# Check GDAL binding
gis-view --info sample.tif
```

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
git clone <repo>
cd gis-viewer-cli
cargo build --release
sudo cp target/release/gis-view /usr/local/bin/
```

### Amazon Linux 2

```bash
sudo yum install -y gdal-devel gcc
# Then follow Rust installation above
```

### Docker

```dockerfile
FROM rust:1.75 as builder

RUN apt-get update && apt-get install -y libgdal-dev

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libgdal32 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/gis-view /usr/local/bin/
ENTRYPOINT ["gis-view"]
```

## Troubleshooting

### `gdal-sys` build fails

GDAL not found. Check:
```bash
gdal-config --version  # Should print version
pkg-config --libs gdal # Should print linker flags
```

### Missing `libgdal.so`

```bash
# Find where GDAL is installed
ldconfig -p | grep gdal

# If not found, add to library path
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

### Homebrew GDAL not detected (macOS)

```bash
export GDAL_HOME=$(brew --prefix gdal)
export PKG_CONFIG_PATH="$GDAL_HOME/lib/pkgconfig:$PKG_CONFIG_PATH"
cargo build --release
```
