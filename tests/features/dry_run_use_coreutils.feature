Feature: Dry-run use coreutils (Arch)
  As an Arch operator
  I want a safe dry-run for using replacements

  Scenario: dry-run use coreutils
    Given a staging root at /tmp/fakeroot
    And a fakeroot with stock coreutils applets
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch use coreutils`
    Then the command exits 0
    And it reports a dry-run with a non-zero planned action count
