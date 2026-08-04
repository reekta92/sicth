# Maintainer: reekta92 mdag.92988@protonmail.com
pkgname=sicth-bin
pkgver=1.0.2
pkgrel=1
pkgdesc="Minimal TUI file navigator with fuzzy search"
url="https://github.com/reekta92/sicth"
license=("GPL-3.0")
arch=("x86_64")
provides=("sicth")
conflicts=("sicth")
depends=("gcc-libs")
source=("https://github.com/reekta92/sicth/releases/download/v${pkgver}/sicth-x86_64-unknown-linux-gnu.tar.xz")
sha256sums=("3d6c4837e732e261e71ef243a5e04a95e900c4662463e942ef4032d8196f1a4a")

package() {
    install -Dm755 "sicth" -t "$pkgdir/usr/bin"
}
