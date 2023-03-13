
# Building the CLI and the agents

The Sockets Map CLI and agents can be built using `cargo build --release -p sockets_map_cli` and `cargo build --release -p sockets_map_agent`. They can be cross compiled using [Cross](https://github.com/cross-rs/cross).

You can install the CLI by using `cargo install -p sockets_map_cli`.

To build the agent with MUSL support (if your libc version does not match the one on the machine you want to deploy the agent on):

```bash
cargo build --release --target x86_64-unknown-linux-musl -p sockets_map_agent
```

# Building the GUI

## For Linux

To build the GUI on Linux, make sure you have the necessary GTK4, libadwaita and adwaita-icon-theme packages installed. For more information, see:

- [GTK4 Linux development installation](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_linux.html)
- [libadwaita Linux development installation](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html)

Then, run:

```bash
# Build
cargo build --release -p sockets_map_gui

# Or install
cargo install -p sockets_map_gui
```

A .deb archive can be built for Debian-based distributions using [cargo deb](https://crates.io/crates/cargo-deb): 

```bash
cargo deb -p sockets_map_gui
```

## For Windows

There are two ways to build the GUI for Windows: either on a Windows machine, or by cross-compiling from a Linux machine.

### From a Linux machine

You can use the following commands to cross-compile from a GTK4 ready docker. These commands also bundle the necessary dependencies with the Adwaita icons into a zip file.

```bash
docker run -ti -v `pwd`:/mnt mglolenstine/gtk4-cross:rust-gtk-4.8 /bin/bash -c "build; package; cp -r /usr/share/icons/Adwaita/ ./package/share/icons/Adwaita"
zip -r sockets_map_gui_w64.zip package/*
rm -rf package
```

You can then ship the ZIP file, and execute the `sockets_map_gui.exe` binary from the extracted folder.

### From a Windows machine

To build the GUI on Windows, execute the following commands (this uses the Chocolatey package manager):

```powershell
# Compilation toolchain
rustup default stable-msvc
choco install git
choco install msys2
choco install visualstudio2022-workload-vctools
python -m pip install --user pipx
python -m pipx ensurepath
python -m pipx install gvsbuild

# Build GTK4 and libadwaita
gvsbuild build gtk4 libadwaita librsvg adwaita-icon-theme

# Build the GUI
cargo build --release -p sockets_map_gui
```
