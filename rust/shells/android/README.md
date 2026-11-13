# Android live wallpaper

The Android shell is a `WallpaperService` in Kotlin over the shared Rust
player. The wallpaper gets an `android.view.Surface` from the system; JNI hands
`ANativeWindow_fromSurface` to `attractor-gpu`, which creates the same wgpu swap
chain the desktop shells use (ADR-2 in the design document). There is no second
renderer and no raw EGL code in this shell.

| Path | Contents |
| --- | --- |
| `app/src/main/kotlin/…/AttractorWallpaperService.kt` | Engine lifecycle, one rendering thread per visible surface, 60 Hz loop |
| `app/src/main/kotlin/…/NativeBridge.kt` | The six JNI entry points |
| `app/src/main/kotlin/…/SettingsActivity.kt` | Attractor picker, also the system's live-wallpaper settings screen |
| `app/src/main/kotlin/…/Preferences.kt` | The pinned attractor id in `SharedPreferences` |
| `build-native.sh` | `cargo ndk` build of `attractor-ffi` into `app/src/main/jniLibs/` |

The JNI names are spelled out in `rust/crates/attractor-ffi/src/android.rs`;
`cargo test -p attractor-ffi` compares the two sides and fails on a typo, which
would otherwise surface only as an `UnsatisfiedLinkError` on a device.

## Build

Prerequisites: JDK 17, Android SDK with `platforms;android-35` and
`build-tools;35.0.0`, the NDK, `cargo-ndk`, and the Rust targets
`aarch64-linux-android` and `x86_64-linux-android`.

```bash
rustup target add aarch64-linux-android x86_64-linux-android
cargo install --locked cargo-ndk
export ANDROID_NDK_HOME=/path/to/ndk

./build-native.sh                 # arm64-v8a and x86_64
./gradlew assembleDebug lint
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

`./build-native.sh arm64-v8a` builds one ABI; the emulator needs `x86_64`.
Without a built library the APK still assembles but the wallpaper fails at
`System.loadLibrary`; CI always builds the libraries first.

The debug APK is unsigned for release. Signing, versioning and Play/F-Droid
packaging belong to I12.

## Settings

The picker offers the validated catalog (the same list the player draws from)
plus "Random each time". The choice is stored under the `attractor` key in the
`settings` preferences file — the same key and the same "empty means random"
rule as the desktop `settings.json`. An unknown id falls back to a random
attractor, so a removed attractor never breaks the wallpaper.

The system wallpaper picker opens the activity through
`android:settingsActivity` in `res/xml/wallpaper.xml`; the launcher icon opens
the same screen with a "Set as wallpaper" button. A change applies the next
time the wallpaper engine starts (re-apply the wallpaper or lock/unlock).

## Manual checklist

Run once on the emulator and once on the friend's phone at the milestone:

1. Install, open the app, pick a named attractor, "Set as wallpaper": the
   attractor animates and shows its formula overlay.
2. Pick "Random each time", re-apply: a different attractor appears.
3. Rotate the device: the picture keeps running and fills the new surface.
4. Home, open another app, return: rendering stops while invisible and resumes
   without recreating a broken surface.
5. Lock and unlock: the wallpaper restarts cleanly.
6. Change the attractor to a stale id in `SharedPreferences` (or reinstall a
   build without it): the wallpaper still starts with a random attractor.
7. `adb logcat -s AttractorWallpaper`: no repeating errors during a few
   minutes of rendering.
