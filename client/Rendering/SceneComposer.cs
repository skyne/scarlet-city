using ScarletCity.Client.Services;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Rendering;

public static class SceneComposer
{
    public const float Scale = 14f;
    public const float ZScale = 12f;
    public const float VisionRadius = 32f;

    static readonly Dictionary<string, float> BuildingAlpha = new();

    public static RenderFrame Compose(
        int width,
        int height,
        MapData map,
        ServerMessage.Snapshot snap,
        Guid localId,
        float aimWorldX,
        float aimWorldY,
        float clientAimAngle,
        DebugGhosts? ghosts = null)
    {
        var frame = new RenderFrame { Width = width, Height = height };
        var focus = snap.Players.FirstOrDefault(p => p.Id == localId) ?? snap.Players.FirstOrDefault();
        var fx = focus?.X ?? map.Bounds.Width * 0.5f;
        var fy = focus?.Y ?? map.Bounds.Depth * 0.5f;
        var fz = focus?.Z ?? 0f;

        IsoProjection.Project(fx, fy, fz, Scale, ZScale, 0, 0, out var psx, out var psy);
        var originX = width * 0.5f - psx;
        var originY = height * 0.42f - psy;

        void P(float x, float y, float z, out float sx, out float sy) =>
            IsoProjection.Project(x, y, z, Scale, ZScale, originX, originY, out sx, out sy);

        DrawGround(frame, map, P);
        foreach (var road in map.Roads)
            DrawSlab(frame, road.X, road.Y, road.Width, road.Depth, 0, 0.05f, "#3a2a28", "#2a1c1b", P);

        var buildings = map.Obstacles.OrderBy(o => o.X + o.Y).ThenBy(o => o.ZMin);
        P(fx, fy, fz, out var playerSx, out var playerSy);
        foreach (var o in buildings)
            DrawBuilding(frame, o, playerSx, playerSy, fx, fy, P);

        foreach (var zone in snap.ExfilZones)
            DrawSlab(frame, zone.X, zone.Y, zone.Width, zone.Depth, zone.ZMin, zone.ZMax,
                "rgba(80,220,160,0.28)", "#5fe0a8", P);

        foreach (var loot in snap.LootNodes.Where(n => !n.Taken))
        {
            MaybeShadow(frame, loot.X, loot.Y, loot.Z, 0.5f, P);
            DrawDiamond(frame, loot.X, loot.Y, loot.Z, LootColor(loot.Tier), P);
        }

        foreach (var crate in snap.DeathCrates)
        {
            MaybeShadow(frame, crate.X, crate.Y, crate.Z, 0.7f, P);
            DrawEntity(frame, crate.X, crate.Y, crate.Z, 1.1f, 1.1f, 0.9f, "#c9a227", P);
            P(crate.X, crate.Y, crate.Z + 1.4f, out var cx, out var cy);
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { cx, cy },
                Fill = "#ffe8a3",
                Text = "Death crate",
                Size = 11
            });
        }

        foreach (var v in snap.Vehicles.OrderBy(v => v.X + v.Y).ThenBy(v => v.Z))
        {
            MaybeShadow(frame, v.X, v.Y, v.Z, 1.3f, P);
            var color = v.Kind == VehicleKind.HoverCraft ? "#6ec8ff" : "#d4553a";
            DrawEntity(frame, v.X, v.Y, v.Z, 1.8f, 3.2f, 1.1f, color, P);
            DrawFacing(frame, v.X, v.Y, v.Z, v.Yaw, 2.4f, color, P);
        }

        foreach (var p in snap.Players.OrderBy(p => p.X + p.Y).ThenBy(p => p.Z))
        {
            MaybeShadow(frame, p.X, p.Y, p.Z, 0.55f, P);
            var color = p.Id == localId ? "#ffe066" : "#ff6b6b";
            DrawEntity(frame, p.X, p.Y, p.Z, 0.7f, 0.7f, 1.7f, color, P);
            var yaw = p.Id == localId ? clientAimAngle : p.Yaw;
            DrawFacing(frame, p.X, p.Y, p.Z, yaw, 1.8f, color, P);
            if (p.Id == localId)
                DrawAim(frame, p.X, p.Y, p.Z, aimWorldX, aimWorldY, yaw, P);
            P(p.X, p.Y, p.Z + 2.1f, out var tx, out var ty);
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { tx, ty },
                Fill = "#f4e8dc",
                Text = $"{p.Name}  {p.Health:0}",
                Size = 12
            });
        }

        foreach (var shot in snap.Projectiles)
        {
            P(shot.X, shot.Y, shot.Z, out var sx, out var sy);
            frame.Ops.Add(new DrawOp
            {
                Kind = "dot",
                Pts = new[] { sx, sy, 3 },
                Fill = "#fff3c4"
            });
        }

        if (ghosts != null)
            DrawNetGhosts(frame, ghosts, P);

        P(fx, fy, fz, out var fowX, out var fowY);
        frame.Ops.Add(new DrawOp
        {
            Kind = "fow",
            Pts = new[] { fowX, fowY, VisionRadius * Scale, VisionRadius * Scale * 0.5f },
            Fill = "rgba(8,6,8,0.72)"
        });

        return frame;
    }

    static void DrawNetGhosts(RenderFrame frame, DebugGhosts g, ProjectFn P)
    {
        // Predicted (solid you) is already drawn gold. Ghosts: server + delayed interp.
        DrawGhostBody(frame, g.ServerX, g.ServerY, g.ServerZ, g.ServerYaw, "#4df0ff", "server", P);
        if (g.HasDelayed)
            DrawGhostBody(frame, g.DelayedX, g.DelayedY, g.DelayedZ, g.DelayedYaw, "#c77dff", "interp 90ms", P);
        DrawGhostBody(frame, g.PredX, g.PredY, g.PredZ, g.PredYaw, "#ffe066", "pred", P, wireOnly: true);

        P(g.ServerX, g.ServerY, g.ServerZ + 0.9f, out var sx, out var sy);
        P(g.PredX, g.PredY, g.PredZ + 0.9f, out var px, out var py);
        frame.Ops.Add(new DrawOp
        {
            Kind = "line",
            Pts = new[] { sx, sy, px, py },
            Stroke = "rgba(77,240,255,0.85)",
            Size = 2
        });
        if (g.HasDelayed)
        {
            P(g.DelayedX, g.DelayedY, g.DelayedZ + 0.9f, out var dx, out var dy);
            frame.Ops.Add(new DrawOp
            {
                Kind = "line",
                Pts = new[] { sx, sy, dx, dy },
                Stroke = "rgba(199,125,255,0.7)",
                Size = 1.5f
            });
        }

        if (g.InVehicle)
        {
            DrawBox(frame,
                g.VehicleServerX - 0.9f, g.VehicleServerY - 1.6f, 1.8f, 3.2f,
                g.VehicleServerZ, g.VehicleServerZ + 1.1f,
                "#4df0ff", P, 0.28f);
            DrawFacing(frame, g.VehicleServerX, g.VehicleServerY, g.VehicleServerZ, g.VehicleServerYaw, 2.4f, "#4df0ff", P, 0.55f);
        }
    }

    static void DrawGhostBody(RenderFrame frame, float x, float y, float z, float yaw, string color, string label,
        ProjectFn P, bool wireOnly = false)
    {
        P(x, y, 0, out var gx, out var gy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "ring",
            Pts = new[] { gx, gy, 10f },
            Stroke = color,
            Size = 1.5f,
            Alpha = 0.9f
        });
        DrawBox(frame, x - 0.38f, y - 0.38f, 0.76f, 0.76f, z, z + 1.7f, color, P, wireOnly ? 0.22f : 0.38f);
        DrawFacing(frame, x, y, z, yaw, 1.8f, color, P, wireOnly ? 0.45f : 0.7f);
        P(x, y, z + 2.35f, out var tx, out var ty);
        frame.Ops.Add(new DrawOp
        {
            Kind = "text",
            Pts = new[] { tx, ty },
            Fill = color,
            Text = label,
            Size = 11
        });
    }

    static void MaybeShadow(RenderFrame frame, float x, float y, float z, float r,
        ProjectFn P)
    {
        if (z <= 0.01f) return;
        P(x, y, 0, out var sx, out var sy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "ellipse",
            Pts = new[] { sx, sy, r * Scale, r * Scale * 0.5f },
            Fill = "rgba(0,0,0,0.45)"
        });
    }

    static void DrawGround(RenderFrame frame, MapData map, ProjectFn P)
    {
        var b = map.Bounds;
        DrawSlab(frame, b.X, b.Y, b.Width, b.Depth, 0, 0, "#1b1412", "#3d2a24", P);
    }

    static void DrawBuilding(RenderFrame frame, Obstacle o, float playerSx, float playerSy,
        float playerX, float playerY, ProjectFn P)
    {
        var top = Shade(o.ZMax);
        var target = 1f;
        if (o.ZMax >= 4f && BuildingOccludesPlayer(o, playerSx, playerSy, playerX, playerY, P))
            target = 0.25f;
        if (!BuildingAlpha.TryGetValue(o.Id, out var alpha))
            alpha = 1f;
        alpha += (target - alpha) * 0.18f;
        BuildingAlpha[o.Id] = alpha;
        DrawBox(frame, o.X, o.Y, o.Width, o.Depth, o.ZMin, o.ZMax, top, P, alpha);
    }

    static bool BuildingOccludesPlayer(Obstacle o, float psx, float psy, float px, float py, ProjectFn P)
    {
        var inFront = o.X + o.Width * 0.5f + o.Y + o.Depth * 0.5f >= px + py - 1.5f;
        P(o.X, o.Y, o.ZMax, out var a0, out var a1);
        P(o.X + o.Width, o.Y, o.ZMax, out var b0, out var b1);
        P(o.X + o.Width, o.Y + o.Depth, o.ZMax, out var c0, out var c1);
        P(o.X, o.Y + o.Depth, o.ZMax, out var d0, out var d1);
        P(o.X, o.Y, o.ZMin, out var e0, out var e1);
        P(o.X + o.Width, o.Y + o.Depth, o.ZMin, out var f0, out var f1);
        var minX = Math.Min(Math.Min(a0, b0), Math.Min(Math.Min(c0, d0), Math.Min(e0, f0)));
        var maxX = Math.Max(Math.Max(a0, b0), Math.Max(Math.Max(c0, d0), Math.Max(e0, f0)));
        var minY = Math.Min(Math.Min(a1, b1), Math.Min(Math.Min(c1, d1), Math.Min(e1, f1)));
        var maxY = Math.Max(Math.Max(a1, b1), Math.Max(Math.Max(c1, d1), Math.Max(e1, f1)));
        var overlaps = psx >= minX - 8 && psx <= maxX + 8 && psy >= minY - 8 && psy <= maxY + 8;
        return overlaps && inFront;
    }

    static string Shade(float zMax) => zMax >= 12 ? "#6a3a44" : zMax >= 6 ? "#8a4d3c" : "#a36a4e";

    static void DrawSlab(RenderFrame frame, float x, float y, float w, float d, float z0, float z1,
        string fill, string stroke, ProjectFn P)
    {
        P(x, y, z1, out var a0, out var a1);
        P(x + w, y, z1, out var b0, out var b1);
        P(x + w, y + d, z1, out var c0, out var c1);
        P(x, y + d, z1, out var d0, out var d1);
        frame.Ops.Add(new DrawOp
        {
            Kind = "poly",
            Pts = new[] { a0, a1, b0, b1, c0, c1, d0, d1 },
            Fill = fill,
            Stroke = stroke,
            Alpha = 1f
        });
        _ = z0;
    }

    static void DrawBox(RenderFrame frame, float x, float y, float w, float d, float z0, float z1,
        string top, ProjectFn P, float alpha = 1f)
    {
        P(x, y, z0, out var bl0, out var bl1);
        P(x + w, y, z0, out var br0, out var br1);
        P(x + w, y + d, z0, out var fr0, out var fr1);
        P(x, y + d, z0, out var fl0, out var fl1);
        P(x, y, z1, out var tbl0, out var tbl1);
        P(x + w, y, z1, out var tbr0, out var tbr1);
        P(x + w, y + d, z1, out var tfr0, out var tfr1);
        P(x, y + d, z1, out var tfl0, out var tfl1);

        frame.Ops.Add(new DrawOp
        {
            Kind = "poly",
            Pts = new[] { fl0, fl1, fr0, fr1, tfr0, tfr1, tfl0, tfl1 },
            Fill = Darken(top, 0.72f),
            Alpha = alpha
        });
        frame.Ops.Add(new DrawOp
        {
            Kind = "poly",
            Pts = new[] { br0, br1, fr0, fr1, tfr0, tfr1, tbr0, tbr1 },
            Fill = Darken(top, 0.85f),
            Alpha = alpha
        });
        frame.Ops.Add(new DrawOp
        {
            Kind = "poly",
            Pts = new[] { tbl0, tbl1, tbr0, tbr1, tfr0, tfr1, tfl0, tfl1 },
            Fill = top,
            Stroke = "rgba(0,0,0,0.35)",
            Alpha = alpha
        });
        _ = (bl0, bl1);
    }

    static void DrawEntity(RenderFrame frame, float x, float y, float z, float w, float d, float h,
        string color, ProjectFn P)
    {
        DrawBox(frame, x - w * 0.5f, y - d * 0.5f, w, d, z, z + h, color, P);
    }

    static void DrawFacing(RenderFrame frame, float x, float y, float z, float yaw, float length, string color, ProjectFn P,
        float alpha = 1f)
    {
        var tipX = x + MathF.Cos(yaw) * length;
        var tipY = y + MathF.Sin(yaw) * length;
        var back = length * 0.55f;
        var side = 0.42f;
        var lx = x + MathF.Cos(yaw) * back - MathF.Sin(yaw) * side;
        var ly = y + MathF.Sin(yaw) * back + MathF.Cos(yaw) * side;
        var rx = x + MathF.Cos(yaw) * back + MathF.Sin(yaw) * side;
        var ry = y + MathF.Sin(yaw) * back - MathF.Cos(yaw) * side;
        P(tipX, tipY, z + 0.08f, out var t0, out var t1);
        P(lx, ly, z + 0.08f, out var l0, out var l1);
        P(rx, ry, z + 0.08f, out var r0, out var r1);
        frame.Ops.Add(new DrawOp
        {
            Kind = "poly",
            Pts = new[] { t0, t1, l0, l1, r0, r1 },
            Fill = color,
            Stroke = "#111",
            Alpha = alpha
        });
    }

    static void DrawAim(RenderFrame frame, float x, float y, float z, float aimX, float aimY, float yaw, ProjectFn P)
    {
        var chest = z + 0.95f;
        var range = 7.5f;
        var endX = x + MathF.Cos(yaw) * range;
        var endY = y + MathF.Sin(yaw) * range;
        P(x, y, chest, out var a0, out var a1);
        P(endX, endY, chest, out var b0, out var b1);
        frame.Ops.Add(new DrawOp
        {
            Kind = "line",
            Pts = new[] { a0, a1, b0, b1 },
            Stroke = "rgba(255,224,102,0.85)",
            Size = 2
        });
        P(aimX, aimY, z, out var cx, out var cy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "ring",
            Pts = new[] { cx, cy, 9f },
            Stroke = "#ffe066",
            Size = 2
        });
        frame.Ops.Add(new DrawOp
        {
            Kind = "line",
            Pts = new[] { cx - 12, cy, cx + 12, cy },
            Stroke = "#ffe066",
            Size = 1.5f
        });
        frame.Ops.Add(new DrawOp
        {
            Kind = "line",
            Pts = new[] { cx, cy - 7, cx, cy + 7 },
            Stroke = "#ffe066",
            Size = 1.5f
        });
    }

    static void DrawDiamond(RenderFrame frame, float x, float y, float z, string color, ProjectFn P)
    {
        P(x, y, z + 0.6f, out var sx, out var sy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "dot",
            Pts = new[] { sx, sy, 6 },
            Fill = color
        });
    }

    static string LootColor(LootTier tier) => tier switch
    {
        LootTier.Epic => "#c77dff",
        LootTier.Rare => "#4cc9f0",
        _ => "#bde0fe"
    };

    static string Darken(string hex, float t)
    {
        if (hex.Length != 7 || hex[0] != '#') return hex;
        int r = Convert.ToInt32(hex.Substring(1, 2), 16);
        int g = Convert.ToInt32(hex.Substring(3, 2), 16);
        int b = Convert.ToInt32(hex.Substring(5, 2), 16);
        r = (int)(r * t);
        g = (int)(g * t);
        b = (int)(b * t);
        return $"#{r:X2}{g:X2}{b:X2}";
    }

    delegate void ProjectFn(float x, float y, float z, out float sx, out float sy);
}
