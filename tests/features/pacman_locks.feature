Feature: Pacman locks prevent commit (Arch)
  As an Arch operator
  I want oxidizr-arch to refuse to commit when package manager locks are present

  Scenario: pacman db lock blocks commit use
    Given a staging root at /tmp/fakeroot
    And a fakeroot with stock coreutils applets
    And a pacman db lock is held
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch --commit use coreutils`
    Then the command exits 1
    And stderr contains `Package manager busy (pacman db.lck detected); retry after current operation finishes.`
