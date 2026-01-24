# SSH Workflow

## How Terminal Graphics Work Over SSH

Terminal graphics protocols tunnel through SSH transparently. The key requirement is that **your local terminal** (the one you're typing in) supports the protocol.

```
┌─────────────────┐     SSH      ┌─────────────────┐
│  Local Machine  │◄────────────►│  Remote Server  │
│  (Your laptop)  │              │  (Cloud VM)     │
├─────────────────┤              ├─────────────────┤
│  Kitty/iTerm2   │◄─ graphics ──│  gis-viewer-cli │
│  WezTerm        │   protocol   │  outputs escape │
│                 │   escapes    │  sequences      │
└─────────────────┘              └─────────────────┘
```

The remote server outputs escape sequences. Your local terminal interprets them.

## Local Terminal Setup

### macOS

**Option 1: iTerm2 (Recommended)**
```bash
brew install --cask iterm2
```
Supports iTerm2 inline images natively.

**Option 2: Kitty**
```bash
brew install --cask kitty
```
Full Kitty protocol support.

### Linux

**Kitty**
```bash
# Ubuntu/Debian
sudo apt install kitty

# Arch
sudo pacman -S kitty
```

**WezTerm**
```bash
# Flatpak (universal)
flatpak install flathub org.wezfurlong.wezterm
```

### Windows

**WezTerm**
Download from: https://wezfurlong.org/wezterm/

**Windows Terminal + WSL**
Limited support; Unicode blocks only.

## SSH Configuration

Standard SSH works. No special configuration needed:

```bash
ssh user@remote-server
gis-view /path/to/raster.tif
```

### Preserving Terminal Type

Ensure `TERM` environment variable is forwarded:

```bash
# In ~/.ssh/config
Host myserver
    HostName 1.2.3.4
    SendEnv TERM TERM_PROGRAM LC_TERMINAL
```

On server, ensure `/etc/ssh/sshd_config` has:
```
AcceptEnv TERM TERM_PROGRAM LC_TERMINAL
```

## Protocol Detection Over SSH

The CLI detects protocols by checking environment variables and querying terminal capabilities:

| Variable | Protocol |
|----------|----------|
| `TERM=xterm-kitty` | Kitty |
| `TERM_PROGRAM=iTerm.app` | iTerm2 |
| `LC_TERMINAL=iTerm2` | iTerm2 |

These are typically preserved over SSH.

## Troubleshooting

### Images not displaying (Unicode blocks shown)

1. **Check local terminal**: Ensure Kitty/iTerm2/WezTerm is being used
2. **Check TERM variable**: `echo $TERM` on remote should show capability
3. **Force protocol**: `gis-view --protocol kitty raster.tif`

### Garbled output

Terminal doesn't support the detected protocol. Force fallback:
```bash
gis-view --protocol blocks raster.tif
```

### Slow rendering

Large images over slow connections. Use smaller output:
```bash
gis-view --width 40 raster.tif
```

## tmux/screen Considerations

Terminal multiplexers may interfere with graphics protocols.

**tmux**: Partial Sixel support in recent versions. Kitty protocol not supported.

**Workaround**: Connect directly without multiplexer for image viewing, or use Unicode fallback.

## Performance Tips

1. **Use overviews**: COG files with internal overviews load faster
2. **Limit resolution**: `--width 80` for quick previews
3. **Local processing**: For large datasets, process remotely but view locally:
   ```bash
   ssh server "gis-view raster.tif --output /tmp/preview.png"
   scp server:/tmp/preview.png - | kitty icat
   ```
