package org.librarian.tv;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.net.http.SslError;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.WindowManager;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputMethodManager;
import android.webkit.CookieManager;
import android.webkit.RenderProcessGoneDetail;
import android.webkit.SslErrorHandler;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebStorage;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

/** Device-test client: the hosted frontend keeps its existing same-origin auth and player. */
public final class MainActivity extends Activity {
    private final Handler handler = new Handler(Looper.getMainLooper());
    private FrameLayout root;
    private WebView browser;
    private View status;
    private View fullscreen;
    private WebChromeClient.CustomViewCallback fullscreenCallback;
    private SharedPreferences preferences;
    private String server;
    private boolean failed;
    private boolean resumed;
    private boolean backHeld;
    private AlertDialog menu;

    private final Runnable loadTimeout = () -> {
        if (browser != null) browser.stopLoading();
        showError();
    };
    private final Runnable longBack = () -> {
        backHeld = true;
        showMenu();
    };
    private final Runnable checkPlayback = new Runnable() {
        @Override public void run() {
            if (!resumed || browser == null) return;
            browser.evaluateJavascript(
                    "Array.from(document.querySelectorAll('video,audio')).some(m => !m.paused && !m.ended)",
                    result -> {
                        if (resumed && "true".equals(result)) {
                            getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
                        } else {
                            getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
                        }
                    });
            handler.postDelayed(this, 5000);
        }
    };

    @Override public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        preferences = getSharedPreferences("connection", MODE_PRIVATE);
        root = new FrameLayout(this);
        root.setBackgroundColor(Color.rgb(10, 10, 15));
        setContentView(root);
        immersive();
        server = preferences.getString("server", null);
        if (server == null) showConnection();
        else openServer();
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private LinearLayout panel() {
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setGravity(Gravity.CENTER_VERTICAL);
        panel.setPadding(dp(48), dp(24), dp(48), dp(24));
        panel.setBackgroundColor(Color.rgb(10, 10, 15));
        return panel;
    }

    private TextView text(int resource, int size) {
        TextView text = new TextView(this);
        text.setText(resource);
        text.setTextSize(size);
        text.setTextColor(Color.WHITE);
        text.setPadding(0, dp(6), 0, dp(6));
        return text;
    }

    private Button button(int resource, Runnable action) {
        Button button = new Button(this);
        button.setText(resource);
        button.setAllCaps(false);
        button.setOnClickListener(view -> action.run());
        return button;
    }

    private void setStatus(View view) {
        if (status != null) root.removeView(status);
        status = view;
        if (view != null) root.addView(view, new FrameLayout.LayoutParams(-1, -1));
        if (browser != null) browser.setVisibility(view == null ? View.VISIBLE : View.GONE);
    }

    private void showConnection() {
        handler.removeCallbacks(loadTimeout);
        destroyBrowser();
        LinearLayout panel = panel();
        panel.addView(text(R.string.server_title, 28));
        panel.addView(text(R.string.server_help, 18));
        EditText address = new EditText(this);
        address.setText(server == null ? "" : server);
        address.setHint(R.string.server_hint);
        address.setSingleLine(true);
        address.setTextColor(Color.WHITE);
        address.setTextSize(18);
        address.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        address.setImeOptions(EditorInfo.IME_ACTION_GO);
        address.setSelectAllOnFocus(true);
        panel.addView(address, new LinearLayout.LayoutParams(-1, dp(56)));
        Runnable connect = () -> {
            final String next;
            try {
                next = ServerAddress.normalize(address.getText().toString());
            } catch (IllegalArgumentException exception) {
                address.setError(exception.getMessage());
                return;
            }
            ((InputMethodManager) getSystemService(INPUT_METHOD_SERVICE))
                    .hideSoftInputFromWindow(address.getWindowToken(), 0);
            server = next;
            preferences.edit().putString("server", server).apply();
            openServer();
        };
        Button connectButton = button(R.string.connect, connect);
        panel.addView(connectButton);
        if (server != null) panel.addView(button(R.string.cancel, this::openServer));
        address.setOnEditorActionListener((view, action, event) -> {
            if (action != EditorInfo.IME_ACTION_GO) return false;
            connect.run();
            return true;
        });
        setStatus(panel);
        address.requestFocus();
    }

    @SuppressLint("SetJavaScriptEnabled") // Required by the user-selected Librarian SPA; no JavaScript interface is exposed.
    private void openServer() {
        destroyBrowser();
        failed = false;
        try {
            browser = new WebView(this);
        } catch (RuntimeException exception) {
            LinearLayout panel = panel();
            panel.addView(text(R.string.webview_unavailable, 22));
            panel.addView(button(R.string.close_app, this::finish));
            setStatus(panel);
            return;
        }
        WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG);
        WebSettings settings = browser.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
        settings.setJavaScriptCanOpenWindowsAutomatically(false);
        settings.setSupportMultipleWindows(false);
        settings.setMediaPlaybackRequiresUserGesture(false);
        settings.setUseWideViewPort(true);
        settings.setLoadWithOverviewMode(true);
        settings.setUserAgentString(settings.getUserAgentString() + " LibrarianTV/" + BuildConfig.VERSION_NAME + " Android TV");
        CookieManager.getInstance().setAcceptCookie(true);
        CookieManager.getInstance().setAcceptThirdPartyCookies(browser, false);
        browser.setBackgroundColor(Color.rgb(10, 10, 15));
        browser.setWebViewClient(new Client());
        browser.setWebChromeClient(new WebChromeClient() {
            @Override public void onShowCustomView(View view, CustomViewCallback callback) {
                if (fullscreen != null) {
                    callback.onCustomViewHidden();
                    return;
                }
                fullscreen = view;
                fullscreenCallback = callback;
                browser.setVisibility(View.GONE);
                root.addView(view, new FrameLayout.LayoutParams(-1, -1));
                view.requestFocus();
                immersive();
            }

            @Override public void onHideCustomView() {
                hideFullscreen();
            }
        });
        root.addView(browser, 0, new FrameLayout.LayoutParams(-1, -1));
        LinearLayout loading = panel();
        loading.addView(text(R.string.loading, 24));
        loading.addView(text(R.string.menu_hint, 18));
        setStatus(loading);
        handler.removeCallbacks(loadTimeout);
        handler.postDelayed(loadTimeout, 30000);
        browser.loadUrl(server);
        handler.removeCallbacks(checkPlayback);
        if (resumed) handler.post(checkPlayback);
    }

    private final class Client extends WebViewClient {
        @Override public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
            if (!request.isForMainFrame()) return false;
            if (ServerAddress.sameOrigin(server, request.getUrl().toString())) return false;
            Toast.makeText(MainActivity.this, R.string.outside_server, Toast.LENGTH_LONG).show();
            return true;
        }

        @Override public void onPageStarted(WebView view, String url, android.graphics.Bitmap favicon) {
            if (!ServerAddress.sameOrigin(server, url)) {
                view.stopLoading();
                showError();
            }
        }

        @Override public void onPageFinished(WebView view, String url) {
            if (view != browser || failed || !ServerAddress.sameOrigin(server, url)) return;
            handler.removeCallbacks(loadTimeout);
            setStatus(null);
            view.requestFocus();
            CookieManager.getInstance().flush();
        }

        @Override public void onReceivedError(WebView view, WebResourceRequest request, WebResourceError error) {
            if (view == browser && request.isForMainFrame()) showError();
        }

        @Override public void onReceivedHttpError(WebView view, WebResourceRequest request, WebResourceResponse error) {
            if (view == browser && request.isForMainFrame()) showError();
        }

        @Override public void onReceivedSslError(WebView view, SslErrorHandler sslHandler, SslError error) {
            sslHandler.cancel();
            if (view == browser) showError();
        }

        @Override public boolean onRenderProcessGone(WebView view, RenderProcessGoneDetail detail) {
            if (view == browser) {
                destroyBrowser();
                showError();
            }
            return true;
        }
    }

    private void showError() {
        failed = true;
        handler.removeCallbacks(loadTimeout);
        pauseMedia();
        hideFullscreen();
        LinearLayout panel = panel();
        panel.addView(text(R.string.connection_error, 22));
        Button retry = button(R.string.retry, this::openServer);
        panel.addView(retry);
        panel.addView(button(R.string.change_server, this::confirmChangeServer));
        setStatus(panel);
        retry.requestFocus();
    }

    private void hideFullscreen() {
        if (fullscreen == null) return;
        root.removeView(fullscreen);
        fullscreen = null;
        WebChromeClient.CustomViewCallback callback = fullscreenCallback;
        fullscreenCallback = null;
        if (callback != null) callback.onCustomViewHidden();
        if (browser != null && status == null) {
            browser.setVisibility(View.VISIBLE);
            browser.requestFocus();
        }
    }

    private void showMenu() {
        if (menu != null && menu.isShowing()) return;
        pauseMedia();
        String version = getString(R.string.app_name) + " " + BuildConfig.VERSION_NAME;
        android.content.pm.PackageInfo provider = WebView.getCurrentWebViewPackage();
        if (provider != null) version += " · WebView " + provider.versionName;
        menu = new AlertDialog.Builder(this).setTitle(version)
                .setItems(new CharSequence[] { getString(R.string.reload), getString(R.string.change_server), getString(R.string.close_app) },
                        (dialog, which) -> {
                            if (which == 0) { if (server != null) openServer(); else showConnection(); }
                            else if (which == 1) confirmChangeServer();
                            else finish();
                        })
                .setNegativeButton(R.string.cancel, null).create();
        menu.show();
    }

    private void confirmChangeServer() {
        new AlertDialog.Builder(this).setMessage(R.string.server_changed)
                .setPositiveButton(R.string.change_server, (dialog, which) -> {
                    if (browser != null) browser.clearCache(true);
                    destroyBrowser();
                    WebStorage.getInstance().deleteAllData();
                    // The renderer is destroyed before asynchronous cookie clearing; no old session can reconnect.
                    CookieManager.getInstance().removeAllCookies(removed -> {
                        CookieManager.getInstance().flush();
                        showConnection();
                    });
                }).setNegativeButton(R.string.cancel, null).show();
    }

    private void handleBack() {
        if (fullscreen != null) {
            hideFullscreen();
            return;
        }
        if (browser == null || status != null) {
            showMenu();
            return;
        }
        WebView current = browser;
        // Let React Aria close its active overlay before navigating browser history.
        current.evaluateJavascript("(() => {"
                + "const overlay = Array.from(document.querySelectorAll('[role=dialog],[role=alertdialog],[role=menu],[role=listbox]'))"
                + ".find(e => e.getClientRects().length && !e.closest('[aria-hidden=true],[inert]'));"
                + "if (!overlay) return false;"
                + "const target = overlay.contains(document.activeElement) ? document.activeElement : overlay;"
                + "target.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',code:'Escape',bubbles:true,cancelable:true}));"
                + "target.dispatchEvent(new KeyboardEvent('keyup',{key:'Escape',code:'Escape',bubbles:true,cancelable:true}));"
                + "return true; })()", result -> {
                    if (browser != current || "true".equals(result)) return;
                    if (current.canGoBack()) current.goBack();
                    else showMenu();
                });
    }

    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.getKeyCode() == KeyEvent.KEYCODE_BACK) {
            if (event.getAction() == KeyEvent.ACTION_DOWN && event.getRepeatCount() == 0) {
                backHeld = false;
                handler.postDelayed(longBack, 700);
            } else if (event.getAction() == KeyEvent.ACTION_UP) {
                handler.removeCallbacks(longBack);
                if (!backHeld && !event.isCanceled()) handleBack();
            }
            return true;
        }
        if (event.getKeyCode() == KeyEvent.KEYCODE_MENU) {
            if (event.getAction() == KeyEvent.ACTION_UP) showMenu();
            return true;
        }
        String mediaAction = switch (event.getKeyCode()) {
            case KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE -> "m.paused ? m.play().catch(() => {}) : m.pause()";
            case KeyEvent.KEYCODE_MEDIA_PLAY -> "m.play().catch(() => {})";
            case KeyEvent.KEYCODE_MEDIA_PAUSE, KeyEvent.KEYCODE_MEDIA_STOP -> "m.pause()";
            case KeyEvent.KEYCODE_MEDIA_FAST_FORWARD -> "m.currentTime = Math.min(m.duration || Infinity, m.currentTime + 10)";
            case KeyEvent.KEYCODE_MEDIA_REWIND -> "m.currentTime = Math.max(0, m.currentTime - 10)";
            default -> null;
        };
        if (mediaAction != null && browser != null && status == null) {
            if (event.getAction() == KeyEvent.ACTION_DOWN && event.getRepeatCount() == 0) {
                browser.evaluateJavascript("(() => { const m = document.querySelector('video') || document.querySelector('audio');"
                        + "if (m) { " + mediaAction + "; } })()", null);
            }
            return true;
        }
        // D-pad and Select remain trusted WebView key events, including text-input/IME behavior.
        return super.dispatchKeyEvent(event);
    }

    @SuppressWarnings("deprecation")
    @Override public void onBackPressed() {
        handleBack();
    }

    private void immersive() {
        getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_FULLSCREEN
                | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                | View.SYSTEM_UI_FLAG_LAYOUT_STABLE);
    }

    @Override protected void onResume() {
        super.onResume();
        resumed = true;
        immersive();
        if (browser != null) {
            browser.onResume();
            browser.evaluateJavascript("window.dispatchEvent(new Event('focus'))", null);
        }
        handler.removeCallbacks(checkPlayback);
        handler.post(checkPlayback);
    }

    @Override protected void onPause() {
        resumed = false;
        handler.removeCallbacks(checkPlayback);
        handler.removeCallbacks(longBack);
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        if (browser != null) {
            pauseMedia();
            browser.onPause();
            CookieManager.getInstance().flush();
        }
        super.onPause();
    }

    private void pauseMedia() {
        if (browser != null) browser.evaluateJavascript("document.querySelectorAll('video,audio').forEach(m => m.pause())", null);
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
    }

    private void destroyBrowser() {
        handler.removeCallbacks(loadTimeout);
        handler.removeCallbacks(checkPlayback);
        hideFullscreen();
        if (browser == null) return;
        WebView old = browser;
        browser = null;
        root.removeView(old);
        old.stopLoading();
        old.destroy();
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
    }

    @Override protected void onDestroy() {
        handler.removeCallbacksAndMessages(null);
        destroyBrowser();
        super.onDestroy();
    }
}
