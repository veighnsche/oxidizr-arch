Feature: Dry-run use findutils (Arch)
  As an Arch operator
  I want a safe dry-run for using replacements (findutils)

  Scenario: dry-run use findutils
    Given a staging root at /tmp/fakeroot
    And a verified replacement artifact is available for package "findutils"
    When I run `oxidizr-arch use findutils`
    Then the command exits 0
    And it reports a dry-run with a non-zero planned action count
