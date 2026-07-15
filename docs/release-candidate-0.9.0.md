# x-img 0.9.0 release candidate

This GitHub prerelease is an evaluation checkpoint, not the Pinakotheke 1.0
brand migration. It packages the metadata CLI and versioned Monas bootstrap,
plus the least-privilege Firefox extension. Monas remains responsible for login
and authenticated host context; DASObjectStore remains the only durable media
authority.

## Evaluation scope

- DEB and RPM packages for x86_64 and arm64.
- Unsigned macOS PKGs for x86_64 and arm64.
- Deterministic unsigned Firefox XPIs labelled for macOS, Windows, and Linux on
  x86_64 and arm64. The extension files are platform-independent.
- `SHA256SUMS`, `release-manifest.v1.json`, and a CycloneDX 1.6 SBOM.
- Genuine 0.3.0 → 0.9.0 → 0.3.0 DEB/RPM acceptance on both architectures.

All artifacts explicitly report `signed: false`. Apple notarization and Mozilla
Add-ons signing were not performed because release credentials are not present.
Users must review platform warnings and should prefer source builds for this
evaluation checkpoint.

## Important limits

- Native packages contain the current CLI, MPL license, and host-composable
  Monas bootstrap; they do not install Monas, DASObjectStore, a competing login
  service, or durable media.
- X account functionality remains behind official authorization, policy, and
  host-secret boundaries. Fixture-tested contracts are not a claim of live X
  service availability.
- Instagram is handled as an explicitly enabled ordinary website through the
  Firefox observed-media path; there is no required dedicated Instagram API
  connector.
- Website capture is user opt-in. Only displayed thumbnails and explicitly
  opened originals are eligible; ordinary origin loading remains the fallback.
- Video normalization requires an authorized, digest-pinned Docker/FFmpeg
  worker and a supported Pinakotheke playback profile. DRM circumvention is not
  supported.
- GEO, SRA, ENA, and NCBI paths accept explicit reviewed resources only; no
  repository crawl or bulk discovery is provided.
- Hosted CI was not used. Local Rust, Firefox, packaging, contract, audit,
  rollback, and pinned Sphinx-container checks are the release evidence.

## Verification

Download the desired artifacts together with `SHA256SUMS`, then verify from the
download directory:

```console
shasum -a 256 -c SHA256SUMS
```

The checksum file covers the thirteen release artifacts. Inspect
`release-manifest.v1.json` for platform, architecture, byte length, checksum,
and signing state, and `x-img-0.9.0.cdx.json` for the software inventory.

Report defects against the public x-img repository. The coordinated rename to
Pinakotheke, compatibility aliases, and repository migration remain the 1.0.0
gate.
