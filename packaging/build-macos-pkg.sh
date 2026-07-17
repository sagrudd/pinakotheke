#!/bin/sh
# SPDX-License-Identifier: MPL-2.0
set -eu

arch=${1:?usage: build-macos-pkg.sh x86_64|arm64 VERSION DIST}
version=${2:?usage: build-macos-pkg.sh x86_64|arm64 VERSION DIST}
dist=${3:?usage: build-macos-pkg.sh x86_64|arm64 VERSION DIST [PRODUCT]}
product=${4:-x-img}
case "$product" in x-img|pinakotheke) ;; *) echo "unsupported product: $product" >&2; exit 2 ;; esac
[ "$(uname -s)" = Darwin ] || { echo "macOS PKG builds require macOS and pkgbuild" >&2; exit 2; }
command -v pkgbuild >/dev/null || { echo "pkgbuild is required (install Xcode command-line tools)" >&2; exit 2; }
case "$arch" in
  x86_64) target=x86_64-apple-darwin ;;
  arm64) target=aarch64-apple-darwin ;;
  *) echo "unsupported macOS architecture: $arch" >&2; exit 2 ;;
esac

rustup target add "$target"
web="$dist/web"
[ -f "$web/index.html" ] || { echo "missing built web application: run make web" >&2; exit 2; }
PINAKOTHEKE_DEFAULT_WEB_ROOT="/usr/local/share/$product/web" \
  cargo +1.97.0 build --locked --release -p pinakotheke-cli --target "$target"
root="target/package-macos/$arch/root"
scripts="target/package-macos/$arch/scripts"
rm -rf "$root"
rm -rf "$scripts"
mkdir -p "$root/usr/local/bin" "$root/usr/local/share/$product/monas" "$root/usr/local/share/$product/web" "$root/usr/local/share/doc/$product" "$dist/macos/$arch" "$scripts"
bootstrap=contracts/monas/x-img-product-bootstrap.v1.json
if [ "$product" = pinakotheke ]; then
  bootstrap=contracts/monas/x-img-product-bootstrap.v1.json
  install -m 0755 "target/$target/release/pinakotheke" "$root/usr/local/bin/pinakotheke"
  install -m 0755 "target/$target/release/x-img" "$root/usr/local/bin/x-img"
else
  install -m 0755 "target/$target/release/x-img" "$root/usr/local/bin/x-img"
fi
install -m 0644 "$bootstrap" "$root/usr/local/share/$product/monas/product-bootstrap.json"
cp -a "$web/." "$root/usr/local/share/$product/web/"
install -m 0644 LICENSE "$root/usr/local/share/doc/$product/LICENSE"
install -m 0755 packaging/macos/preinstall "$scripts/preinstall"
COPYFILE_DISABLE=1 pkgbuild --root "$root" --identifier "com.github.sagrudd.$product" --version "$version" \
  --scripts "$scripts" --install-location / "$dist/macos/$arch/$product-$version-macos-$arch.pkg"
