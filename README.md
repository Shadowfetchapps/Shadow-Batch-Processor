# Shadow Batch Processor

**Many files, one local pipeline.**

A native GTK4 / libadwaita desktop app for renaming, converting, compressing, and organizing copies of video, audio, and images. It is not a website and not Electron.

![Shadow Batch Processor icon](data/icons/hicolor/128x128/apps/shadow-batch-processor.png)

## Features

- Drop or add tens to hundreds of files
- Reorderable pipeline: rename / number, image convert+resize, video convert, audio convert, extract audio, strip metadata
- Built-in presets: YouTube Images, Website Images, Archive Photos, 1080p Video, MP3 Conversion, App Assets
- Preview source → copy names before you run
- **Process Copies** is the default. Replacing originals needs an explicit confirm
- Pause between files, cancel (kills child processes), isolated failures
- Completion report with per-file errors and technical details
- Offline. No accounts, ads, or telemetry

## Screenshots

Add captures to `docs/screenshots/` without private paths. The folder is reserved if no captures are checked in.

## Install

```bash
git clone https://github.com/Shadowfetchapps/Shadow-Batch-Processor.git
cd Shadow-Batch-Processor
cargo build --release
./scripts/install-user.sh
shadow-batch-processor
```

## Limitations

- Pause waits between files; a file already inside FFmpeg finishes or is cancelled
- Very large batches (thousands) stay sequential
- Image ICC handling matches ImageMagick / FFmpeg limits documented in Shadow Convert
- Destructive mode is discouraged and confirmed

## License

MIT. See [LICENSE](LICENSE).
