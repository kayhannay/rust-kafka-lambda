DOCKER_IMAGE ?= ghcr.io/kayhannay/lambda-rust/lambda-rust:latest

test:
	RUST_LOG=rust_kafka_lambda=info cargo test

build-local:
	cargo build --release

build:
	mkdir -p target
	docker pull $(DOCKER_IMAGE)
	docker run --pull --rm \
    	-v ${PWD}:/code \
    	-v ${HOME}/.cargo/registry:/root/.cargo/registry \
    	-v ${HOME}/.cargo/git:/root/.cargo/git \
    	$(DOCKER_IMAGE)
	cp ./target/lambda/release/rust_kafka_lambda.zip ./target/bootstrap.zip
