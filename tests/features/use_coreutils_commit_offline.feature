Feature: Use coreutils with offline artifact (dry-run)
  As an Arch operator
  I want to preview using a local artifact safely

  Scenario: dry-run use coreutils with offline artifact
    Given a staging root at /tmp/fakeroot
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch use coreutils`
    Then the command exits 0
    And it reports a dry-run with a non-zero planned action count
