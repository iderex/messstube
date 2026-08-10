# Changelog

Nothing has been released from this repository. The first release is milestone
9, and #70 is where this file gets the entry shape a reader can rely on, the two
version numbers, and the release gate that refuses a release whose entry is
missing or still sits under the unreleased heading. Nothing here settles any of
that.

The file exists from this change because the pull request hygiene check refuses
a version in a manifest that no changelog entry accounts for, and the first
reader adds a crate and so a version. An entry below is a statement about what
landed, not a promise about the shape of the next one.

## Unreleased

- The Tektronix ISF waveform reader, at the sketched maturity level, which is
  the level that claims no file from a physical instrument has been read
  (#48).
