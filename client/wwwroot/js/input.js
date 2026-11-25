window.ScarletInput = {
  keys: {},
  pointer: { x: 0, y: 0, down: false },
  interact: false,
  debugToggle: false,
  profileToggle: false,
  attached: false,
  attach() {
    if (this.attached) return;
    this.attached = true;
    window.addEventListener("keydown", (e) => {
      if (e.code === "F3" && !this.keys.F3) {
        this.debugToggle = true;
        e.preventDefault();
      }
      if (e.code === "F4" && !this.keys.F4) {
        this.profileToggle = true;
        e.preventDefault();
      }
      this.keys[e.code] = true;
      if (e.code === "KeyE") this.interact = true;
      if (["Space", "KeyW", "KeyA", "KeyS", "KeyD", "F3", "F4"].includes(e.code)) e.preventDefault();
    });
    window.addEventListener("keyup", (e) => {
      this.keys[e.code] = false;
    });
    window.addEventListener("mousemove", (e) => {
      this.pointer.x = e.clientX;
      this.pointer.y = e.clientY;
    });
    window.addEventListener("mousedown", () => { this.pointer.down = true; });
    window.addEventListener("mouseup", () => { this.pointer.down = false; });
  },
  poll() {
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
