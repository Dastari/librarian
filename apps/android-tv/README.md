# Librarian TV device-test app

This is an installable Android TV / Google TV **debug prototype**. It opens your existing
Librarian frontend in an Android WebView, with a native server-address screen, TV launcher
assets, remote controls, fullscreen video support, and connection recovery. It uses the
server's existing cookie authentication and HTML video/hls.js player.

You can install the APK directly on your own TV. No Google Play publication, developer
account, root access, or bootloader unlock is needed for ADB installation.
[Android device testing](https://developer.android.com/training/tv/get-started/create)

## What this build does

- Loads a user-selected root URL such as `https://library.example/` or `http://192.168.1.20:3000/`.
- Identifies itself as an Android TV client so the existing frontend selects TV input mode.
- Keeps login and GraphQL/media requests on the selected server's origin. Credentials stay
  in WebView's cookie store; there is no JavaScript-to-native credential bridge.
- Sends D-pad/Select through WebView and handles media play/pause and ten-second seek keys.
- Uses Back to close fullscreen/overlays or return through page history. At the root, Back
  opens an app menu. **Hold Back for 0.7 seconds**, or press Menu, to reload/change server/exit.
- Pauses media when the app is backgrounded; does not automatically resume it on return.
- Keeps the display awake while media is playing and flushes persistent cookies on page load/pause.
- Allows HTTP for local servers; refuses TLS certificate errors, mixed HTTP content on an
  HTTPS site, and top-level navigation to other origins.

The frontend is **hosted on your server**, not bundled into this APK. Server frontend updates
therefore reach the TV on reload. This is the first Android feasibility build from the
[TV plan](../../docs/tv-applications-plan.md), not the completed multi-platform release.
The shell uses Java and platform Android APIs to keep this prototype's dependencies small.
Native Media3 playback, packaged frontend assets, voice/Cast integration, downloads, and
4K/HDR/passthrough qualification are still pending. Browser/PWA behavior inside the hosted
site remains the server frontend's behavior.

Install minimum: Android 8 / API 26. This is not a tested compatibility floor for the frontend:
the TV also needs a sufficiently recent Android System WebView. Tailwind 4's Chrome baseline
is 111, and device playback must be tested independently.
[Tailwind browser support](https://tailwindcss.com/docs/compatibility)
The app menu displays the actual WebView version.

## Install the built APK on your TV

The build produces `app/build/outputs/apk/debug/app-debug.apk` relative to this directory.
Copy/download it to a computer on the same network as your TV and optionally rename it to
`librarian-google-tv-debug.apk`. The commands below use that name.

### 1. Prepare the computer and TV

Download and extract Google's [SDK Platform Tools](https://developer.android.com/tools/releases/platform-tools)
for Windows, macOS, or Linux. You only need these tools to install the APK; Android Studio
and the build SDK are unnecessary if using the APK already built here.

On Google TV, open **Settings → System → About → Android TV OS build**, then select the build
entry repeatedly (usually seven times) until developer mode is enabled. Return to **System →
Developer options**. Menu names vary by manufacturer. Older Android TV devices may place
About under Device Preferences. Enable the debugging option supported by your device.

Run the following commands from the extracted `platform-tools` directory. In Windows
PowerShell use `./adb.exe` in place of `./adb`; quoted APK paths can be absolute paths.

### 2. Connect with the method your TV provides

**If Developer options has Wireless debugging and “Pair using pairing code”:** enable it
on your trusted home network. Open the pairing-code screen and run:

```sh
./adb pair TV_IP:PAIRING_PORT
```

Enter the code displayed on the TV. Return to the main Wireless debugging screen and use
its **connection** IP/port, which is usually different from the pairing port:

```sh
./adb connect TV_IP:CONNECTION_PORT
./adb devices
```

Google documents wireless pairing for TVs on Android 13 and later, but availability depends
on the device. Use the menu actually provided by your TV.
[Wireless ADB instructions](https://developer.android.com/tools/adb#connect-to-a-device-over-wi-fi)

**If your TV instead provides Network debugging, or exposes network ADB after enabling USB
debugging:** use the TV's displayed debugging address/port. Some models use port 5555:

```sh
./adb connect TV_IP:5555
./adb devices
```

Approve the computer's debugging authorization on the TV. Port 5555 is a device-dependent
option, not a universal Google TV feature. If it refuses the connection, use wireless pairing
if available or the manufacturer's supported USB debugging connection. Do not assume every
TV USB port can connect to a computer for debugging. If neither method exists, APK installation
through a TV file manager/USB drive is an alternative; that installer needs the TV's
“Install unknown apps” permission. Exact steps depend on the TV model.

If `adb devices` reports `unauthorized`, accept the prompt on the TV. If multiple devices
are listed, add `-s DEVICE_SERIAL` immediately after `adb` to each install/shell command.

### 3. Install and launch

```sh
./adb install -r "/path/to/librarian-google-tv-debug.apk"
./adb shell am start -n org.librarian.tv.debug/org.librarian.tv.MainActivity
```

The app is named **Librarian TV** and declares a TV launcher activity. It may appear in your
Apps list without being pinned to the home-screen favourites. Launching with the command
above also works if the launcher has not refreshed. `install -r` upgrades an existing copy
while retaining its data, provided both APKs use the same signing key.
[ADB installation](https://developer.android.com/tools/adb#move)

On first launch, enter the URL you normally use to open Librarian, then sign in using the
normal login screen. Use the **frontend URL**, not a backend-only port that does not serve
the web app. `localhost` and `127.0.0.1` on the TV mean the TV itself, not your server.
For a LAN server, use its LAN IP/hostname and port. For an HTTP-to-HTTPS redirect, enter the
final HTTPS address directly. Root deployments are supported; `/librarian` proxy subpaths
and external SSO redirects are not supported by this prototype.

The computer is needed for installation/debugging, not normal playback. Once installed,
the TV connects directly to the Librarian server. Do not expose ADB to the internet; turn
off debugging when you finish testing. The debug APK enables WebView inspection for testing.

## First device test

Use a test account/library if possible; this client uses the actual selected server and
playback writes real progress. Record the TV model, Android version, firmware, WebView
version (hold Back), app version, and server frontend version.

1. Open the app from the TV launcher, enter the server, and sign in using only the remote.
2. Navigate Home → library → movie/show → episode; try search, dialogs, and Back.
3. Play a known H.264/AAC MP4, pause/resume, seek forward/back, and leave/reopen the player.
4. Play a file requiring HLS/remux/transcode; let it run past 15 minutes to exercise renewal.
5. Check saved progress from another Librarian client; verify subtitle/audio selection where supported.
6. Press Home during playback: audio should stop. Return and resume explicitly; repeat after standby.
7. Briefly disconnect the network, then restore it. Check recovery and saved sign-in.
8. Hold Back → Reload. Then sign out, close/relaunch, and confirm the account stays signed out.
9. Check Change server clears local credentials/site storage and does not show the old account.

Changing servers clears the app's local sign-in, but does not itself revoke the server's
session. Sign out first if you want to revoke that session. Avoid treating untested HEVC,
HDR, multichannel audio, and vendor-specific subtitle behavior as failures of the baseline test.
If the UI renders incorrectly, check/update Android System WebView through the TV's available
system/app updates before investigating layout changes.

For diagnostics while connected:

```sh
./adb shell getprop ro.product.model
./adb shell getprop ro.build.version.release
./adb shell dumpsys webviewupdate
./adb logcat -s AndroidRuntime chromium
```

Use Chrome's `chrome://inspect/#devices` on the connected computer to inspect this debug
WebView. Share the failing action and redacted error text; don't share cookies or credentials.
If installation says `INSTALL_FAILED_UPDATE_INCOMPATIBLE`, the installed APK uses another
signing key. Rebuild with the original key, or uninstall (which erases this app's local data):

```sh
./adb uninstall org.librarian.tv.debug
```

## Build from source

Pinned toolchain: JDK 17, Gradle 8.11.1 (checked wrapper distribution), Android Gradle Plugin
8.9.2, SDK platform 35, Build Tools 35.0.0. Install these with Android Studio's SDK Manager
or Android command-line tools and set `JAVA_HOME` / `ANDROID_HOME`. Keep SDK paths and signing
keys out of git. The toolchain follows [AGP 8.9 compatibility](https://developer.android.com/build/releases/agp-8-9-0-release-notes).

From this directory on a Linux cgroup-v2 build host:

```sh
./build.sh
```

This runs `assembleDebug testDebugUnitTest lintDebug` under a shared aggregate CPU quota of
80% of available CPU capacity. The build host must delegate a writable CPU-enabled cgroup
at `/sys/fs/cgroup/librarian-builds`, or set `LIBRARIAN_BUILD_CGROUP` to an equivalent shared
build group. The launcher fails rather than silently running uncapped. Gradle's worker limit
alone is not a CPU cap. Keep application servers outside that group.

On other hosts, use `gradlew.bat` (Windows) or `./gradlew` (macOS/Linux) with the same tasks
inside a build VM/container or externally managed aggregate CPU quota capped at 80%. Android
Studio can open this directory; its builds need the same host resource limit. No frontend
bundle rebuild or backend startup is required to build this hosted prototype.

Debug application ID: `org.librarian.tv.debug`. Android's standard debug keystore signs the
APK. Preserve that local keystore for repeatable device upgrades; don't commit or distribute
it. The `release` variant is unsigned and has not been prepared for distribution.

Build/test evidence and physical-device results belong in [the TV plan](../../docs/tv-applications-plan.md).
