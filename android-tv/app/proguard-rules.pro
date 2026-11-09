# The page calls these through the LibrarianTV JavaScript bridge; R8 cannot see those calls.
-keepclassmembers class net.dastari.librarian.tv.TvBridge {
    @android.webkit.JavascriptInterface <methods>;
}
-keepattributes JavascriptInterface
