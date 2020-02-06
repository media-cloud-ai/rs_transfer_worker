.PHONY: build ci-code-format ci-code-coverage ci-lint ci-tests docker-build docker-clean docker-push-registry run version

ENVFILE?=.env
ifeq ($(shell test -e $(ENVFILE) && echo -n yes),yes)
	include ${ENVFILE}
	export
endif

CI_REGISTRY?=
DOCKER_IMG_NAME?=mediacloudai/transfer_worker
ifneq ($(CI_REGISTRY), ) 
	DOCKER_IMG_NAME := /${DOCKER_IMG_NAME}
endif
VERSION=$(shell cargo metadata --no-deps --format-version 1 | jq '.packages[0].version' )

build:
	@cargo build

ci-code-format:
	@cargo fmt --all -- --check

ci-code-coverage:
	@cargo tarpaulin

ci-lint:
	@cargo clippy

ci-tests:
	@cargo test

docker-build:
	@docker build -t ${CI_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} .
	@docker tag ${CI_REGISTRY}${DOCKER_IMG_NAME}:${VERSION} ${CI_REGISTRY}${DOCKER_IMG_NAME}:${CI_COMMIT_SHORT_SHA}

docker-clean:
	@docker rmi ${CI_REGISTRY}${DOCKER_IMG_NAME}:${VERSION}
	@docker rmi ${CI_REGISTRY}${DOCKER_IMG_NAME}:${CI_COMMIT_SHORT_SHA}

docker-registry-login:
	@docker login --username "${CI_REGISTRY_USER}" -p"${CI_REGISTRY_PASSWORD}" ${CI_REGISTRY}
	
docker-push-registry:
	@docker push ${CI_REGISTRY}${DOCKER_IMG_NAME}:${VERSION}
	@docker push ${CI_REGISTRY}${DOCKER_IMG_NAME}:${CI_COMMIT_SHORT_SHA}

run:
	@cargo run rs_transfer_worker

version:
	@echo ${VERSION}
