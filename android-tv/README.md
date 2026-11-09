# Librarian for Android TV

A small Android TV app that shows the Librarian web app from your server, full screen and driven
by the remote. It is one Activity with a WebView (no AndroidX, no native code), so it installs on
any Android TV from Android 7 (API 24), including 32-bit-only budget TVs. Design notes:
`docs/design.md` Q137 (the shell), Q138 (finding the server), Q139 (budget TVs), Q140 (the native
player planned next).

- **Package:** `net.dastari.librarian.tv` (debug builds: `net.dastari.librarian.tv.debug`, so both can be installed)
- **Version:** `version.properties` is the only place it is set. `versionCode` is
  `major × 10000 + minor × 100 + patch`, so raise `versionName` for every release.

## What it does

- **First launch:** a server screen lists the Librarian servers it finds on the network (mDNS
  `_librarian._tcp`, announced by the backend) and has an address field for the rest. A bare
  address such as `192.168.1.20` is tried on ports 3001 and 3000, then on plain http.
- **Changing the server later:** hold **Back** or **Menu** on the remote, or open
  *Settings › About › TV app › Change server* in the web app.
- **Remote:**
  - The arrows and OK drive the web app's TV mode.
  - **Back** closes an open dialog or menu, then goes back in the app. It leaves the app only from Home.
  - **Menu** opens the focused item's actions.
  - **Media buttons, Info and Guide** reach the page as standard key events (`MediaPlayPause`,
    `MediaTrackNext`, `Info`…; the full table is in design.md Q137).
  - **Volume keys** are left to Android.
- **Screen:** the page is laid out 1920 CSS px wide on any TV, by rewriting its viewport.
  *Settings › General › Display size* (80–120 %) makes everything larger or smaller, on this TV
  only, without a reload.
- **Volume:** the player's volume slider and mute are the TV's own volume, and follow the remote.
  Where the TV's volume is fixed (or set by a sound system), the player shows a note instead of the
  slider.
- **Staying signed in:** cookies are kept by the WebView, so sign-in survives restarts.
- **Plain http:** allowed, as LAN servers rarely have certificates.
- **While media plays:** the screen is kept on.
- **If the page breaks:** a crashed renderer is replaced and the page reloaded.
- **If the server is unreachable** (or restarting): "Reconnecting to …" with *Try again* and
  *Change server*. It retries after 2, 4, 8 and 16 s, then every 30 s, and goes away as soon as a
  load succeeds.

## Server side

- The backend announces itself unless *Settings › General › Network › Announce on the local
  network* is off.
- *Address for TV apps* sets the address it announces. Its seed comes from
  `LIBRARIAN_PUBLIC_WEB_URL`, and `LIBRARIAN_LAN_ADVERTISE=false` seeds the switch off.
- Left empty, it is `http://<LAN address>:<port>`: the backend's port in a release build (which
  serves the web app), or 3000 for a development backend behind Vite.
- If cookies are `Secure` (`LIBRARIAN_SECURE_COOKIES`, an https site), set the https address: an
  http page cannot keep a `Secure` cookie, so sign-in would not stick.

## Build

Needs JDK 17 and the Android SDK (platform 37, build-tools 37.0.0) in `/opt/android-sdk`.

```sh
android-tv/build.sh
```

`build.sh` does the following:

- It runs Gradle under the shared build lock with a CPU and memory cap (`/root/.librarian-build.lock`, 320 % CPU, 6 GB).
- The Gradle settings are `-Xmx2g`, 2 workers, no Gradle or Kotlin daemon.
- It runs the unit tests and builds both APKs:
  - `app/build/dist/librarian-tv-<version>.apk`: release, minified and signed;
  - `app/build/dist/librarian-tv-<version>-debug.apk`: debug, with WebView remote debugging on.
- It copies the release APK to `/root/librarian-android-dist/` and prints its size and SHA-256.

By hand: `ANDROID_HOME=/opt/android-sdk ./gradlew --no-daemon testDebugUnitTest distApks`.

To regenerate the icons and the TV banner from the brand art, run `pnpm pwa-assets --android-tv` in `web/`.

## Signing

The release key never enters the repository.

- **Key:** `/root/.config/librarian/android-tv-release.jks` (PKCS12, alias `librarian-tv`, RSA 4096, valid to 2126), root-only.
- **Passwords:** `/root/.config/librarian/android-tv-keystore.properties` (`storeFile`, `storePassword`, `keyAlias`, `keyPassword`), root-only.
  - Another location can be given with `LIBRARIAN_TV_KEYSTORE_PROPERTIES`.
- **Without the properties file:** release builds are signed with the debug key. They install, but cannot update a properly signed install, and the other way round.
- **Certificate SHA-256:**
  `8F:8C:52:96:C3:7C:C2:49:7B:30:42:00:83:5C:92:D2:60:78:50:E2:66:C8:CB:66:0E:32:69:49:9F:26:00:AA`

Check an APK's signer:

```sh
/opt/android-sdk/build-tools/37.0.0/apksigner verify --print-certs librarian-tv-<version>.apk
```

Keep a copy of the keystore and its properties file somewhere safe. Without them no later version
can install over this one, and users would have to uninstall, losing the app's data and sign-in.

## Install with adb

On the TV, turn on *Developer options › USB debugging* (or *Network debugging*), then:

```sh
adb connect <tv-ip>:5555                 # network debugging
adb install -r librarian-tv-<version>.apk
adb shell monkey -p net.dastari.librarian.tv -c android.intent.category.LEANBACK_LAUNCHER 1
```

- **Updates:** the same `adb install -r` with a newer APK keeps the server and sign-in.
- **To forget the server:** `adb shell pm clear net.dastari.librarian.tv`. This also forgets the sign-in.
- **Debugging the page:** use the debug APK, then open `chrome://inspect` on a computer connected with adb.
