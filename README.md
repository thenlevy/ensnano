# Dependencies

## All platforms

You need to have the latest rust compiler installed see [this page](https://www.rust-lang.org/tools/install).

** If you already have `cargo` installed but face compilation issue, make sure that you have the lastest version by running `rustup update` **

## Linux
You need the GTK3 development packages to build the dependency `nfd2`.

* Debian/Ubuntu: `apt-get install libgtk-3-dev`
* Fedora: `dnf install gtk3-devel`
* Arch: `pacman -S gtk3`

You also need to have the **Vulkan** driver for your graphic card installed. The installation methods depends on your distribution and graphic card,
but there should be a tutorial on the internet for any combination of those.

