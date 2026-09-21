# Building

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev ffmpeg imagemagick librsvg2-bin desktop-file-utils
cargo build --release
cargo test
./scripts/generate-icons.sh
./scripts/install-user.sh
./scripts/build-deb.sh
```

`rustfmt` / `clippy` are optional on system Rust packages.
