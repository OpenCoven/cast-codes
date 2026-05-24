// Find-in-page implementation for the CastCodes embedded browser pane.
//
// Loaded once per webview via `evaluate_script` and idempotent on re-load
// (clears prior state via window.__castcodes_find__.clear if present).
//
// Exposes window.__castcodes_find__:
//   setQuery(query: string): sets the search and highlights all matches,
//     scrolling the first match into view.
//   next() / prev(): advance/retreat the active match.
//   clear(): remove all highlights.
//
// On every state change, posts a JSON message via window.ipc.postMessage:
//   {"kind":"find_results","current":number,"total":number}
//   `current` is 1-based when matches > 0; 0 when no matches.
//
// Limitations (intentional for v0):
//   - Case-insensitive only.
//   - Plain-string search (no regex, no whole-word).
//   - Walks the main document only — iframes and shadow DOM are skipped.
//   - Re-walking the DOM on every setQuery is O(n); fine for typical pages.

(function () {
    "use strict";

    // Tear down any previous instance so re-injection is safe.
    if (window.__castcodes_find__ && typeof window.__castcodes_find__.clear === "function") {
        window.__castcodes_find__.clear();
    }

    const HIGHLIGHT_CLASS = "castcodes-find-hl";
    const ACTIVE_CLASS = "castcodes-find-active";
    const STYLE_ID = "castcodes-find-style";

    function ensureStylesheet() {
        if (document.getElementById(STYLE_ID)) return;
        const style = document.createElement("style");
        style.id = STYLE_ID;
        style.textContent =
            "." + HIGHLIGHT_CLASS + " { background: rgba(255, 230, 0, 0.6); color: inherit; border-radius: 2px; }" +
            "." + HIGHLIGHT_CLASS + "." + ACTIVE_CLASS + " { background: rgba(255, 150, 0, 0.9); outline: 1px solid rgba(255, 100, 0, 0.9); }";
        (document.head || document.documentElement).appendChild(style);
    }

    function postResults(current, total) {
        try {
            if (window.ipc && typeof window.ipc.postMessage === "function") {
                window.ipc.postMessage(JSON.stringify({
                    kind: "find_results",
                    current: current,
                    total: total,
                }));
            }
        } catch (_e) { /* swallow; host will time out if needed */ }
    }

    let matches = []; // array of HTMLSpanElement
    let activeIndex = -1;
    let lastQuery = "";

    function clearHighlights() {
        for (const el of document.querySelectorAll("." + HIGHLIGHT_CLASS)) {
            const parent = el.parentNode;
            if (!parent) continue;
            while (el.firstChild) parent.insertBefore(el.firstChild, el);
            parent.removeChild(el);
            parent.normalize();
        }
        matches = [];
        activeIndex = -1;
    }

    function isVisible(node) {
        // Cheap visibility check: skip if any ancestor is display:none or
        // visibility:hidden. We do this once per text node to avoid
        // scrolling to invisible matches.
        let el = node.parentElement;
        while (el) {
            const style = window.getComputedStyle(el);
            if (style.display === "none" || style.visibility === "hidden") return false;
            el = el.parentElement;
        }
        return true;
    }

    function collectTextNodes(root) {
        const nodes = [];
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
            acceptNode: function (node) {
                const parent = node.parentNode;
                if (!parent) return NodeFilter.FILTER_REJECT;
                const tag = parent.nodeName;
                if (tag === "SCRIPT" || tag === "STYLE" || tag === "NOSCRIPT" || tag === "TEXTAREA") {
                    return NodeFilter.FILTER_REJECT;
                }
                // Don't search inside our own highlight wrappers.
                if (parent.classList && parent.classList.contains(HIGHLIGHT_CLASS)) {
                    return NodeFilter.FILTER_REJECT;
                }
                if (!node.nodeValue || node.nodeValue.length === 0) return NodeFilter.FILTER_REJECT;
                return NodeFilter.FILTER_ACCEPT;
            },
        });
        let n;
        while ((n = walker.nextNode())) nodes.push(n);
        return nodes;
    }

    function highlightInNode(node, needle) {
        const text = node.nodeValue;
        const lower = text.toLowerCase();
        let offset = 0;
        let found = [];
        while (true) {
            const idx = lower.indexOf(needle, offset);
            if (idx === -1) break;
            found.push(idx);
            offset = idx + needle.length;
            if (needle.length === 0) break; // defensive
        }
        if (found.length === 0) return [];

        const parent = node.parentNode;
        if (!parent) return [];

        const wrappers = [];
        let cursor = 0;
        const frag = document.createDocumentFragment();
        for (const i of found) {
            if (i > cursor) frag.appendChild(document.createTextNode(text.slice(cursor, i)));
            const span = document.createElement("span");
            span.className = HIGHLIGHT_CLASS;
            span.appendChild(document.createTextNode(text.slice(i, i + needle.length)));
            frag.appendChild(span);
            wrappers.push(span);
            cursor = i + needle.length;
        }
        if (cursor < text.length) frag.appendChild(document.createTextNode(text.slice(cursor)));
        parent.replaceChild(frag, node);
        return wrappers;
    }

    function setActive(index) {
        for (const el of matches) el.classList.remove(ACTIVE_CLASS);
        if (matches.length === 0) {
            activeIndex = -1;
            postResults(0, 0);
            return;
        }
        const n = matches.length;
        activeIndex = ((index % n) + n) % n;
        const el = matches[activeIndex];
        el.classList.add(ACTIVE_CLASS);
        try {
            el.scrollIntoView({ behavior: "smooth", block: "center", inline: "nearest" });
        } catch (_e) {
            el.scrollIntoView();
        }
        postResults(activeIndex + 1, n);
    }

    function setQuery(query) {
        clearHighlights();
        lastQuery = (query || "");
        const needle = lastQuery.toLowerCase();
        if (needle.length === 0) {
            postResults(0, 0);
            return;
        }
        ensureStylesheet();
        const textNodes = collectTextNodes(document.body || document.documentElement);
        for (const node of textNodes) {
            if (!isVisible(node)) continue;
            const wrappers = highlightInNode(node, needle);
            if (wrappers.length) matches.push.apply(matches, wrappers);
        }
        if (matches.length > 0) setActive(0);
        else postResults(0, 0);
    }

    function next() {
        if (matches.length === 0) {
            postResults(0, 0);
            return;
        }
        setActive(activeIndex + 1);
    }

    function prev() {
        if (matches.length === 0) {
            postResults(0, 0);
            return;
        }
        setActive(activeIndex - 1);
    }

    function clear() {
        clearHighlights();
        lastQuery = "";
        postResults(0, 0);
    }

    window.__castcodes_find__ = {
        setQuery: setQuery,
        next: next,
        prev: prev,
        clear: clear,
    };
})();
