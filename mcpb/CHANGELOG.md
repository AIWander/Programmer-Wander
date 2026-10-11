# Changelog: Programmer Claude Desktop extension

The extension version follows the Programmer binary version (Cargo.toml).

## 2.0.0-rc.1 (2026-10-11, pre-release, unsigned)

First Claude Desktop extension (MCPB) packaging of Programmer v2.

- One bundle per processor: `win-arm64` and `win-x64`, each with a single `programmer.exe`
  started by Claude Desktop as `programmer.exe serve`.
- 49 tools, listed in `manifest.json` and checked against the live `tools/list` at pack time.
- Manifest metadata for distribution: icon, homepage, documentation, support, repository,
  license (Apache-2.0), keywords, privacy policy, compatibility (Windows, Claude Desktop
  0.10.0 or newer).
- Bundle includes LICENSE, NOTICE, README (install, uninstall, troubleshooting) and PRIVACY.md.
- `pack.ps1` builds, checks, packs, unpack-tests and hashes the bundles; `-Sign` signs the exe
  before packing (see SIGNING.md).
- Built from repository main including the rustls update for RUSTSEC-2026-0285.

Known limits: no folder restriction setting; tools carry no MCP annotations yet; unsigned.
