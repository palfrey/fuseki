DEVICE_HOST ?= remarkable
DEVICE_CONN ?= root@$(DEVICE_HOST)

deploy-demo:
	ssh $(DEVICE_CONN) 'killall -q -9 fuseki || true; killall -q -9 tarnish || true'
	rsync gnugo/interface/gnugo $(DEVICE_CONN):
	rsync target/armv7-unknown-linux-gnueabihf/release/fuseki $(DEVICE_CONN):
	ssh $(DEVICE_CONN) 'RUST_BACKTRACE=1 RUST_LOG=debug ./fuseki'

build:
	cross build --target armv7-unknown-linux-gnueabihf --release