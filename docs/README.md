# GIS Viewer CLI Documentation

A Rust CLI tool for viewing high-resolution GIS raster images in terminal environments, optimized for headless/SSH workflows.

## Documentation Index

### Architecture & Design

- [Project Overview](./overview.md) - Goals, scope, and high-level architecture
- [Terminal Graphics Protocols](./terminal-graphics.md) - Sixel, Kitty, iTerm2 protocol comparison
- [GIS Data Handling](./gis-data-handling.md) - Raster formats, GDAL vs pure-Rust approaches

### Implementation

- [Rust Crate Selection](./rust-crates.md) - Dependencies and tradeoffs
- [Rendering Pipeline](./rendering-pipeline.md) - From GeoTIFF to terminal pixels

### Deployment

- [SSH Workflow](./ssh-workflow.md) - Using the tool over remote connections
- [Installation](./installation.md) - Build requirements and setup

### Reference

- [Feasibility Analysis](./feasibility.md) - Original research and constraints
