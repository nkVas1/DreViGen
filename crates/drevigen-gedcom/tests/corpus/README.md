# Test corpora

## `gedcom7code/`

GEDCOM 5.5.1 files in `5/`, and in `7/` the GEDCOM 7 file each should convert to — written by
Luther Tychonievich, the editor of the GEDCOM 7 specification, to test a 5.5.1-to-7.0 converter.
Taken from [gedcom7code/test-files](https://github.com/gedcom7code/test-files) at commit
`4418c40978bdf9e56bbf612558548d670c590816` (2025-11-19), unchanged.

They are released into the public domain under the Unlicense (`gedcom7code/LICENSE`), which is
why they can be committed here; the official FamilySearch test files cannot, and are fetched by
`tools/fetch-gedcom-testfiles.sh` instead. The upstream README is kept beside them as
`UPSTREAM-README.md`: it lists the differences between two files that do not change their
meaning, which matters when comparing a conversion against `7/`.

Some files are UTF-16 and some are not UTF-8 at all, so `.gitattributes` exempts this directory
from line-ending normalisation: every byte is what the test reads.
