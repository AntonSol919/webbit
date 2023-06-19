
.PHONY: build run linkspace-js 

build: linkspace-js
	cargo build

watch: linkspace-js
	watchexec -o restart -w ./src/ -- make run 

run: linkspace-js
	RUST_LOG=trace cargo run


linkspace-js:
	make -C "../linkspace/ffi/linkspace-js"
	cp -r ./static/linkspace-dev/* ./static/linkspace/
