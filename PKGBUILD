# Maintainer: reekta92 mdag.92988@protonmail.com
pkgname=sicth-bin
pkgver=1.0.3
pkgrel=1
pkgdesc="Minimal TUI file navigator with fuzzy search"
url="https://github.com/reekta92/sicth"
license=("GPL-3.0")
arch=("x86_64")
provides=("sicth")
conflicts=("sicth")
depends=("gcc-libs")
source=("https://github.com/reekta92/sicth/releases/download/v${pkgver}/sicth-x86_64-unknown-linux-gnu.tar.xz")
sha256sums=("cd33a034c5ae5c58b5b8846a8d6d615292396cd91dea6afec7d5666d1481005b")

package() {
    install -Dm755 "sicth" -t "$pkgdir/usr/bin"
}
