# tesla-flac

CLI for processing `flac` files to be better rendered by Tesla's USB music player. Specifically updates multi-disc albums and artist metadata.

## Usage

Build/install with cargo. Nix users may do so from the dev shell with `nix develop`

```bash
Usage: tesla-flac [OPTIONS] --root <ROOT>

Options:
  -r, --root <ROOT>  Path to music directory
      --reset        Reset previously processed files
  -h, --help         Print help
  -V, --version      Print version
```
