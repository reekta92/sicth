# Maintainer: reekta92 mdag.92988@protonmail.com
pkgname=sicth-bin
pkgver=1.1.1
pkgrel=1
pkgdesc="Minimal TUI file navigator with fuzzy search"
url="https://github.com/reekta92/sicth"
license=("GPL-3.0")
arch=("x86_64")
provides=("sicth")
conflicts=("sicth")
depends=("gcc-libs")
source=("https://github.com/reekta92/sicth/releases/download/v1.1.1/sicth-x86_64-unknown-linux-gnu.tar.xz")
sha256sums=("2e73af2a00576697fc930878ee543ba58fadfa1a33ca6c702be75a9f1fefc079")

package() {
    install -Dm755 "sicth" -t "$pkgdir/usr/bin"
}
