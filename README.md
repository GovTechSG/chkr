# chkr

Utility to check md5 checksums of some files.

## Usage

```bash
$ cargo run -- --help
          oooo        oooo
          `888        `888
 .ooooo.   888 .oo.    888  oooo  oooo d8b
d88' `"Y8  888P"Y88b   888 .8P'   `888""8P
888        888   888   888888.     888
888   .o8  888   888   888 `88b.   888
`Y8bod8P' o888o o888o o888o o888o d888b

Usage:
  chkr file <file-path> <expected-checksum>
  chkr manifest <checksum-path>
  chkr (-h | --help)

chkr will return 0 for matches, 0x01 for mismatch, and 0x10 for other errors.

Options:
  -h --help     Show this screen.
```
