# SPDX-License-Identifier: MPL-2.0
%global __strip /bin/true
Name: %{product_name}
Version: %{ximg_version}
Release: 1%{?dist}
Summary: %{product_name} metadata CLI and Monas product contract
License: MPL-2.0
URL: https://github.com/sagrudd/%{product_name}
%if "%{product_name}" == "pinakotheke"
Provides: x-img = %{version}-%{release}
Obsoletes: x-img < %{version}-%{release}
%endif

%description
%{product_name} command-line metadata tools and the versioned Monas product bootstrap.
Media bytes are not included and remain under DASObjectStore authority.

%install
mkdir -p %{buildroot}/usr/bin %{buildroot}/usr/share/%{product_name}/monas %{buildroot}/usr/share/%{product_name}/web %{buildroot}/usr/share/doc/%{product_name}
install -m 0755 %{product_binary} %{buildroot}/usr/bin/%{product_name}
install -m 0644 %{product_bootstrap} %{buildroot}/usr/share/%{product_name}/monas/product-bootstrap.json
cp -a %{product_web}/. %{buildroot}/usr/share/%{product_name}/web/
install -m 0644 /workspace/LICENSE %{buildroot}/usr/share/doc/%{product_name}/LICENSE
%if "%{product_name}" == "pinakotheke"
install -m 0755 %{legacy_binary} %{buildroot}/usr/bin/x-img
%endif

%files
/usr/bin/%{product_name}
%if "%{product_name}" == "pinakotheke"
/usr/bin/x-img
%endif
/usr/share/%{product_name}/monas/product-bootstrap.json
/usr/share/%{product_name}/web/
/usr/share/doc/%{product_name}/LICENSE
