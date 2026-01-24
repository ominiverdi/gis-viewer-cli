# Terminal Graphics Protocols

## Protocol Comparison

| Protocol | Origin | Color Depth | Resolution | Adoption |
|----------|--------|-------------|------------|----------|
| Sixel | DEC (1980s) | 256 colors | Pixel-level | Moderate |
| Kitty | Kitty terminal | 24-bit | Pixel-level | Growing |
| iTerm2 | iTerm2 | 24-bit | Pixel-level | macOS-centric |
| Unicode Blocks | Universal | Terminal colors | 2 pixels/char | Universal |

## Sixel

Legacy bitmap format from Digital Equipment Corporation.

**Pros:**
- Widest support among graphics protocols
- Works in xterm (with compile flag), mlterm, foot

**Cons:**
- Limited to 256 colors
- Larger data size than modern protocols

**Detection:** Query terminal with `\e[c` and check for `;4;` in response.

## Kitty Graphics Protocol

Modern protocol designed for flexibility and performance.

**Pros:**
- 24-bit color
- Efficient binary encoding
- Animation support
- Terminal-agnostic design

**Cons:**
- Requires modern terminal

**Supported terminals:** Kitty, WezTerm, Konsole, Ghostty

**Detection:** Check `TERM=xterm-kitty` or query capabilities.

## iTerm2 Inline Images

Proprietary protocol for iTerm2 on macOS.

**Pros:**
- Native macOS integration
- 24-bit color

**Cons:**
- macOS only
- iTerm2 specific

**Detection:** Check `TERM_PROGRAM=iTerm.app` or `LC_TERMINAL=iTerm2`.

## Unicode Block Characters

Fallback using Unicode half/quarter blocks with ANSI colors.

**Characters used:**
- `▀` (upper half block) - 2 vertical pixels per cell
- `▄` (lower half block)
- Quarter blocks for higher resolution (4 pixels per cell)

**Pros:**
- Works everywhere
- No special terminal support needed

**Cons:**
- Much lower resolution
- Quality depends on font and terminal color support

## Protocol Selection Strategy

```
1. Check for Kitty protocol support
2. Check for iTerm2 protocol support
3. Check for Sixel support
4. Fall back to Unicode blocks
```

The `viuer` crate handles this detection automatically.
