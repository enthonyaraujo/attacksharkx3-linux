%global debug_package %{nil}
%{!?app_version:%global app_version 1.0.0}
%{!?_udevrulesdir:%global _udevrulesdir %{_prefix}/lib/udev/rules.d}

Name:           attack-shark-x3
Version:        %{app_version}
Release:        1%{?dist}
Summary:        Configuration utility for the Attack Shark X3 gaming mouse
License:        MIT
URL:            https://github.com/enthonyaraujo/attacksharkx3-linux
Source0:        %{name}-%{version}.tar.gz
ExclusiveArch:  x86_64 aarch64

# Graphics libraries are dlopened, so ELF dependency scanning misses them.
# SONAME dependencies work across RPM distributions.
Requires:       libX11.so.6()(64bit)
Requires:       libX11-xcb.so.1()(64bit)
Requires:       libxcb.so.1()(64bit)
Requires:       libXcursor.so.1()(64bit)
Requires:       libXi.so.6()(64bit)
Requires:       libXrandr.so.2()(64bit)
Requires:       libxkbcommon.so.0()(64bit)
Requires:       libxkbcommon-x11.so.0()(64bit)
Requires:       libGL.so.1()(64bit)
Requires:       libEGL.so.1()(64bit)
Requires:       /usr/bin/Xwayland
Requires:       /usr/bin/pkexec
Requires:       /usr/bin/udevadm

%description
Native graphical configuration utility for the Attack Shark X3 gaming mouse.
Supports DPI, sensor, buttons, macros, profiles and live battery status.
The binary is built separately so releases can use an older glibc baseline.

%prep
%autosetup

%install
mkdir -p %{buildroot}
cp -a usr %{buildroot}/

%post
udevadm control --reload-rules 2>/dev/null || :
udevadm trigger --subsystem-match=hidraw --action=change 2>/dev/null || :
udevadm settle --timeout=5 2>/dev/null || :

%postun
udevadm control --reload-rules 2>/dev/null || :
udevadm trigger --subsystem-match=hidraw --action=change 2>/dev/null || :

%files
%{_bindir}/attack-shark-x3
%{_prefix}/libexec/attack-shark-x3-install-udev
%{_udevrulesdir}/70-attack-shark-x3.rules
%{_datadir}/attack-shark-x3/
%{_datadir}/applications/attack-shark-x3.desktop
%{_datadir}/icons/hicolor/256x256/apps/attack-shark-x3.png
%license %{_datadir}/licenses/attack-shark-x3/
%doc %{_datadir}/doc/attack-shark-x3/

%changelog
* Tue Oct 20 2026 Enthony <contact@example.com> - 1.0.0-1
- Install graphics dependencies and configure mouse access automatically
