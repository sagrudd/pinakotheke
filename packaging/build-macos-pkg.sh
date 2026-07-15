#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
set -eu

arch=${1:?usage: build-macos-pkg.sh x86_64|arm64 VERSION DIST}
version=${2:?usage: build-macos-pkg.sh x86_64|arm64 VERSION DIST}
dist=${3:?usage: build-macos-pkg.sh x86_64|arm64 VERSION DIST}
[ "$(uname -s)" = Darwin ] || { echo "macOS PKG builds require macOS and pkgbuild" >&2; exit 2; }
command -v pkgbuild >/dev/null || { echo "pkgbuild is required (install Xcode command-line tools)" >&2; exit 2; }
case "$arch" in
  x86_64) target=x86_64-apple-darwin ;;
  arm64) target=aarch64-apple-darwin ;;
  *) echo "unsupported macOS architecture: $arch" >&2; exit 2 ;;
esac

rustup target add "$target"
cargo +1.97.0 build --locked --release -p x-img-cli --target "$target"
root="target/package-macos/$arch/root"
rm -rf "$root"
mkdir -p "$root/usr/local/bin" "$root/usr/local/share/x-img/monas" "$root/usr/local/share/doc/x-img" "$dist/macos/$arch"
install -m 0755 "target/$target/release/x-img" "$root/usr/local/bin/x-img"
install -m 0644 contracts/monas/x-img-product-bootstrap.v1.json "$root/usr/local/share/x-img/monas/product-bootstrap.json"
install -m 0644 LICENSE "$root/usr/local/share/doc/x-img/LICENSE"
COPYFILE_DISABLE=1 pkgbuild --root "$root" --identifier com.github.sagrudd.x-img --version "$version" \
  --install-location / "$dist/macos/$arch/x-img-$version-macos-$arch.pkg"
