# Inno Setup installer script

## What is `windows-installer.iss`?

On Windows, programs are conventionally installed using an installer, also known as an installation wizard.
The installer is a single executable that takes care of:
* Creating a directory to store the program's files
* Downloading assets
* Initializing registry entries
* Creating a desktop icon
* ... and more, depending on the application's needs.


`windows-installer.iss` is an **Inno Setup script**:
a configuration file for building a Warp installer.
The Inno Setup Compiler takes a script file and generates an installer executable.
This is roughly equivalent to the bundling process on MacOS.


## How to edit the installer

See the Inno Setup documentation: [Inno Setup Help](https://jrsoftware.org/ishelp/).
This script can be edited manually using any code editor.
However, it requires the Inno Setup compiler to be turned into a `.exe` file.


## How to compile this installer

First, ensure you've set up your environment.
* Download and install the [Inno Setup Compiler](https://jrsoftware.org/isdl.php).
* Run `cargo build` to ensure the installer uses the latest version of Warp.

### Option 1: Use the CLI
1. Add the Inno Setup Command-line Compiler executable to your shell path.
By default, it is located at `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`.
2. Compile the installer:
```shell
iscc .\script\windows\windows-installer.iss
```
3. Run the generated executable:
```shell
.\script\windows\Output\Warp-Windows-Setup.exe
```

The script begins with a series of preprocessor definitions.
From the command line, use the `/D` flag to emulate preprocessor definitions
and override the hardcoded defaults.
Usage: `iscc <script path> /D<name>[=<value>]`

The following constants can be overwritten:
* `MyAppVersion` (default: `0.1.0`)
* `MyAppExeName` (default: `warp.exe`)
* `ReleaseChannel` (default: `dev`)
* `TargetProfileDir` (default: `debug`)

### Option 2: Use the GUI
1. Open the Inno Setup application and select this script.
2. Click the "compile" button. This will generate an installer executable in a directory called `Output` at the same level as this script.
2. To run the installer, click the "run" button in Inno Setup.


## Code signing & Microsoft Defender SmartScreen

> **Artifact note:** CastCodes ships an **Inno Setup setup executable**
> (`script/windows/Output/<AppName>.exe`), *not* a Windows Installer `.msi`
> package. "Signed MSI" is a loose shorthand for this signed setup `.exe`.
> Everything below applies to that installer.

### What gets signed

When a sign-tool command is provided, Inno Setup Authenticode-signs the
**setup engine** and the **uninstaller** (`SignedUninstaller=yes`). The setup
engine extracts a temporary bootstrapper into `%TEMP%`; signing keeps that
bootstrapper Authenticode-signed so that Microsoft Defender's ASR rule
`D4F940AB` does not block the installer in enterprise environments (see the
comment above the `#ifdef SIGN_TOOL` block in `windows-installer.iss`).

Signing is **opt-in**:

* `bundle.ps1` accepts `-SIGN_TOOL_CMD` (alias `-sign-tool-cmd`) or reads the
  `SIGN_TOOL_CMD` environment variable.
* When set, it invokes the compiler as
  `iscc ... /DSIGN_TOOL=1 /Scodesign=<your command>`, which activates
  `SignTool=codesign` in the script.
* When empty (local dev builds), **the installer is built unsigned** and no
  signing step runs.

Build a signed installer locally by supplying a `signtool` command; Inno Setup
substitutes `$f` with each file to sign:

```powershell
$env:SIGN_TOOL_CMD = 'signtool.exe sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $f'
.\script\windows\bundle.ps1 -Channel stable -arch x64
```

### Fresh-certificate SmartScreen behavior

**A valid Authenticode signature is not the same as SmartScreen trust.**
Microsoft Defender SmartScreen keeps a *reputation* score per file hash and per
signing certificate, and it is earned over time and download volume — not
granted at signing.

* A **freshly issued** standard code-signing certificate — Individual
  Validation (**IV**) or Organization Validation (**OV**) — starts with little
  or no reputation. Until that certificate accrues reputation, SmartScreen shows
  the blue **"Windows protected your PC — Microsoft Defender SmartScreen
  prevented an unrecognized app from starting"** prompt **even though the
  installer is correctly signed.** This is the expected fresh-certificate
  experience for early releases.
* **Extended Validation (EV)** certificates no longer receive automatic
  SmartScreen reputation. Microsoft's current SmartScreen app-reputation
  guidance says reputation must build organically, so paying for EV solely to
  avoid SmartScreen prompts is not justified.
* **What users see and how to proceed:** click **More info → Run anyway**. The
  prompt disappears on its own once the certificate/file reputation is
  established.
* **Reputation is tied to the certificate.** Rotating or renewing to a new
  certificate restarts the reputation clock, so prefer certificate continuity
  across releases.

An **unsigned** installer produces a stronger warning ("unknown publisher") and
never accrues publisher reputation — so a fresh-cert SmartScreen prompt is
still a strict improvement over shipping unsigned.

### What the OSS release pipeline does

The `release_windows` job in `.github/workflows/create_release.yml` builds the
x64 installer, writes a `.sha256` checksum, publishes both to the GitHub
Release, uploads them as a workflow artifact, and **attests build provenance**
via `actions/attest`.

The OSS workflow **intentionally does not perform Azure/cloud code signing**
(see the header comment in `create_release.yml`). Signing, when done, is
supplied out-of-band through `SIGN_TOOL_CMD`. A default OSS CI build is
therefore **unsigned** and will trigger the "unknown publisher" SmartScreen
prompt described above.

## Using icons

Windows has its own icon file format that bundles together multiple icon sizes.
App icons are located in `app/channels/<channel_name>/icon/no-padding`.
The `.ico` files are generated using imagemagick:

```shell
convert 16x16.png 32x32.png 48x48.png 64x64.png 256x256.png icon.ico
```

Note that sizes above 256x256 are not supported.
See the [Inno Setup docs](https://jrsoftware.org/ishelp/index.php?topic=setup_setupiconfile).
