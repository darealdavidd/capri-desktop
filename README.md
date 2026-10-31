# Capri Desktop Mate (Rust / Wayland Edition)

> A high-performance, Wayland-native Rust rewrite of [Capri Desktop Mate](https://github.com/mochiiwuff/capri-desktop) by **mochiiwuff**, purpose-built for Linux and Tiling Window Managers (Hyprland, Sway, River, etc.) with **zero X11 dependencies**.

> [!WARNING]
> **Project Status: Work in Progress / Buggy & Unfinished**
> This is an experimental, early-stage native Rust & Wayland port. It is currently **unfinished and buggy** — there may be rendering glitches, compositor-specific quirks (e.g. Hyprland vs Sway vs River), or unpolished behaviors. I'm still learning :D
>
> 🤝 **Contributions Welcome**: Anyone interested in helping test, polish, or improve this port is very warmly encouraged to contribute in a professional and constructive way! Pull requests, bug reports, and architectural feedback are welcome.

---

## ⚡ Highlights

- **Pure Wayland Native (`wlr-layer-shell`)**: Runs on `Layer::Overlay` anchored to all screen edges with `exclusive_zone = -1`. Never gets tiled, never messes up your tiling layouts, and persists seamlessly across workspaces.
- **Dynamic Click-Through**: Clicks pass directly through the transparent overlay to your underlying windows (terminals, editors, browsers) except when clicking directly on Capri, her dialogue bubble, or her biochip dock (`wl_surface_set_input_region`).
- **Zero Electron Overhead**: Replaced the Node.js / Electron stack with a lean, compiled Rust binary that sips minimal RAM and CPU at 60 FPS.
- **Zero X11 / Xlib**: Absolutely no legacy X11 dependencies.
- **Integrated Terminal & Workflow Hooks**:
  - Direct integration with developer tools and terminal workflows (Claude Code, Codex, shell scripts).
  - Capri reacts in real-time when commands execute, tools run, builds succeed, or errors happen.
- **Synthesized Audio Engine**: Direct in-memory WAV audio synthesis for all 14 sound cues (teleports, alerts, fanfare, landing, blips) streamed through `pw-play` (PipeWire), `paplay` (PulseAudio), or `aplay` (ALSA).
- **All 25 Original Exposure Sheet Acts & FX**:
  - Poses: Stand, Sit, Sleep, Peek, Talk, Curious, Victory, Hop, Fall, Zap, Point, Split, and more.
  - Visual Effects: Teleport beam, particle bursts, shockwave rings, lightning bolts, anime cut-ins, and cyberpunk typewriter transmissions.
  - Multilingual support: English, French, Spanish, Portuguese, and Russian.

---

## 📦 System Requirements

### Arch Linux / CachyOS / Manjaro / EndeavourOS

Install required system libraries via `pacman`:

```bash
sudo pacman -S --needed gtk3 gtk-layer-shell pipewire base-devel rust
```

*(PipeWire or PulseAudio/ALSA will handle audio playback via `pw-play`, `paplay`, or `aplay` automatically).*

### Debian / Ubuntu (22.04+)

```bash
sudo apt update
sudo apt install -y libgtk-3-dev libgtk-layer-shell-dev libasound2-dev pkg-config build-essential
```

### Fedora (39+)

```bash
sudo dnf install -y gtk3-devel gtk-layer-shell-devel pkgconf-pkg-config
```

---

## 🚀 Building & Running

### 1. Build the binary

```bash
# Debug build
cargo build

# Optimized release build
cargo build --release
```

### 2. Launch Capri

```bash
# Run release build
./target/release/capri-desktop

# Or via cargo
cargo run --release
```

---

## 🔧 Terminal & Tool Hooks

Capri includes a local async event bridge on `127.0.0.1:47823` that listens for events from terminal tools, build runners, and workflows.

### Automatic Hook Installation

Capri can auto-configure your installed tools:

```bash
# For Claude Code (~/.claude/settings.json)
capri-desktop install-hooks claude

# For Codex CLI
capri-desktop install-hooks codex
```

To remove configured hooks:

```bash
capri-desktop uninstall-hooks
```

### Manual CLI Hooking

You can forward events directly from shell scripts, Git hooks, or Makefile steps:

```bash
capri-desktop hook --event thinking --message "Compiling crate..."
capri-desktop hook --event tool --action "cargo test"
capri-desktop hook --event done --message "All tests passed!"
capri-desktop hook --event error --message "Syntax error in main.rs"
```

Available events: `prompt`, `thinking`, `tool`, `edit`, `done`, `error`, `wake`, `sleep`.

---

## 🎮 Controls & Interactions

- **Left Click on Capri**: Drag to move Capri around your screen or toss her onto the floor.
- **Right Click on Capri**: Opens context menu:
  - ⚡ *Teleport*: Random jump across your screen with beam fx.
  - 👀 *Look around*: Triggers active gaze tracking.
  - 🎬 *Replay Demo*: Performs the signature victory sequence.
  - 💤 *Sleep / Wake*: Toggle sleep mode with sleep mask.
  - 📦 *Stow / Dock*: Minimizes Capri into the biochip dock in the bottom-left corner.
  - 🚪 *Exit*: Clean shutdown.
- **Left Click on Biochip (bottom-left)**: Wakes Capri from her dock.
- **Left Click on Speech Panel**: Dismisses the current transmission immediately.

---

## ⚙️ Configuration

Configuration is automatically stored at `~/.config/capri/config.json`:

```json
{
  "language": "en",
  "assistant": "claude",
  "name": "Capri",
  "muted": false,
  "volume": 0.3,
  "scale": 1.0,
  "teleport": true,
  "autonomy": true,
  "dnd": false
}
```

---

## 🤝 Contributing & Known Issues

This Rust port is in active initial development and is currently **unfinished and buggy**:
- Testing across different Wayland compositors (Hyprland, Sway, River, Cosmic, KDE, GNOME) is ongoing.
- Transparency and input pass-through behavior can vary depending on compositor layer-shell implementation.
- Audio synthesis and assistant event hooks may need tuning.

If you would like to help test, fix bugs, optimize rendering, or improve Wayland compositor compatibility, **contributions and feedback submitted in a professional, constructive manner are very welcome!** Feel free to open an issue or pull request.

---

## 📜 License & Credits

- **Original Concept & Artwork**: Copyright (c) 2026 **mochiiwuff** ([GitHub Repository](https://github.com/mochiiwuff/capri-desktop)).
- **Code License**: Licensed under the **PolyForm Noncommercial License 1.0.0**. Free for noncommercial personal use and modification. See [LICENSE](LICENSE) for details.
- Character artwork and sprites are the intellectual property of their respective rights holders.
