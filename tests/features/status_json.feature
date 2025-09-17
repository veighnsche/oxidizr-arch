Feature: Status JSON
  As an Arch operator
  I want to see active packages in JSON

  Scenario: status after commit use coreutils shows active
    Given a staging root at /tmp/fakeroot
    And a verified replacement artifact is available for package "coreutils"
    When I run `oxidizr-arch --commit use coreutils`
    Then the command exits 0
    When I run `oxidizr-arch status --json`
    Then the command exits 0
    And stdout contains `"coreutils":"active"`
