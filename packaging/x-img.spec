# SPDX-License-Identifier: MPL-2.0
Name: x-img
Version: %{ximg_version}
Release: 1%{?dist}
Summary: x-img metadata CLI and Monas product contract
License: MPL-2.0
URL: https://github.com/sagrudd/x-img
BuildArch: %{_arch}

%description
x-img command-line metadata tools and the versioned Monas product bootstrap.
Media bytes are not included and remain under DASObjectStore authority.

%install
mkdir -p %{buildroot}/usr/bin %{buildroot}/usr/share/x-img/monas %{buildroot}/usr/share/doc/x-img
install -m 0755 %{ximg_binary} %{buildroot}/usr/bin/x-img
install -m 0644 /workspace/contracts/monas/x-img-product-bootstrap.v1.json %{buildroot}/usr/share/x-img/monas/product-bootstrap.json
install -m 0644 /workspace/LICENSE %{buildroot}/usr/share/doc/x-img/LICENSE

%files
/usr/bin/x-img
/usr/share/x-img/monas/product-bootstrap.json
/usr/share/doc/x-img/LICENSE
