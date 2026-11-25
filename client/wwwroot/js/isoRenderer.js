window.ScarletIso = {
  canvas: null,
  ctx: null,
  ensure() {
    if (this.canvas) return;
    this.canvas = document.getElementById("game-canvas");
    this.ctx = this.canvas.getContext("2d");
    const resize = () => {
      this.canvas.width = window.innerWidth;
      this.canvas.height = window.innerHeight;
    };
    window.addEventListener("resize", resize);
    resize();
  },
  size() {
    this.ensure();
    return [this.canvas.width, this.canvas.height];
  },
  draw(frame) {
    this.ensure();
    const ctx = this.ctx;
    ctx.globalAlpha = 1;
    ctx.fillStyle = frame.clear || "#140c10";
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    for (const op of frame.ops || []) {
      const alpha = op.alpha == null ? 1 : op.alpha;
      ctx.globalAlpha = alpha;
      if (op.kind === "poly") {
        const p = op.pts;
        ctx.beginPath();
        ctx.moveTo(p[0], p[1]);
        for (let i = 2; i < p.length; i += 2) ctx.lineTo(p[i], p[i + 1]);
        ctx.closePath();
        ctx.fillStyle = op.fill;
        ctx.fill();
        if (op.stroke && op.stroke !== "transparent") {
          ctx.strokeStyle = op.stroke;
          ctx.stroke();
        }
      } else if (op.kind === "ellipse") {
        const [x, y, rx, ry] = op.pts;
        ctx.beginPath();
        ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
        ctx.fillStyle = op.fill;
        ctx.fill();
      } else if (op.kind === "dot") {
        const [x, y, r] = op.pts;
        ctx.beginPath();
        ctx.arc(x, y, r, 0, Math.PI * 2);
        ctx.fillStyle = op.fill;
        ctx.fill();
      } else if (op.kind === "line") {
        const p = op.pts;
        ctx.beginPath();
        ctx.moveTo(p[0], p[1]);
        ctx.lineTo(p[2], p[3]);
        ctx.strokeStyle = op.stroke || op.fill;
        ctx.lineWidth = op.size || 2;
        ctx.lineCap = "round";
        ctx.stroke();
      } else if (op.kind === "ring") {
        const [x, y, r] = op.pts;
        ctx.beginPath();
        ctx.arc(x, y, r, 0, Math.PI * 2);
        ctx.strokeStyle = op.stroke || op.fill;
        ctx.lineWidth = op.size || 2;
        ctx.stroke();
      } else if (op.kind === "text") {
        ctx.fillStyle = op.fill;
        ctx.font = `${op.size || 12}px sans-serif`;
        ctx.textAlign = "center";
        ctx.fillText(op.text || "", op.pts[0], op.pts[1]);
      } else if (op.kind === "actors") {
        ctx.globalAlpha = 1;
        if (window.ScarletCharacters) window.ScarletCharacters.render(ctx, this.canvas, op);
      } else if (op.kind === "sprite") {
        const img = window.ScarletWorldAssets && window.ScarletWorldAssets.get(op.asset || op.text);
        if (img) {
          const [x, y, w, h] = op.pts;
          const anchor = op.anchor === "center" ? 0.5 : 1;
          ctx.save();
          ctx.translate(x, y);
          if (op.rotation) ctx.rotate(op.rotation);
          ctx.drawImage(img, -w * 0.5, -h * anchor, w, h);
          ctx.restore();
        }
      } else if (op.kind === "fow") {
        ctx.globalAlpha = 1;
        const [x, y, rx, ry] = op.pts;
        ctx.save();
        ctx.beginPath();
        ctx.rect(0, 0, this.canvas.width, this.canvas.height);
        ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
        ctx.fillStyle = op.fill || "rgba(8,6,8,0.72)";
        ctx.fill("evenodd");
        const g = ctx.createRadialGradient(x, y, Math.min(rx, ry) * 0.72, x, y, Math.max(rx, ry));
        g.addColorStop(0, "rgba(8,6,8,0)");
        g.addColorStop(1, "rgba(8,6,8,0.55)");
        ctx.beginPath();
        ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
        ctx.fillStyle = g;
        ctx.fill();
        ctx.restore();
      }
    }
    ctx.globalAlpha = 1;
  }
};

window.ScarletCanvas = window.ScarletIso;
