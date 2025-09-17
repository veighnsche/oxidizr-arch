Feature: Sudo setuid guard fails without 4755
  As an Arch operator
  I want commit use sudo to fail if the replacement is not setuid root

  Scenario: commit use sudo fails without setuid 4755
    Given a staging root at /tmp/fakeroot
    And a verified replacement artifact is available for package "sudo"
    When I run `oxidizr-arch --commit use sudo`
    Then the command exits 1
    And stderr contains `not setuid`
