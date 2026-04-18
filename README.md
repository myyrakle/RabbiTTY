# Rabbitty

<div align="center">
  <img src="assets/logo.png" alt="Rabbitty logo" width="200" />
  <p>Fast, lean, cross-platform terminal emulator.</p>
</div>

> Warn: This is a work-in-progress project.

Rabbitty is a terminal emulator chasing `foot`-like memory thrift and cross-platform speed, with feature-ful and polish.

- Lean memory: small, steady footprint even with deep scrollback.
- Fast paths: low-latency rendering and input.
- Cross-platform: consistent on macOS, Linux, Windows.
- Featureful and fancy: tabs, themes, and modern UX without bloat.

## Goals

- [x] SSH Managing
- [ ] Plugin support with wasm
- [x] Easy changing theme
- [ ] Easy file upload & download with SFTP
- [ ] Split terminal in single tab

## Release

GitHub Actions release workflow is available at `.github/workflows/release.yml`.

1. Create and push a git tag such as `v0.0.3`.
2. Run the `Release` workflow manually from the Actions tab with that tag selected as the workflow ref.
3. The workflow builds these targets and uploads them as workflow artifacts first:
   - `linux-amd64`
   - `linux-arm64`
   - `windows-amd64`
   - `macos-arm64`
4. The same artifacts are then attached to the GitHub Release for the tag.
5. If the workflow is started from a branch instead of a tag, it fails early.
