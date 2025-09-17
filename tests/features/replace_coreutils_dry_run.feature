Feature: Replace coreutils dry-run (Arch)
  As an Arch operator
  I want a safe preview of pacman removals under replace

  Scenario: replace coreutils dry-run prints pacman command
    Given a staging root at /tmp/fakeroot
    And a fakeroot with stock coreutils applets
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch replace coreutils`
    Then the command exits 0
    And stderr contains `[dry-run] would run: pacman -R --noconfirm coreutils`
