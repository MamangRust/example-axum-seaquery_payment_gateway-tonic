build-genproto:
	cargo build -p genproto

clipy:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all -- --check

build-client:
	cargo build --release --target x86_64-unknown-linux-musl --package seaquery_client_payment_gateway

build-server:
	cargo build --release --target x86_64-unknown-linux-musl --package seaquery_server_payment_gateway

up:
	docker compose up -d

down:
	docker compose down
