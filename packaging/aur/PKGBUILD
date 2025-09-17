# Maintainer: Your Name <you@example.com>

pkgname=oxidizr-arch
pkgver=0.1.0
pkgrel=1
epoch=
pkgdesc="oxidizr-arch style coreutils switching tool (dry-run capable)"
arch=('x86_64' 'aarch64')
url="https://github.com/veighnsche/oxidizr-arch"
license=('Apache' 'MIT')
depends=('bash' 'pacman')
makedepends=('rust' 'cargo')
provides=('oxidizr-arch')
conflicts=('oxidizr-arch-git')
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/veighnsche/oxidizr-arch/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
  cd "${srcdir}/${pkgname}-${pkgver}" 2>/dev/null || cd "${srcdir}/${pkgname}-v${pkgver}"
  cargo build -p oxidizr-arch --release --locked
}

package() {
  cd "${srcdir}/${pkgname}-${pkgver}" 2>/dev/null || cd "${srcdir}/${pkgname}-v${pkgver}"
  install -Dm755 "target/release/oxidizr-arch" "${pkgdir}/usr/bin/oxidizr-arch"
  install -Dm644 cargo/oxidizr-arch/README.md "${pkgdir}/usr/share/doc/${pkgname}/README.md"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
  install -Dm644 LICENSE-MIT "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE-MIT"
}
