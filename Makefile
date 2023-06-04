
.PHONY: build run linkspace-js 

build: linkspace-js
	cargo build

watch: linkspace-js
	watchexec -o restart -w ./src/ -- cargo run 

run: linkspace-js
	./sneak-in.sh ./example.forum.editor.html editor/basic.forum.html
	./sneak-in.sh ./template/html_editor.html hello/world.html
	cargo run


linkspace-js:
	make -C "../linkspace/ffi/linkspace-js"
