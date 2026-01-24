# Feasibility Analysis

## Executive Summary

**Verdict: Feasible** with well-understood constraints.

Displaying GIS raster images in terminal environments is achievable using modern terminal graphics protocols. Quality depends on terminal capabilities, with graceful degradation to Unicode characters for basic terminals.

## Research Findings

### Terminal Image Display Technologies

| Technology | Quality | Support |
|------------|---------|---------|
| Kitty Graphics Protocol | Excellent (24-bit, pixel-level) | Kitty, WezTerm, Konsole, Ghostty |
| iTerm2 Inline Images | Excellent (24-bit) | iTerm2 (macOS) |
| Sixel | Good (256 colors) | xterm, mlterm, foot, many others |
| Unicode Blocks | Acceptable | Universal |

### GIS Tooling Landscape

**GDAL** remains the industry standard for raster data handling. The Rust `gdal` crate provides solid bindings but requires system library installation.

Pure-Rust alternatives exist (`tiff` crate) but lack:
- Full GeoTIFF metadata support
- Projection handling
- Format variety (COG optimizations, compression variants)

### Existing Tools Reviewed

| Tool | Purpose | Limitation |
|------|---------|------------|
| `chafa` | General image viewer | No GIS awareness |
| `viu` | Terminal image display | No GIS support |
| `timg` | Terminal graphics | No GIS support |
| `gdalinfo` | GIS metadata | No visual display |
| `leafmap` | Python GIS visualization | Opens browser, not terminal |

**Gap identified**: No terminal-native tool combines GIS format reading with terminal graphics rendering.

## Constraints

### Resolution Limits

Terminal resolution is fundamentally limited:
- Typical terminal: 80-200 columns, 24-50 rows
- With graphics protocols: ~1000-4000 pixels wide (varies by terminal)
- High-res satellite imagery must be downsampled

This is acceptable for **preview** use case, not detailed analysis.

### SSH Considerations

Graphics protocols work over SSH when:
1. Local terminal supports the protocol
2. Environment variables are forwarded
3. No interfering multiplexer (tmux has limited support)

### System Dependencies

GDAL dependency is non-trivial:
- 50MB+ library
- Platform-specific installation
- Potential version conflicts

Tradeoff: Full compatibility vs. easy distribution.

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| GDAL build issues | Medium | Clear documentation, Docker option |
| Protocol not detected | Low | Manual protocol override flag |
| Large file performance | Medium | Overview support, size limits |
| SSH graphics fail | Low | Fallback to Unicode blocks |

## Recommendation

Proceed with implementation using:
- GDAL for raster reading (full compatibility)
- `viuer` for terminal rendering (protocol auto-detection)
- Clear fallback path for basic terminals

The tool fills a genuine gap in the GIS/terminal tooling ecosystem.

## References

- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [Sixel Graphics](https://en.wikipedia.org/wiki/Sixel)
- [GDAL Rust Bindings](https://github.com/georust/gdal)
- [viuer crate](https://github.com/atanunq/viuer)
- [Are We Sixel Yet?](https://www.arewesixelyet.com/)
