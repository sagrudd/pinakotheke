# SPDX-License-Identifier: MPL-2.0
SHELL := /bin/sh
.DEFAULT_GOAL := help

VERSION := $(shell cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')
DIST := $(CURDIR)/dist

.PHONY: help all packages linux linux-x86_64 linux-arm64 linux-deb linux-rpm \
	linux-deb-x86_64 linux-deb-arm64 linux-rpm-x86_64 linux-rpm-arm64 \
	macos-pkg macos-pkg-x86_64 macos-pkg-arm64 firefox firefox-macos-x86_64 \
	firefox-macos-arm64 firefox-windows-x86_64 firefox-windows-arm64 \
	firefox-linux-x86_64 firefox-linux-arm64 checksums verify quality clean

help:
	@echo "x-img $(VERSION) packaging targets"
	@echo "  make packages              Build every native package and Firefox XPI"
	@echo "  make linux                 Build DEB and RPM for Linux x86_64 and arm64"
	@echo "  make macos-pkg             Build macOS PKG for x86_64 and arm64 (macOS only)"
	@echo "  make firefox               Build labelled XPIs for macOS/Windows/Linux x86_64/arm64"
	@echo "  make verify                Verify produced package structure and checksums"
	@echo "  make quality               Run local source, audit, and package checks"
	@echo "  make clean                 Remove dist/ and packaging scratch"

all: packages
packages: linux macos-pkg firefox checksums verify

linux: linux-x86_64 linux-arm64
linux-deb: linux-deb-x86_64 linux-deb-arm64
linux-rpm: linux-rpm-x86_64 linux-rpm-arm64
linux-deb-x86_64 linux-rpm-x86_64: linux-x86_64
linux-deb-arm64 linux-rpm-arm64: linux-arm64

linux-x86_64:
	@mkdir -p "$(DIST)/linux/x86_64"
	docker buildx build --build-arg VERSION=$(VERSION) \
		--build-arg RUST_TARGET=x86_64-unknown-linux-gnu --build-arg DEB_ARCH=amd64 --build-arg RPM_ARCH=x86_64 \
		-f packaging/Dockerfile.linux --output type=local,dest="$(DIST)/linux/x86_64" .

linux-arm64:
	@mkdir -p "$(DIST)/linux/arm64"
	docker buildx build --build-arg VERSION=$(VERSION) \
		--build-arg RUST_TARGET=aarch64-unknown-linux-gnu --build-arg DEB_ARCH=arm64 --build-arg RPM_ARCH=aarch64 \
		-f packaging/Dockerfile.linux --output type=local,dest="$(DIST)/linux/arm64" .

macos-pkg: macos-pkg-x86_64 macos-pkg-arm64
macos-pkg-x86_64:
	packaging/build-macos-pkg.sh x86_64 $(VERSION) "$(DIST)"

macos-pkg-arm64:
	packaging/build-macos-pkg.sh arm64 $(VERSION) "$(DIST)"

firefox: firefox-macos-x86_64 firefox-macos-arm64 firefox-windows-x86_64 \
	firefox-windows-arm64 firefox-linux-x86_64 firefox-linux-arm64

firefox-macos-x86_64:
	python3 packaging/build-firefox.py --os macos --arch x86_64 --version $(VERSION) --dist "$(DIST)"
firefox-macos-arm64:
	python3 packaging/build-firefox.py --os macos --arch arm64 --version $(VERSION) --dist "$(DIST)"
firefox-windows-x86_64:
	python3 packaging/build-firefox.py --os windows --arch x86_64 --version $(VERSION) --dist "$(DIST)"
firefox-windows-arm64:
	python3 packaging/build-firefox.py --os windows --arch arm64 --version $(VERSION) --dist "$(DIST)"
firefox-linux-x86_64:
	python3 packaging/build-firefox.py --os linux --arch x86_64 --version $(VERSION) --dist "$(DIST)"
firefox-linux-arm64:
	python3 packaging/build-firefox.py --os linux --arch arm64 --version $(VERSION) --dist "$(DIST)"

checksums:
	python3 packaging/check.py --dist "$(DIST)" --version $(VERSION) --write-checksums

verify:
	python3 packaging/check.py --dist "$(DIST)" --version $(VERSION)

quality:
	scripts/quality/check.sh
	scripts/audit/check.sh
	python3 packaging/check.py --source-only --version $(VERSION)

clean:
	rm -rf "$(DIST)" target/package-macos
