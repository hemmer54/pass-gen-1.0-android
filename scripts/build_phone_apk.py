#!/usr/bin/env python3
"""Build and package an ARM64 phone APK from cargo-apk's release outputs."""

from __future__ import annotations

import subprocess
import sys
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
APK_DIR = ROOT / "target" / "release" / "apk"
SOURCE_APK = APK_DIR / "pass-gen-1.0-unaligned.apk"
NATIVE_LIBRARY = APK_DIR / "lib" / "arm64-v8a" / "libpass_gen_mobile.so"
OUTPUT_APK = APK_DIR / "pass-gen-1.0-phone-unaligned.apk"
LIBRARY_ENTRY = "lib/arm64-v8a/libpass_gen_mobile.so"


def main() -> int:
    APK_DIR.mkdir(parents=True, exist_ok=True)
    for generated_file in (SOURCE_APK, APK_DIR / "AndroidManifest.xml", NATIVE_LIBRARY):
        generated_file.unlink(missing_ok=True)

    result = subprocess.run(
        [
            "cargo",
            "apk",
            "build",
            "--release",
            "--target",
            "aarch64-linux-android",
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)

    if not SOURCE_APK.is_file() or not NATIVE_LIBRARY.is_file():
        print(
            "cargo-apk did not produce the release manifest/APK and ARM64 library. "
            "Check that the Android SDK, NDK, and Rust target are installed.",
            file=sys.stderr,
        )
        return result.returncode or 1

    output = (result.stdout or "") + (result.stderr or "")
    if result.returncode and "Configure a release keystore" not in output:
        print(
            "cargo-apk failed before its expected release-signing step; refusing to "
            "package potentially stale build output.",
            file=sys.stderr,
        )
        return result.returncode

    with zipfile.ZipFile(SOURCE_APK) as source, zipfile.ZipFile(
        OUTPUT_APK, "w"
    ) as output:
        if LIBRARY_ENTRY in source.namelist():
            print(f"{SOURCE_APK.name} already contains {LIBRARY_ENTRY}")
            return 1
        for entry in source.infolist():
            output.writestr(entry, source.read(entry.filename))
        output.write(NATIVE_LIBRARY, LIBRARY_ENTRY, compress_type=zipfile.ZIP_DEFLATED)

    with zipfile.ZipFile(OUTPUT_APK) as packaged:
        if packaged.testzip() is not None or LIBRARY_ENTRY not in packaged.namelist():
            print("The packaged APK failed ZIP/content verification.", file=sys.stderr)
            return 1

    print(f"Packaged ARM64 APK: {OUTPUT_APK}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
