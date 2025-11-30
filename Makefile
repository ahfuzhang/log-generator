IMAGE ?= log-generator:latest

.PHONY: docker-build
docker-build:
	docker build -t $(IMAGE) . --platform=linux/amd64

.PHONY: docker-run
docker-run:
	docker run --rm -it ahfuzhang/log-generator --batch_bytes=64k --sleep_ms=100 --output=stdout

.PHONY: docker-push
docker-push:
	docker tag $(IMAGE) ahfuzhang/log-generator:latest
	docker push ahfuzhang/log-generator:latest

.PHONY: build-linux
build-linux:
	docker run --rm -v $(PWD):/app -w /app rust:1.82 bash -c "\
		export PATH=/usr/local/cargo/bin:\$$PATH && \
		apt-get update && \
		apt-get install -y gcc-x86-64-linux-gnu libc6-dev-amd64-cross && \
		rustup target add x86_64-unknown-linux-gnu && \
		CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
		CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
		PKG_CONFIG_ALLOW_CROSS=1 \
		cargo build --release --target x86_64-unknown-linux-gnu \
	"

.PHONY: build-linux-musl
build-linux-musl:
	docker run --rm -v $(PWD):/home/rust/src -w /home/rust/src messense/rust-musl-cross:x86_64-musl \
		bash -c "\
			export PATH=\"/usr/local/musl/bin:/root/.cargo/bin:/usr/local/cargo/bin:\$$$$PATH\" && \
			export RUSTUP_TMPDIR=/root/.rustup/tmp && \
			mkdir -p /root/.rustup/tmp && \
			export CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc && \
			export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc && \
			for comp in clippy rustfmt; do \
					for p in /root/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/share/doc/\$$comp /root/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/lib/rustlib/aarch64-unknown-linux-gnu/share/doc/\$$comp; do \
						mkdir -p \$$p; \
					for f in README.md LICENSE-MIT LICENSE-APACHE; do touch \$$p/\$$f; done; \
				done; \
			done && \
			cargo build --release --target x86_64-unknown-linux-musl \
		"

k8s_deploy:
	KUBECONFIG=~/code/pg-stress-test.mp-games-ali-cn-hk-d.txt \
	kubectl apply -n logging -f k8s/deployment.yaml

jsonline_test:
	cargo run -- --output=http \
		--sleep_ms=1000 \
		--batch_bytes=64k \
		--http.jsonline="http://127.0.0.1:9428/insert/jsonline?_time_field=_time&_msg_field=_msg,http_request_query_string&_stream_fields=server_name,http_request_path,status_code&ignore_fields=&decolorize_fields=&AccountID=0&ProjectID=0&debug=false&extra_fields="
