# Code signing policy

G-SpaceAnalyzer is free and open source software (MIT). Its Windows binaries are signed so that
Windows can show who published them and confirm they were not altered after the build.

## What is signed

| File | Signed by | Where |
| --- | --- | --- |
| `GSpaceAnalyzerSetup.exe` (release asset) | SignPath Foundation certificate | GitHub Actions, tag builds |

The application executable is embedded inside the setup file, so signing the setup covers everything
a user runs. Files built on a developer machine are not published unless they come from the release
workflow.

## How it is signed

1. A tag matching the version in `Cargo.toml` triggers `.github/workflows/release.yml` on a
   GitHub-hosted Windows runner.
2. The workflow builds the app with `cargo build --release`, runs the test suite, and builds the
   installer with `installer/build-installer.ps1`.
3. The unsigned installer is stored as a GitHub Actions artifact and submitted to
   [SignPath.io](https://signpath.io/) with `signpath/github-action-submit-signing-request`.
   SignPath fetches the artifact itself from the GitHub run, verifies the repository, commit and
   workflow, and only then applies the signature with the key held in its HSM.
4. The signed installer replaces the unsigned one, is verified with `Get-AuthenticodeSignature`, and
   is attached to the GitHub release.

Free code signing for this project is provided by [SignPath.io](https://signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

## Verifying a download

On Windows:

```powershell
Get-AuthenticodeSignature .\GSpaceAnalyzerSetup.exe | Format-List Status, SignerCertificate
```

`Status` should be `Valid` and the signer should be SignPath Foundation. With the Windows SDK
installed, `signtool verify /pa /v GSpaceAnalyzerSetup.exe` prints the full certificate chain.

## Enabling signing on a fork

Fork maintainers need their own SignPath Foundation project. In the repository:

- Secret `SIGNPATH_API_TOKEN` — API token of a SignPath user with submitter permission.
- Variable `SIGNPATH_ORGANIZATION_ID` — the SignPath organization id.
- Optional variables, with defaults: `SIGNPATH_PROJECT_SLUG` (`g-spaceanalyzer`),
  `SIGNPATH_SIGNING_POLICY_SLUG` (`release-signing`), `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`
  (`release-installer`).

In SignPath, create an artifact configuration whose root is the uploaded ZIP archive and mark
`GSpaceAnalyzerSetup.exe` for Authenticode signing, then make it the one the slug above refers to.
Without these settings the workflow still builds and publishes an unsigned installer.

## Alternative: your own certificate

`installer/build-installer.ps1` accepts `-PfxPath` / `-PfxPassword` (or the `CODESIGN_PFX` and
`CODESIGN_PFX_PASSWORD` environment variables) and signs the app before it is embedded, then the
setup, timestamping and verifying both. Set the `CODESIGN_PFX` (base64 of the `.pfx`) and
`CODESIGN_PFX_PASSWORD` repository secrets to use it from the workflow instead of SignPath.
