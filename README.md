# Pass Gen 1.0

A Fallout-inspired Android password generator built with Rust and GPUI.

## Features

- Random password and passphrase generation
- Password strength indicator
- In-memory password history with clear-history controls
- Optional clipboard auto-clear after 30 seconds
- Bundled `Fixedsys Excelsior` and `JH_Fallout` fonts

## Checks

```bash
cargo fmt --all
cargo check
cargo test
```

## Build an ARM64 APK for a real phone

Install the Android SDK and NDK, the Rust target `aarch64-linux-android`, Python 3, and `cargo-apk`. Set `ANDROID_HOME` and `ANDROID_NDK_ROOT` for your SDK/NDK installation.

From the repository root, run:

```bash
python scripts/build_phone_apk.py
```

The helper builds the ARM64 native library, then packages it into the APK. The APK emitted by `cargo-apk` may omit that `.so` from the archive when release signing is not configured; the helper ensures it is present under `lib/arm64-v8a/` and validates the resulting ZIP.

Then align and sign the packaged APK using Android SDK Build Tools. For local testing only, the standard debug keystore can be used; use your own private release keystore for publishing and future production updates.

Example PowerShell commands (replace `36.0.0` with an installed Build Tools version):

```powershell
$bt = "$env:ANDROID_HOME\build-tools\36.0.0"
& "$bt\zipalign.exe" -f 4 `
  "target\release\apk\pass-gen-1.0-phone-unaligned.apk" `
  "target\release\apk\pass-gen-1.0-phone-aligned.apk"
& "$bt\apksigner.bat" sign `
  --ks "$env:USERPROFILE\.android\debug.keystore" `
  --ks-key-alias androiddebugkey `
  --ks-pass pass:android `
  --key-pass pass:android `
  --out "target\release\apk\pass-gen-1.0-phone.apk" `
  "target\release\apk\pass-gen-1.0-phone-aligned.apk"
& "$bt\apksigner.bat" verify --verbose `
  "target\release\apk\pass-gen-1.0-phone.apk"
```

Never commit keystores or signing passwords. APKs and Cargo build output are excluded by `.gitignore`.
