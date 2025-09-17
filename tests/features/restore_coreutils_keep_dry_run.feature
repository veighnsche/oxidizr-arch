Feature: Restore coreutils with keep (dry-run)
  As an Arch operator
  I want to see planned restore actions and keep replacements installed

  Scenario: restore coreutils dry-run with --keep-replacements
    Given a staging root at /tmp/fakeroot
    And a fakeroot with stock coreutils applets
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch restore coreutils --keep-replacements`
    Then the command exits 0
    And stderr contains `[dry-run] would run: pacman -S --noconfirm coreutils`
