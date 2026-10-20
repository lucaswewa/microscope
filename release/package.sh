#!/usr/bin/env bash
# Builds the release's Linux archive and .deb (P02), from the repository's
# root, once the web app and the release binaries are built:
#
#   npm --prefix web run build
#   cargo build --release -p microscope-server -p microscope-launcher --locked
#   ./release/package.sh 0.1.0
#
# The archive holds what the Windows zip holds (package.ps1). The .deb runs
# the server as the microscope service (release/linux/): the binaries in
# /usr/bin, its configuration in /etc/microscope, and its settings and data
# in /var/lib/microscope. Both are written to release-out/.
set -euo pipefail
version=${1:-dev}
# Where Cargo built the binaries, and where the packages go.
target=${2:-target}
out=${3:-release-out}

case $(uname -m) in
    x86_64) arch=x86_64 deb_arch=amd64 ;;
    aarch64) arch=arm64 deb_arch=arm64 ;;
    *) echo "no package for $(uname -m)" >&2; exit 1 ;;
esac
bin=$target/release

name=microscope-$version-linux-$arch
stage=$out/$name
rm -rf "$stage"
mkdir -p "$stage/configs"
cp "$bin/microscope-launcher" "$bin/microscope-server" "$stage"
cp configs/*.json "$stage/configs"
cp README.md LICENSE NOTICE "$stage"
# The third-party licences, as package.ps1 gathers them.
cargo fetch --locked
cargo about generate --offline --fail -c release/about.toml \
    -o "$stage/THIRD-PARTY-RUST.txt" release/about.hbs
cp web/dist/third-party-licences.txt "$stage/THIRD-PARTY-NPM.txt"
tar -czf "$out/$name.tar.gz" -C "$out" "$name"
echo "packaged $out/$name.tar.gz"

# A Debian version starts with a digit.
deb_version=${version#v}
[[ $deb_version =~ ^[0-9] ]] || deb_version=0~$deb_version
deb=$out/deb-$arch
rm -rf "$deb"
install -D -m 755 -t "$deb/usr/bin" "$bin/microscope-launcher" "$bin/microscope-server"
install -D -m 644 release/linux/microscope.service "$deb/usr/lib/systemd/system/microscope.service"
install -D -m 644 release/linux/default "$deb/etc/default/microscope"
mkdir -p "$deb/etc/microscope"
jq '.settings_folder = "/var/lib/microscope/settings"
    | .application_config.data_folder = "/var/lib/microscope/data"
    | .application_config.log_folder = "/var/log/microscope"' \
    configs/simulation.json > "$deb/etc/microscope/config.json"
install -D -m 644 -t "$deb/usr/share/doc/microscope" README.md NOTICE "$stage"/THIRD-PARTY-*.txt
install -m 644 LICENSE "$deb/usr/share/doc/microscope/copyright"
install -D -m 755 -t "$deb/DEBIAN" release/linux/postinst release/linux/prerm release/linux/postrm
printf '%s\n' /etc/default/microscope /etc/microscope/config.json > "$deb/DEBIAN/conffiles"
# The newest glibc the binaries need.
glibc=$(objdump -T "$bin/microscope-launcher" "$bin/microscope-server" \
    | grep -o 'GLIBC_[0-9.]*' | cut -d_ -f2 | sort -uV | tail -1)
sed -e "s/@VERSION@/$deb_version/" -e "s/@ARCH@/$deb_arch/" -e "s/@GLIBC@/$glibc/" \
    release/linux/control > "$deb/DEBIAN/control"
dpkg-deb -Zxz --root-owner-group --build "$deb" "$out/microscope_${deb_version}_$deb_arch.deb"
dpkg-deb --info "$out/microscope_${deb_version}_$deb_arch.deb"
