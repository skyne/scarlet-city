window.ScarletInput = {
  keys: {},
  // Chromium sends keyup for keys that are still held while the pointer moves.
  // Keep those keys down until the pointer settles and no keydown reasserts them.
  suspectRelease: {},
  lastPointerMove: 0,
  pointer: { x: 0, y: 0, down: false },
  interact: false,
  debugToggle: false,
  profileToggle: false,
  attached: false,
  notePointer() {
    this.lastPointerMove = performance.now();
  },
  expireSuspects() {
    const now = performance.now();
    if (now - this.lastPointerMove < 80) return;
    for (const code of Object.keys(this.suspectRelease)) {
      if (now >= this.suspectRelease[code]) {
        this.keys[code] = false;
        delete this.suspectRelease[code];
      }
    }
  },
  attach() {
    if (this.attached) return;
    this.attached = true;
    window.addEventListener("keydown", (e) => {
      // Key repeats while WASD is held starve mousemove in Chromium, so aim
      // freezes until the key is released. The first keydown is enough.
      if (e.repeat) return;
      if (e.code === "F3" && !this.keys.F3) {
        this.debugToggle = true;
        e.preventDefault();
      }
      if (e.code === "F4" && !this.keys.F4) {
        this.profileToggle = true;
        e.preventDefault();
      }
      this.keys[e.code] = true;
      delete this.suspectRelease[e.code];
      if (e.code === "KeyE") this.interact = true;
      if (e.code === "Space" || e.code === "F3" || e.code === "F4") e.preventDefault();
    }, true);
    window.addEventListener("keyup", (e) => {
      const now = performance.now();
      if (now - this.lastPointerMove < 120) {
        this.suspectRelease[e.code] = now + 160;
        return;
      }
      this.keys[e.code] = false;
      delete this.suspectRelease[e.code];
    }, true);
    const trackPointer = (e) => {
      this.notePointer();
      this.pointer.x = e.clientX;
      this.pointer.y = e.clientY;
    };
    window.addEventListener("mousemove", trackPointer, true);
    window.addEventListener("pointermove", trackPointer, true);
    window.addEventListener("mousedown", () => { this.pointer.down = true; }, true);
    window.addEventListener("mouseup", () => { this.pointer.down = false; }, true);
    window.addEventListener("blur", () => {
      this.keys = {};
      this.suspectRelease = {};
      this.pointer.down = false;
    });
  },
  poll() {
    this.expireSuspects();
    const k = this.keys;
    const moveX = (k.KeyD ? 1 : 0) - (k.KeyA ? 1 : 0);
    const moveY = (k.KeyW ? 1 : 0) - (k.KeyS ? 1 : 0);
    const up = (k.Space ? 1 : 0) - (k.KeyC ? 1 : 0);
    const interact = this.interact;
    this.interact = false;
    const toggleDebug = this.debugToggle;
    this.debugToggle = false;
    const toggleProfile = this.profileToggle;
    this.profileToggle = false;
    const canvas = document.getElementById("game-canvas");
    return {
      moveX,
      moveY,
      ascendDescend: up,
      pointerX: this.pointer.x,
      pointerY: this.pointer.y,
      shoot: this.pointer.down,
      interact,
      brake: !!k.ShiftLeft || !!k.ShiftRight,
      toggleDebug,
      toggleProfile,
      canvasWidth: canvas ? canvas.width : 0,
      canvasHeight: canvas ? canvas.height : 0
    };
  }
};
