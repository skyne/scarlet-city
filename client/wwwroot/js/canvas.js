// Compatibility shim — isometric draw path lives in isoRenderer.js
if (!window.ScarletCanvas && window.ScarletIso) {
  window.ScarletCanvas = window.ScarletIso;
}
