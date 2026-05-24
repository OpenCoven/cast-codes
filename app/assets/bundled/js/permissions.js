// Permission-denial shim for the CastCodes embedded browser pane.
//
// Why this exists: wry 0.38 hardcodes WKPermissionDecisionGrant for the
// media-capture path on macOS (`wkwebview/mod.rs:798` calls
// `decision_handler.call((1,))`) with no override hook. WKWebView-level
// deny isn't reachable from our code without forking wry. The fallback
// is to override the JS-level entry points so page scripts get a
// rejection before the request reaches WebKit.
//
// Coverage:
//   navigator.mediaDevices.getUserMedia   → Promise.reject(NotAllowedError)
//   navigator.mediaDevices.getDisplayMedia → Promise.reject(NotAllowedError)
//   navigator.mediaDevices.enumerateDevices → Promise.resolve([])
//   navigator.geolocation.getCurrentPosition → call error cb with PERMISSION_DENIED
//   navigator.geolocation.watchPosition   → return a fake id, never fire
//   navigator.permissions.query(name)     → resolve {state: "denied"}
//   Notification.requestPermission()      → resolve "denied"
//   Notification.permission               → "denied" (read-only override)
//
// Not covered (out of scope for v0; tracked as follow-ups):
//   - <iframe allow="camera"> sub-frames create their own JS contexts
//     with their own `navigator`. The shim runs in the top frame only.
//   - Service-worker push subscriptions, Web Bluetooth, WebUSB, MIDI,
//     Web Serial, and other permission-gated APIs.
//   - Picture-in-Picture, screen wake lock.
//
// This shim is a defense-in-depth measure layered on the WKWebView
// behavior. The upstream fix is a wry change that exposes the
// permission decision handler so we can deny at the WebKit boundary.

(function () {
    "use strict";

    if (window.__castcodes_permissions_installed__) {
        return;
    }
    window.__castcodes_permissions_installed__ = true;

    function deniedError() {
        // DOMException with name "NotAllowedError" is what real
        // permission rejections look like — pages branch on err.name.
        try {
            return new DOMException(
                "Permission denied by CastCodes browser pane policy",
                "NotAllowedError"
            );
        } catch (_e) {
            const err = new Error("NotAllowedError");
            err.name = "NotAllowedError";
            return err;
        }
    }

    function warn(api) {
        try {
            console.warn("[castcodes] permission denied:", api);
        } catch (_e) { /* noop */ }
    }

    // --- mediaDevices ---
    if (
        typeof navigator !== "undefined"
        && navigator.mediaDevices
    ) {
        const md = navigator.mediaDevices;
        try {
            Object.defineProperty(md, "getUserMedia", {
                configurable: true,
                writable: true,
                value: function () {
                    warn("getUserMedia");
                    return Promise.reject(deniedError());
                },
            });
        } catch (_e) { /* noop */ }
        try {
            Object.defineProperty(md, "getDisplayMedia", {
                configurable: true,
                writable: true,
                value: function () {
                    warn("getDisplayMedia");
                    return Promise.reject(deniedError());
                },
            });
        } catch (_e) { /* noop */ }
        try {
            Object.defineProperty(md, "enumerateDevices", {
                configurable: true,
                writable: true,
                value: function () {
                    // Empty list — don't leak the user's hardware fingerprint.
                    return Promise.resolve([]);
                },
            });
        } catch (_e) { /* noop */ }
    }

    // --- geolocation ---
    if (
        typeof navigator !== "undefined"
        && navigator.geolocation
    ) {
        const geo = navigator.geolocation;
        const fakePosError = {
            code: 1, // PERMISSION_DENIED
            PERMISSION_DENIED: 1,
            POSITION_UNAVAILABLE: 2,
            TIMEOUT: 3,
            message: "Permission denied by CastCodes browser pane policy",
        };
        try {
            geo.getCurrentPosition = function (_success, error) {
                warn("geolocation.getCurrentPosition");
                if (typeof error === "function") {
                    try { error(fakePosError); } catch (_e) { /* noop */ }
                }
            };
        } catch (_e) { /* noop */ }
        try {
            geo.watchPosition = function (_success, error) {
                warn("geolocation.watchPosition");
                if (typeof error === "function") {
                    try { error(fakePosError); } catch (_e) { /* noop */ }
                }
                return 0;
            };
        } catch (_e) { /* noop */ }
        try {
            geo.clearWatch = function () { /* noop */ };
        } catch (_e) { /* noop */ }
    }

    // --- permissions API ---
    if (
        typeof navigator !== "undefined"
        && navigator.permissions
        && typeof navigator.permissions.query === "function"
    ) {
        const deniedSet = new Set([
            "camera",
            "microphone",
            "geolocation",
            "notifications",
            "persistent-storage",
            "midi",
            "background-sync",
            "ambient-light-sensor",
            "accelerometer",
            "gyroscope",
            "magnetometer",
            "clipboard-read",
            "clipboard-write",
            "display-capture",
        ]);
        try {
            navigator.permissions.query = function (descriptor) {
                const name = descriptor && descriptor.name;
                if (deniedSet.has(name)) {
                    return Promise.resolve({
                        state: "denied",
                        name: name,
                        onchange: null,
                        addEventListener: function () {},
                        removeEventListener: function () {},
                    });
                }
                // Unknown permission name: return a denied status
                // defensively. Web Platform spec says unknown names
                // should reject, but conservative deny is safer here.
                return Promise.resolve({
                    state: "denied",
                    name: name || "unknown",
                    onchange: null,
                    addEventListener: function () {},
                    removeEventListener: function () {},
                });
            };
        } catch (_e) { /* noop */ }
    }

    // --- Notification API ---
    if (typeof Notification !== "undefined") {
        try {
            Notification.requestPermission = function (cb) {
                warn("Notification.requestPermission");
                if (typeof cb === "function") {
                    try { cb("denied"); } catch (_e) { /* noop */ }
                }
                return Promise.resolve("denied");
            };
        } catch (_e) { /* noop */ }
        try {
            Object.defineProperty(Notification, "permission", {
                configurable: true,
                get: function () { return "denied"; },
            });
        } catch (_e) { /* noop */ }
    }
})();
