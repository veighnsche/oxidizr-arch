Feature: Restore coreutils without keep (dry-run)
  As an Arch operator
  I want to preview restore actions including removal of the replacement package

  Scenario: restore coreutils dry-run without --keep-replacements
    Given a staging root at /tmp/fakeroot
    And a fakeroot with stock coreutils applets
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch restore coreutils`
    Then the command exits 0
    And stderr contains `[dry-run] would run: pacman -S --noconfirm coreutils`
    And stderr contains `[dry-run] would run: pacman -R --noconfirm uutils-coreutils`
