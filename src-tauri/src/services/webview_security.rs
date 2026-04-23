//! Webview security hardening — equivalent of C# `MainController.SetWebviewSettings()`.
//!
//! # Why
//!
//! Both webviews host JS code the app didn't write (React bundle, passbolt-browser-extension,
//! openpgp). If any of that JS — or a compromised dependency — tries to abuse browser features
//! (devtools, context menu inspection, new windows, password autofill), the native layer must
//! block it. CSP covers sub-resource loads; this module covers browser-level features.
//!
//! # How
//!
//! Called from `webview_service::create_webviews` after each webview is built, using the
//! pattern: `visible(false) → build → harden() → show()`. This ensures no frame loads
//! with default (insecure) settings before hardening completes.
//!
//! Each platform has its own function because WKWebView (macOS), WebView2 (Windows), and
//! WebKitGTK (Linux) expose different APIs with different security surfaces.
//!
//! # Mapping C# → Rust
//!
//! | C# (`SetWebviewSettings`)                | macOS (WKWebView)                                | Windows (WebView2)                            |
//! |-------------------------------------------|--------------------------------------------------|-----------------------------------------------|
//! | `AreDevToolsEnabled = false`              | `.devtools(false)` on builder                    | `SetAreDevToolsEnabled(false)`                |
//! | `IsSwipeNavigationEnabled = false`        | `setAllowsBackForwardNavigationGestures(false)`  | `SetIsSwipeNavigationEnabled(false)`          |
//! | `IsPasswordAutosaveEnabled = false`       | N/A (WKWebView has no autofill)                  | `SetIsPasswordAutosaveEnabled(false)`         |
//! | `AreHostObjectsAllowed = false`           | N/A (WebView2-specific COM interop)              | `SetAreHostObjectsAllowed(false)`             |
//! | `AreDefaultScriptDialogsEnabled = false`  | N/A (alert/confirm go to WKUIDelegate)           | `SetAreDefaultScriptDialogsEnabled(false)`    |
//! | `AreDefaultContextMenusEnabled = false`   | `class_replaceMethod` on `WKContentView`         | `SetAreDefaultContextMenusEnabled(false)`     |
//! | `AreBrowserAcceleratorKeysEnabled = false` | JS polyfill (`keyboardSecurityPolyfill.js`)      | `SetAreBrowserAcceleratorKeysEnabled(false)`  |
//! | `NewWindowRequested` handler              | `on_new_window(Deny)` on builder                 | `add_NewWindowRequested` event handler        |

use log::debug;
use tauri::WebviewWindow;

/// Apply security-hardened settings to a webview after creation.
///
/// Must be called before `show()` — the webview should be created with
/// `visible(false)` to prevent a brief window where default settings are active.
///
/// # Errors
///
/// Returns an error string if `with_webview` fails (e.g., webview not yet
/// initialized). The caller should decide whether to fail hard or log a warning.
pub fn harden(window: &WebviewWindow) -> Result<(), String> {
    let label = window.label().to_string();
    debug!("[Security] Hardening webview '{}'", label);

    window
        .with_webview(move |webview| {
            #[cfg(target_os = "macos")]
            apply_macos_settings(&webview, &label);

            #[cfg(windows)]
            apply_windows_settings(&webview, &label);

            #[cfg(target_os = "linux")]
            apply_linux_settings(&webview, &label);
        })
        .map_err(|error| format!("harden({}): with_webview failed: {}", window.label(), error))?;

    Ok(())
}

// ── macOS (WKWebView) ──────────────────────────────────────────
//
// WKWebView has a smaller security surface than WebView2:
// - No password autofill (WKWebView doesn't offer it for custom schemes)
// - No host objects (WebView2 COM concept, doesn't exist on Apple)
// - No browser accelerator keys (F12/Ctrl+U don't open devtools on macOS)
// - Script dialogs (alert/confirm) are handled by WKUIDelegate, not a setting
//
// What we CAN control natively:
// - Back/forward swipe gestures (touchpad two-finger swipe)
// - Magnification (pinch-to-zoom)
// - Link preview (force-touch popup)
// - Context menu (via class_replaceMethod on WKContentView)
//
// DevTools are controlled via `.devtools()` on the builder (not here).
// Keyboard shortcuts are blocked by keyboardSecurityPolyfill.js (cross-platform).

#[cfg(target_os = "macos")]
fn apply_macos_settings(webview: &tauri::webview::PlatformWebview, label: &str) {
    unsafe {
        // PlatformWebview.inner() returns a *mut c_void pointing to the WKWebView.
        // Cast via .cast() as shown in the official Tauri docs:
        // https://docs.rs/tauri/2.0.0/tauri/webview/struct.WebviewWindow.html#method.with_webview
        let wkwebview: &objc2_web_kit::WKWebView = &*webview.inner().cast();

        // Disable back/forward navigation gestures (two-finger swipe on trackpad).
        // Without this, a user could accidentally swipe back during the MFA flow
        // or navigate away from the current page in the workspace.
        // Equivalent of C# IsSwipeNavigationEnabled = false.
        wkwebview.setAllowsBackForwardNavigationGestures(false);

        // Disable pinch-to-zoom. The app has fixed layout — zooming breaks the UI
        // and could reveal hidden elements or cause layout issues.
        wkwebview.setAllowsMagnification(false);

        // Disable link preview on force-touch. Prevents macOS from showing a
        // popup preview of links, which would bypass the app's navigation guard.
        wkwebview.setAllowsLinkPreview(false);

        // Disable native context menu (Copy/Paste/Inspect Element).
        // Equivalent of C# AreDefaultContextMenusEnabled = false.
        disable_wkcontent_view_context_menu();

        debug!("[Security/{}] macOS WKWebView settings applied", label);
    }
}

/// Disable the native context menu by replacing `menuForEvent:` and
/// `willOpenMenu:withEvent:` directly on the private `WKContentView` class.
///
/// # Why target the class by name instead of traversing subviews?
///
/// WKWebView's context menu ("Reload", "Inspect Element") originates from
/// `WKContentView`, a private Apple class created lazily when content loads.
/// At `harden()` time the subview doesn't exist yet, so traversing the view
/// hierarchy would miss it. By targeting the class itself via `objc_getClass`,
/// the replacement applies to all current and future instances — including
/// those created after `harden()` returns.
///
/// # Why the JS contextmenu event still works
///
/// The JS `contextmenu` event and the native NSMenu are independent systems.
/// `menuForEvent:` controls the Cocoa NSMenu popup. The JS event fires from
/// WebKit's event dispatch before the NSMenu step.
///
/// # Safety
///
/// `class_replaceMethod` on a private class is standard ObjC runtime usage.
/// Called once via `OnceLock` — idempotent and thread-safe.
#[cfg(target_os = "macos")]
unsafe fn disable_wkcontent_view_context_menu() {
    use objc2::runtime::{AnyObject, Sel};
    use objc2::sel;
    use std::sync::Once;

    static SWIZZLE_ONCE: Once = Once::new();

    SWIZZLE_ONCE.call_once(|| {
        // Get the private WKContentView class by name.
        // objc_getClass returns *const, but class_replaceMethod needs *mut.
        let class_name = c"WKContentView";
        let class = objc2::ffi::objc_getClass(class_name.as_ptr()) as *mut objc2::runtime::AnyClass;
        if class.is_null() {
            log::warn!("[Security] WKContentView class not found — context menu not disabled");
            return;
        }

        // Replace menuForEvent: → return nil (no menu created)
        extern "C-unwind" fn no_menu(
            _this: &AnyObject,
            _sel: Sel,
            _event: *mut AnyObject,
        ) -> *mut AnyObject {
            std::ptr::null_mut()
        }

        #[allow(clippy::missing_transmute_annotations)]
        objc2::ffi::class_replaceMethod(
            class,
            sel!(menuForEvent:),
            std::mem::transmute::<_, objc2::runtime::Imp>(
                no_menu as extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
            ),
            c"@@:@".as_ptr(),
        );

        // Replace willOpenMenu:withEvent: → no-op (belt-and-suspenders)
        extern "C-unwind" fn no_will_open_menu(
            _this: &AnyObject,
            _sel: Sel,
            _menu: *mut AnyObject,
            _event: *mut AnyObject,
        ) {}

        #[allow(clippy::missing_transmute_annotations)]
        objc2::ffi::class_replaceMethod(
            class,
            sel!(willOpenMenu:withEvent:),
            std::mem::transmute::<_, objc2::runtime::Imp>(
                no_will_open_menu as extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject, *mut AnyObject),
            ),
            c"v@:@@".as_ptr(),
        );

        debug!("[Security] WKContentView context menu disabled (class-level swizzle)");
    });
}

// Windows and Linux hardening will be added when those platforms are ported.
// See the C# mapping table in the module doc for the WebView2 settings to implement.
