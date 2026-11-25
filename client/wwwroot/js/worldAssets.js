const state = {
  manifest: null,
  images: new Map(),
  pending: new Map(),
  failed: new Set(),
  manifestPromise: null
};

const PRIORITY_PREFIXES = [
  "quaternius/cars/",
  "quaternius/guns/",
  "quaternius/buildings_pack_2/",
  "quaternius/buildings_pack_3/",
  "quaternius/street_pack/streetlight",
  "quaternius/street_pack/trafficlight",
  "quaternius/street_pack/sign_stop",
  "quaternius/survival_pack/trashcan",
  "quaternius/cyberpunk_pack/ac"
];

function normalizeKey(key) {
  return String(key || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9/]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_|_$/g, "");
}

function loadManifest() {
  if (state.manifestPromise) return state.manifestPromise;
  state.manifestPromise = fetch("assets/world/manifest.json?v=2")
    .then((r) => {
      if (!r.ok) throw new Error(`manifest ${r.status}`);
      return r.json();
    })
    .then((m) => {
      state.manifest = m;
      return m;
    })
    .catch((e) => {
      state.failed.add("manifest");
      console.warn("world asset manifest unavailable", e);
      return null;
    });
  return state.manifestPromise;
}

function assetInfo(key) {
  const m = state.manifest;
  if (!m || !m.assets) return null;
  const wanted = normalizeKey(key);
  if (m.assets[wanted]) return m.assets[wanted];
  for (const [candidate, info] of Object.entries(m.assets)) {
    if (normalizeKey(candidate) === wanted) return info;
  }
  return null;
}

function loadSprite(key) {
  if (!key || state.failed.has(key)) return Promise.resolve(null);
  if (state.images.has(key)) return Promise.resolve(state.images.get(key));
  if (state.pending.has(key)) return state.pending.get(key);
  const promise = loadManifest().then(() => new Promise((resolve) => {
    const info = assetInfo(key);
    if (!info) {
      state.failed.add(key);
      resolve(null);
      return;
    }
    const url = (state.manifest.spriteBase || "assets/world/sprites/") + info.sprite + "?v=2";
    const img = new Image();
    img.onload = () => {
      state.images.set(key, img);
      state.pending.delete(key);
      resolve(img);
    };
    img.onerror = () => {
      state.failed.add(key);
      state.pending.delete(key);
      resolve(null);
    };
    img.src = url;
  }));
  state.pending.set(key, promise);
  return promise;
}

function getSprite(key) {
  if (state.images.has(key)) return state.images.get(key);
  loadSprite(key);
  return null;
}

function info(key) {
  return assetInfo(key);
}

function preload(keys) {
  return loadManifest().then(() => Promise.all((keys || []).map(loadSprite)));
}

loadManifest().then(() => {
  const keys = Object.keys(state.manifest?.assets || {}).filter((key) =>
    PRIORITY_PREFIXES.some((prefix) => key.startsWith(prefix))
  );
  return Promise.all(keys.map(loadSprite));
}).catch(() => null);

window.ScarletWorldAssets = {
  get: getSprite,
  info,
  load: loadSprite,
  preload,
  ready: () => state.manifest !== null,
  keys: () => Object.keys(state.manifest?.assets || {})
};
