# NukeAV 🚀

[![Latest Release](https://img.shields.io/github/v/release/AnarchByte0/NukeAV?style=for-the-badge&color=blue)](https://github.com/AnarchByte0/NukeAV/releases/latest)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS-lightgrey?style=for-the-badge)](#installation-)
[![Ko-Fi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/anarchbyte)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/anarchbyte)

**Native AV1, VP8, and VP9 importer & exporter plugin for Adobe Premiere Pro — written in Rust and powered by statically linked FFmpeg.**

Drop `.mkv`, `.webm`, `.mp4`, or `.mov` files with AV1/VP8/VP9 straight onto your Premiere Pro timeline without transcoding, and export back out with hardware acceleration (NVENC, AMF, QSV) or high-quality software encoders.

---

## Installation 📦

No compilation or external FFmpeg install is required — grab the pre-built plugin from the **[Releases page](https://github.com/AnarchByte0/NukeAV/releases/latest)**.

### Windows
1. Download **`NukeAV-*-Windows.prm`** from the latest release (you can rename it to `NukeAV.prm`).
2. Copy it into Adobe's shared MediaCore plugins folder:
   ```text
   C:\Program Files\Adobe\Common\Plug-ins\7.0\MediaCore\
   ```
3. Restart **Adobe Premiere Pro**.

### macOS (Apple Silicon & Intel)
1. Download **`NukeAV-*-macOS.zip`** from the latest release and unzip it to get **`NukeAV.bundle`**.
2. Move `NukeAV.bundle` into Adobe's shared MediaCore plugins folder:
   ```text
   /Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/
   ```
3. If macOS Gatekeeper warns about an unidentified developer on first launch, remove the quarantine attribute:
   ```bash
   sudo xattr -rd com.apple.quarantine "/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/NukeAV.bundle"
   ```
4. Restart **Adobe Premiere Pro**.

> [!TIP]
> **Where to find it in Premiere Pro:**
> - **Importing:** Just drag and drop `.mkv`, `.webm`, `.mp4`, or `.mov` files directly into your Project panel.
> - **Exporting:** In the Export / Media Encoder **Format** dropdown, choose **AV1 (Nuke)**, **VP9 (Nuke)**, or **VP8 (Nuke)**.

---

## Supported Formats & Hardware Acceleration 🎯

| Feature | Import (Decode) | Export (Encode) |
| :--- | :--- | :--- |
| **Video Codecs** | **AV1** (`dav1d` / `libaom`), **VP9**, **VP8** | **AV1**, **VP9**, **VP8** |
| **Containers** | `.mkv`, `.webm`, `.mp4`, `.mov` | `.mp4`, `.mkv`, `.webm`, `.3gp` |
| **Audio Codecs** | Decoded via FFmpeg (`swresample`) | **AAC**, **Opus**, **PCM** (`pcm_s16le`) |
| **GPU Acceleration** | D3D11VA & CUDA (Windows), Auto-fallback | **NVENC** (NVIDIA), **AMF** (AMD), **QSV** (Intel) |
| **Color & Bit Depth** | 8-bit, 10-bit+, HDR (32-bit float / 16-bit / 8-bit BGRA) | 8-bit 4:2:0, 10-bit 4:2:0 (HDR), RGBA |

---

## Key Features ✨

* **Smooth Timeline Scrubbing & A/V Sync:** Asynchronous video importer with dedicated audio demuxing so video and audio never interfere or drift out of sync when seeking.
* **Smart RAM Frame Cache:** Automatically detects your system's physical RAM (`GlobalMemoryStatusEx` on Windows, `sysctl` on macOS) and scales the frame cache from **4 to 32 frames**.
* **Syncs with Premiere Pro Preferences:**
  * **Audio Cache Bypass:** Reads Premiere's *"Save media cache files next to originals"* setting — skips generating `.cfa`/`.pek` clutter next to your source files when enabled, or uses centralized caching when disabled (can also be overridden in `%APPDATA%\NukeAV\config.txt` / `~/Library/Application Support/NukeAV/config.txt`).
  * **GPU Preference Sync:** Respects Premiere Pro's global GPU acceleration toggle.
  * **Sequence Auto-Fill:** Pre-fills default export width, height, and FPS from your most recent sequence.
* **OBS-Style Exporter Controls:**
  * **Rate Control:** CBR, CQP, VBR, and VBR with Target Quality — UI labels and fields update dynamically.
  * **Tuning & Quality:** Presets (`P1`–`P7`), Tuning (`High Quality`, `Low Latency`, `Ultra Low Latency`), Multipass (Single Pass, 2-Pass Quarter/Full Resolution), Look-Ahead, Adaptive Quantization, B-Frames, Film Grain synthesis, and Custom FFmpeg flags (`key=val`).
  * **Multiplexer Control:** Switch between `MP4`, `3GPP`, or `None` (raw video stream with automatic audio silencing).

---

## Building from Source 🛠️

If you want to compile NukeAV yourself:

### Prerequisites
* **Windows:** Windows 10/11, [Rust](https://rustup.rs/) (Edition 2024), Visual Studio Build Tools (MSVC C++), Git, CMake, and LLVM (`libclang`).
* **macOS:** macOS 11.0+, [Rust](https://rustup.rs/) (Edition 2024), Xcode Command Line Tools, and Homebrew dependencies:
  ```bash
  brew install cmake ninja nasm
  ```

### Adobe Premiere Pro C++ SDK
Because Adobe's SDK headers are proprietary, they are not included in this repository:
1. Download the **Premiere Pro C++ SDK** (26.0+) from the Adobe Developer Console.
2. Place the archive or extracted SDK folder into `sdk/`:
   * Windows: `sdk/Premiere Pro 26.0 C++ SDK Windows.zip`
   * macOS: `sdk/Premiere Pro 26.0 C++ SDK Mac.zip`
   * Or point `ADOBE_SDK_DIR` directly to the SDK's `Examples/Headers` directory.

### Compile & Package
```bash
cargo build --release
```
On Windows, you can also build and package `output/NukeAV.prm` in one step:
```bash
cargo run --release --bin package
```

> [!NOTE]
> **First build time:** The first release build automatically clones `vcpkg` and compiles FFmpeg along with `aom`, `dav1d`, and `libvpx` statically (~15–40 minutes depending on your CPU). Subsequent builds take just a few seconds.
>
> **Fast IDE / type-checking (`cargo check`):** Set `SKIP_FFMPEG_BUILD=1` to skip compiling FFmpeg when only checking Rust code or working in an IDE.

<details>
<summary><b>Architecture Overview</b> (for contributors)</summary>

* **`src/lib.rs`** — Main plugin entry points (`xImportEntry` and `xSDKExport`) exported for Premiere Pro.
* **`src/ffi/`** — Raw FFI bindings (`adobe.rs` generated via `bindgen` from `wrapper.hpp`, plus `ffmpeg.rs`).
* **`src/plugin/importer/`** — Importer selectors, format registration, metadata cache, async frame worker, and independent audio/video demuxers.
* **`src/plugin/exporter/`** — Exporter selectors, dynamic parameter UI, summary/query handlers, and FFmpeg encoding pipeline.
* **`src/plugin/shared.rs`** — Pixel format and color space conversion helpers shared between importer and exporter.
* **`src/utils/`** — `UIBuilder`, UTF-16 helpers, RAM detection, and Adobe preference readers.

</details>

---

## License 📄

Licensed under the **GNU General Public License v3.0** with the **Adobe SDK Linking Exception** under **AnarchByte**. See [LICENSE](LICENSE) for full details.

FFmpeg and its bundled codecs are subject to their respective LGPL/GPL and third-party licenses.
