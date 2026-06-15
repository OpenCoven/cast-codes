CREATE TABLE browser_panes (
  id INTEGER PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL DEFAULT 'browser' CHECK (kind = 'browser'),

  -- Stable per-pane session id (UUID v4 string). Keys the per-pane
  -- WebKit data directory at `<warp_home>/browser/data/<session_id>/`
  -- so cookies/localStorage/IndexedDB survive across restarts.
  session_id TEXT NOT NULL,

  -- Serialized `BrowserState` (open intra-pane tabs, active index, etc.)
  -- stored as a JSON string. Schema evolution is handled inside
  -- `BrowserState` via serde rather than alter-table churn.
  state_json TEXT NOT NULL,

  FOREIGN KEY (id, kind) REFERENCES pane_leaves (pane_node_id, kind)
);
