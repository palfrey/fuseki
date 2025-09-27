DEVICE_HOST ?= remarkable
DEVICE_CONN ?= root@$(DEVICE_HOST)

deploy:
	ssh $(DEVICE_CONN) 'killall -q -9 fuseki || true'
	rsync gnugo/interface/gnugo $(DEVICE_CONN):
	rsync target/armv7-unknown-linux-gnueabihf/release/fuseki $(DEVICE_CONN):/opt/bin/fuseki
	ssh $(DEVICE_CONN) 'RUST_BACKTRACE=full RUST_LOG=debug /opt/bin/fuseki'

build:
	cross build --target armv7-unknown-linux-gnueabihf --release