Feature: Replace coreutils coverage preflight fails when missing applets
  As an Arch operator
  I want replace to fail closed if replacement does not cover all GNU applets

  Scenario: preflight fails missing cat
    Given a staging root at /tmp/fakeroot
    And a fakeroot with stock coreutils applets
    And a verified replacement artifact lists applets "ls" for package "coreutils"
    When I run `oxidizr-arch replace coreutils`
    Then the command exits 1
    And stderr contains `missing: cat`
