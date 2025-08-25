# Fuseki

[Fuseki](https://senseis.xmp.net/?Fuseki) is a Go frontend for Remarkable tablets. It uses a copy of [GNU Go](https://github.com/palfrey/gnugo) to do most of the engine work, running locally on the tablets, with a few patches for cross-compiling. Note this has all only been tested on a Remarkable 1, as hacking a 2 is a moving target (and my 2 is my actually used tablet v.s. the 1 which is just for hacking around).

The [Toltec toolchain](https://github.com/toltec-dev/toolchain) is used for the build and we provide [Draft](https://github.com/dixonary/draft-reMarkable?tab=readme-ov-file#draft-remarkable) config for launching.

## Build instructions

1. Checkout this repository (including submodules)
2. cd `gnugo`
3. `autoreconf --install` (note this is done outside of the Docker container in the next step as we need Autoconf >= 2.71 and the build container doesn't have that)
4. ``docker run --rm -it -v `pwd`:/work ghcr.io/toltec-dev/base:v3.2 bash``
    * 5-7 are inside this shell
5. `cd /work/gnugo`
6. `CFLAGS="-O2 -flto=auto" LDFLAGS="-O2 -flto=auto" ./configure --host arm-remarkable-linux-gnueabihf --build x86_64-linux-gnu --without-curses --without-docs`
7. `make`
    * `interface/gnugo` should now exist
8. Exit the docker container, and return to the root directory
9. `rustup target add --toolchain stable armv7-unknown-linux-gnueabihf`
10. `make build deploy-demo`
     * You may need to follow the [SSH access guide](https://remarkable.guide/guide/access/ssh.html) and set `DEVICE_HOST` to something accordingly (`10.11.99.1` probably if you're connected via USB)