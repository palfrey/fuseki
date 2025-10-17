# Fuseki

[Fuseki](https://senseis.xmp.net/?Fuseki) is a Go frontend for Remarkable tablets. It uses a copy of [GNU Go](https://github.com/palfrey/gnugo) to do most of the engine work, running locally on the tablets, with a few patches for cross-compiling. Note this has all only been tested on a Remarkable 1, as hacking a 2 is a moving target (and my 2 is my actually used tablet v.s. the 1 which is just for hacking around).

<table style="border: 0px; border-style: none; border-spacing: 0px" spacing="0"><tr><td><img src="screenshots/start menu.jpg" width="200" /></td><td><img src="screenshots/machine game.jpg" width="200" /></td><td><img src="screenshots/atari game.jpg" width="200" /></td><td><img src="screenshots/dragon game.jpg" width="200" /></td></tr></table>

The [Toltec toolchain](https://github.com/toltec-dev/toolchain) is used for the build and we provide [Draft](https://github.com/dixonary/draft-reMarkable?tab=readme-ov-file#draft-remarkable) config for launching.

## Usage instructions

We have 3 modes: machine, Atari and Dragon Go Server

### Machine game

This is human v.s. machine, all running locally via Gnu Go. It'll get slower as the game goes on as Gnu Go is a pretty heavyweight thing for a Remarkable to run, even though I've dialed down it's accuracy.

### Atari game

This is a human v.s. human game of [Atari Go](https://senseis.xmp.net/?AtariGo). We use Gnu Go for move validation, but that's it.

### Dragon Go Server

To make this work, add your login for [Dragon Go Server](https://www.dragongoserver.net/) to `/opt/dragon-go-server-login`. It's a JSON file with `username` and `password` fields. The app will create a default file on first start of this mode if it doesn't exist. After that, it'll display whatever game in Dragon Go Server you'll lose first due to timeout. Select your move, click "commit" and it'll submit and go to your next game. If you're out of games, it'll show a refresh button, but will also update about every 10 minutes as well.
 
## Build instructions

1. Checkout this repository (including submodules)
2. cd `gnugo`
3. `autoreconf --install` (note this is done outside of the Docker container in the next step as we need Autoconf >= 2.71 and the build container doesn't have that)
4. ``docker run --rm -it -v `pwd`:/work ghcr.io/toltec-dev/base:v3.3 bash``
    * 5-7 are inside this shell
5. `cd /work/gnugo`
6. `CFLAGS="-O2 -flto=auto" LDFLAGS="-O2 -flto=auto" ./configure --host arm-remarkable-linux-gnueabihf --build x86_64-linux-gnu --without-curses --without-docs`
7. `make`
    * `interface/gnugo` should now exist
8. Exit the docker container, and return to the root directory
9. `rustup target add --toolchain stable armv7-unknown-linux-gnueabihf`
10. `make run`
     * You may need to follow the [SSH access guide](https://remarkable.guide/guide/access/ssh.html) and set `DEVICE_HOST` to something accordingly (`10.11.99.1` probably if you're connected via USB)

## Icon

Icon is derived from https://thenounproject.com/icon/go-181270/