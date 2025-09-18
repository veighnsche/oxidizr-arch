# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and this project adheres to Semantic Versioning.

## [0.1.0] - 2025-09-18
- Initial release.
- Features:
  - `status` and `doctor` commands.
  - Safety-first policy defaults; dry-run by default, `--commit` to apply.
  - Arch integration: pacman DB lock detection, package enumeration.
  - Shell completions (bash, zsh, fish) and generated man page during packaging.
- Packaging:
  - PKGBUILD for AUR and `-git` VCS variant.
  - Installs completions and README.
  - Optdepends for `paru`, `sudo`, `policycoreutils` (getenforce).
