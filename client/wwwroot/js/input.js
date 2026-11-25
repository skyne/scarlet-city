window.ScarletInput = {
  keys: {},
  // Chromium can emit keyup for a key that is still held while the pointer
  // moves, and that keyup sometimes arrives before the matching mousemove.
  // Repeats re-assert the key. A keyup is applied only after a short delay,
  // and is dropped if the pointer moved or a newer keydown landed.
  downStamp: {},
  suspectRelease: {},
  lastPointerMove: 0,
  pointer: { x: 0, y: 0, down: false },
  interact: false,
  debugToggle: false,
  profileToggle: false,
  swapSlot: 0,
  swapNext: false,
  orderPing: false,
  orderMove: false,
  attached: false,
  notePointer() {
    this.lastPointerMove = performance.now();
  },
  typingTarget(e) {
    const t = e.target;
    if (!t || !t.tagName) return false;
    const tag = t.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
  },
  uiTarget(e) {
    const t = e.target;
    if (!t || !t.closest) return false;
    return !!t.closest("button, input, select, textarea, a, .panel");
  },
  expireSuspects() {
    const now = performance.now();
    if (now - this.lastPointerMove < 90) return;
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
      if (this.typingTarget(e)) return;
      const now = performance.now();
      this.downStamp[e.code] = now;
      this.keys[e.code] = true;
      delete this.suspectRelease[e.code];
      if (e.repeat) return;
      if (e.code === "F3") {
        this.debugToggle = true;
        e.preventDefault();
      }
      if (e.code === "F4") {
        this.profileToggle = true;
        e.preventDefault();
      }
      if (e.code === "KeyE") this.interact = true;
      if (e.code === "Digit1") this.swapSlot = 1;
      if (e.code === "Digit2") this.swapSlot = 2;
      if (e.code === "Digit3") this.swapSlot = 3;
      if (e.code === "Digit4") this.swapSlot = 4;
      if (e.code === "Tab") {
        this.swapNext = true;
        e.preventDefault();
      }
      if (e.code === "KeyG") this.orderPing = true;
      if (e.code === "Space" || e.code === "F3" || e.code === "F4") e.preventDefault();
    }, true);
    window.addEventListener("keyup", (e) => {
      if (this.typingTarget(e)) return;
      const code = e.code;
      const stamped = this.downStamp[code] || 0;
      const releasedAt = performance.now();
      setTimeout(() => {
        if ((this.downStamp[code] || 0) > stamped) return;
        if (performance.now() - this.lastPointerMove < 180 || releasedAt - this.lastPointerMove < 180) {
          this.suspectRelease[code] = performance.now() + 220;
          return;
        }
        this.keys[code] = false;
        delete this.suspectRelease[code];
      }, 40);
    }, true);
    const trackPointer = (e) => {
      this.notePointer();
      this.pointer.x = e.clientX;
      this.pointer.y = e.clientY;
    };
    window.addEventListener("mousemove", trackPointer, true);
    window.addEventListener("pointermove", trackPointer, true);
    window.addEventListener("pointerrawupdate", trackPointer, true);
    window.addEventListener("mousedown", (e) => {
      if (this.uiTarget(e)) return;
      const alt = e.altKey || this.keys.AltLeft || this.keys.AltRight;
      if (e.button === 1 || (e.button === 0 && alt)) {
        this.orderMove = true;
        e.preventDefault();
        return;
      }
      if (e.button === 0) this.pointer.down = true;
    }, true);
    window.addEventListener("mouseup", (e) => {
      if (e.button === 0) this.pointer.down = false;
    }, true);
    window.addEventListener("auxclick", (e) => {
      if (e.button === 1) e.preventDefault();
    }, true);
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
    const swapSlot = this.swapSlot;
    this.swapSlot = 0;
    const swapNext = this.swapNext;
    this.swapNext = false;
    const orderPing = this.orderPing;
    this.orderPing = false;
    const orderMove = this.orderMove;
    this.orderMove = false;
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
      swapSlot,
      swapNext,
      orderPing,
      orderMove,
      canvasWidth: canvas ? canvas.width : 0,
      canvasHeight: canvas ? canvas.height : 0
    };
  }
};
