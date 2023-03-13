all: clean linux_package windows_package

linux_package: debian_gui_deb
	rm -f ./target/sockets_map_linux.zip || true
	cross build --release -p sockets_map_cli --target=x86_64-unknown-linux-gnu
	cross build --release -p sockets_map_agent --target=x86_64-unknown-linux-gnu
	cargo build --release -p sockets_map_gui
	zip -j ./target/sockets_map_linux.zip target/x86_64-unknown-linux-gnu/release/sockets_map_agent target/x86_64-unknown-linux-gnu/release/sockets_map target/release/sockets_map_gui

debian_gui_deb:
	cargo deb -p sockets_map_gui
	
windows_package:
	docker run -ti -v `pwd`:/mnt mglolenstine/gtk4-cross:rust-gtk-4.8 /bin/bash -c "build; package; cp -r /usr/share/icons/Adwaita/ ./package/share/icons/Adwaita"
	(cd package; zip -r ../target/sockets_map_windows.zip *; cd ..)

clean:
	cargo clean
	sudo rm -rf package
