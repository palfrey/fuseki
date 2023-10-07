DEVICE_IP ?= '192.168.1.144'
DEVICE_HOST ?= root@$(DEVICE_IP)

deploy-demo:
	ssh $(DEVICE_HOST) 'killall -q -9 fuseki || true; killall -q -9 tarnish || true'
	rsync target/armv7-unknown-linux-gnueabihf/release/fuseki $(DEVICE_HOST):
	ssh $(DEVICE_HOST) 'RUST_BACKTRACE=1 RUST_LOG=debug ./fuseki'

build:
	cross build --target armv7-unknown-linux-gnueabihf --release