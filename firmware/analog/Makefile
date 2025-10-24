TARGET=thumbv7em-none-eabihf

.PHONY: build run size clean

build:
	cargo build --target $(TARGET)

run:
	cargo run --target $(TARGET)

size:
	cargo size --target $(TARGET) --release -- -A

clean:
	cargo clean

