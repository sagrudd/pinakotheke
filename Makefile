# SPDX-License-Identifier: MPL-2.0
SHELL := /bin/sh
.DEFAULT_GOAL := help

VERSION := $(shell cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')
DIST := $(CURDIR)/dist
BASELINE_DIST ?=
BASELINE_VERSION ?=
PRODUCT ?= pinakotheke

.PHONY: help all packages web firefox-gallery-check firefox-capture-check linux linux-x86_64 linux-arm64 linux-deb linux-rpm \
	linux-deb-x86_64 linux-deb-arm64 linux-rpm-x86_64 linux-rpm-arm64 \
	macos-pkg macos-pkg-x86_64 macos-pkg-arm64 firefox firefox-macos-x86_64 \
	firefox-macos-arm64 firefox-windows-x86_64 firefox-windows-arm64 \
	firefox-linux-x86_64 firefox-linux-arm64 sbom checksums verify upgrade-rollback v1-preflight v1-package-transition v1-cutover quality clean

help:
	@echo "$(PRODUCT) $(VERSION) packaging targets"
	@echo "  make packages              Build every native package and Firefox XPI"
	@echo "  make web                   Build the Monas-mounted Yew application"
	@echo "  make firefox-gallery-check Exercise the Yew gallery in installed Firefox"
	@echo "  make firefox-capture-check Exercise observed/opened capture in installed Firefox"
	@echo "  make linux                 Build DEB and RPM for Linux x86_64 and arm64"
	@echo "  make macos-pkg             Build macOS PKG for x86_64 and arm64 (macOS only)"
	@echo "  make firefox               Build labelled XPIs for macOS/Windows/Linux x86_64/arm64"
	@echo "  make verify                Verify produced package structure and checksums"
	@echo "  make sbom                  Generate the deterministic CycloneDX release SBOM"
	@echo "  make upgrade-rollback BASELINE_DIST=... BASELINE_VERSION=..."
	@echo "                              Exercise genuine package upgrade/downgrade acceptance"
	@echo "  make v1-preflight          Inventory coordinated Pinakotheke cutover blockers"
	@echo "  make v1-cutover            Refuse release unless every identity is canonical"
	@echo "  make quality               Run local source, audit, and package checks"
	@echo "  make clean                 Remove dist/ and packaging scratch"

all: packages
packages: linux macos-pkg firefox checksums verify

web:
	@mkdir -p "$(DIST)/web"
	cd crates/pinakotheke-web && NO_COLOR=true trunk build index.html --release \
		--public-url /products/pinakotheke/app/ --dist "$(DIST)/web"

firefox-gallery-check: web
	python3 scripts/firefox/check_gallery_browser.py --dist "$(DIST)/web"

firefox-capture-check:
	node scripts/firefox/check_installed_capture.mjs

linux: linux-x86_64 linux-arm64
linux-deb: linux-deb-x86_64 linux-deb-arm64
linux-rpm: linux-rpm-x86_64 linux-rpm-arm64
linux-deb-x86_64 linux-rpm-x86_64: linux-x86_64
linux-deb-arm64 linux-rpm-arm64: linux-arm64

linux-x86_64: web
	@mkdir -p "$(DIST)/linux/x86_64"
	docker buildx build --build-arg VERSION=$(VERSION) \
		--build-context web-assets="$(DIST)/web" \
		--build-arg PRODUCT_NAME=$(PRODUCT) \
		--build-arg RUST_TARGET=x86_64-unknown-linux-gnu --build-arg DEB_ARCH=amd64 --build-arg RPM_ARCH=x86_64 \
		-f packaging/Dockerfile.linux --output type=local,dest="$(DIST)/linux/x86_64" .

linux-arm64: web
	@mkdir -p "$(DIST)/linux/arm64"
	docker buildx build --build-arg VERSION=$(VERSION) \
		--build-context web-assets="$(DIST)/web" \
		--build-arg PRODUCT_NAME=$(PRODUCT) \
		--build-arg RUST_TARGET=aarch64-unknown-linux-gnu --build-arg DEB_ARCH=arm64 --build-arg RPM_ARCH=aarch64 \
		-f packaging/Dockerfile.linux --output type=local,dest="$(DIST)/linux/arm64" .

macos-pkg: macos-pkg-x86_64 macos-pkg-arm64
macos-pkg-x86_64: web
	packaging/build-macos-pkg.sh x86_64 $(VERSION) "$(DIST)" $(PRODUCT)

macos-pkg-arm64: web
	packaging/build-macos-pkg.sh arm64 $(VERSION) "$(DIST)" $(PRODUCT)

firefox: firefox-macos-x86_64 firefox-macos-arm64 firefox-windows-x86_64 \
	firefox-windows-arm64 firefox-linux-x86_64 firefox-linux-arm64

firefox-macos-x86_64:
	python3 packaging/build-firefox.py --product $(PRODUCT) --os macos --arch x86_64 --version $(VERSION) --dist "$(DIST)"
firefox-macos-arm64:
	python3 packaging/build-firefox.py --product $(PRODUCT) --os macos --arch arm64 --version $(VERSION) --dist "$(DIST)"
firefox-windows-x86_64:
	python3 packaging/build-firefox.py --product $(PRODUCT) --os windows --arch x86_64 --version $(VERSION) --dist "$(DIST)"
firefox-windows-arm64:
	python3 packaging/build-firefox.py --product $(PRODUCT) --os windows --arch arm64 --version $(VERSION) --dist "$(DIST)"
firefox-linux-x86_64:
	python3 packaging/build-firefox.py --product $(PRODUCT) --os linux --arch x86_64 --version $(VERSION) --dist "$(DIST)"
firefox-linux-arm64:
	python3 packaging/build-firefox.py --product $(PRODUCT) --os linux --arch arm64 --version $(VERSION) --dist "$(DIST)"

sbom:
	python3 packaging/sbom.py --product $(PRODUCT) --version $(VERSION) --dist "$(DIST)"

checksums: sbom
	python3 packaging/check.py --product $(PRODUCT) --dist "$(DIST)" --version $(VERSION) --write-checksums

verify:
	python3 packaging/check.py --product $(PRODUCT) --dist "$(DIST)" --version $(VERSION)

upgrade-rollback: verify
	BASELINE_DIST="$(BASELINE_DIST)" BASELINE_VERSION="$(BASELINE_VERSION)" scripts/release/check_upgrade_rollback.sh

v1-preflight:
	scripts/release/check_v1_cutover.sh --phase preflight
	python3 scripts/release/check_v1_rehearsal.py

v1-package-transition:
	python3 scripts/release/check_v1_package_transition.py

v1-cutover:
	scripts/release/check_v1_cutover.sh --phase cutover --github

quality:
	scripts/quality/check.sh
	scripts/audit/check.sh
	python3 packaging/check.py --product $(PRODUCT) --source-only --version $(VERSION)

clean:
	rm -rf "$(DIST)" target/package-macos
