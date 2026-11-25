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
        float camX,
        float camY,
        float camZ,
        DebugGhosts? ghosts = null,
        bool debug = false)
    {
        var frame = new RenderFrame { Width = width, Height = height };
        var focus = snap.Players.FirstOrDefault(p => p.Id == localId) ?? snap.Players.FirstOrDefault();
        var fx = focus?.X ?? camX;
        var fy = focus?.Y ?? camY;
        var fz = focus?.Z ?? camZ;

        IsoProjection.Project(camX, camY, camZ, Scale, ZScale, 0, 0, out var psx, out var psy);
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
        DrawWorldDecor(frame, map, P);

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
            var heistBag = crate.Items.Any(i => i.StatsJson.Contains("\"heist\":true"));
            MaybeShadow(frame, crate.X, crate.Y, crate.Z, 0.7f, P);
            DrawEntity(frame, crate.X, crate.Y, crate.Z, 1.1f, 1.1f, 0.9f, heistBag ? "#ffb020" : "#c9a227", P);
            P(crate.X, crate.Y, crate.Z + 1.4f, out var cx, out var cy);
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { cx, cy },
                Fill = heistBag ? "#ffd27a" : "#ffe8a3",
                Text = heistBag ? "Heist bag" : "Death crate",
                Size = 11
            });
        }

        foreach (var v in snap.Vehicles.OrderBy(v => v.X + v.Y).ThenBy(v => v.Z))
        {
            MaybeShadow(frame, v.X, v.Y, v.Z, 1.3f, P);
            var color = v.Law ? "#7aa2ff" : v.Kind == VehicleKind.HoverCraft ? "#6ec8ff" : "#d4553a";
            var asset = WorldAssetCatalog.Vehicle(v.Kind, v.Law, v.Id);
            DrawSprite(frame, asset, v.X, v.Y, v.Z, 3.6f * Scale, 2.5f * Scale, P, alpha: 0.96f);
            DrawFacing(frame, v.X, v.Y, v.Z, v.Yaw, 2.4f, color, P);
        }

        foreach (var npc in snap.Npcs.OrderBy(n => n.X + n.Y).ThenBy(n => n.Z))
        {
            MaybeShadow(frame, npc.X, npc.Y, npc.Z, 0.55f, P);
            var color = npc.Kind switch
            {
                "Swat" => "#d5dbe6",
                "Police" => "#4c8dff",
                _ => "#e24b5a"
            };
            DrawEntity(frame, npc.X, npc.Y, npc.Z, 0.7f, 0.7f, 1.7f, color, P);
            DrawFacing(frame, npc.X, npc.Y, npc.Z, npc.Yaw, 1.8f, color, P);
            DrawSprite(frame, WorldAssetCatalog.NpcWeapon(npc.Kind), npc.X, npc.Y, npc.Z + 1.05f,
                1.8f * Scale, 1.2f * Scale, P, "center", alpha: 0.92f);
            P(npc.X, npc.Y, npc.Z + 2.15f, out var nx, out var ny);
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { nx, ny },
                Fill = color,
                Text = $"{npc.Kind}  {npc.Health:0}",
                Size = 11
            });
        }

        var actors = new List<ActorDraw>();
        foreach (var p in snap.Players.OrderBy(p => p.X + p.Y).ThenBy(p => p.Z))
        {
            MaybeShadow(frame, p.X, p.Y, p.Z, 0.55f, P);
            var yaw = p.Id == localId ? clientAimAngle : p.Yaw;
            var speed = MathF.Sqrt(p.Vx * p.Vx + p.Vy * p.Vy);
            actors.Add(new ActorDraw
            {
                Id = p.Id.ToString("N"),
                X = p.X,
                Y = p.Y,
                Z = p.Z,
                Yaw = yaw,
                Speed = speed,
                Variant = p.Appearance.ToVariant(),
                Appearance = p.Appearance.Normalized()
            });
            if (p.Id == localId)
                DrawAim(frame, p.X, p.Y, p.Z, aimWorldX, aimWorldY, yaw, P);
        }

        // Bodies paint under name labels. Fog is added later and darkens them.
        frame.Ops.Add(new DrawOp
        {
            Kind = "actors",
            Pts = new[] { originX, originY, Scale, ZScale },
            Actors = actors
        });

        // Weapons are attached to 3D character hands in characters.js.

        foreach (var p in snap.Players)
        {
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

        DrawImpacts(frame, snap, P);
        DrawOrders(frame, snap, P);

        if (ghosts != null)
            DrawNetGhosts(frame, ghosts, P);

        P(fx, fy, fz, out var fowX, out var fowY);
        frame.Ops.Add(new DrawOp
        {
            Kind = "fow",
            Pts = new[] { fowX, fowY, VisionRadius * Scale, VisionRadius * Scale * 0.5f },
            Fill = "rgba(8,6,8,0.72)"
        });

        DrawHeistSites(frame, snap, P);
        DrawInteractPrompt(frame, snap, localId, P);

        if (debug)
        {
            foreach (var loot in snap.LootNodes.Where(n => n.Taken && n.RespawnSecs > 0f))
                DrawLootGhost(frame, loot, P);
        }

        return frame;
    }

    const float InteractRange = 2.5f;
    const float InteractZ = 2.0f;

    static void DrawImpacts(RenderFrame frame, ServerMessage.Snapshot snap, ProjectFn P)
    {
        foreach (var hit in snap.Impacts)
        {
            var actor = hit.HitKind == HitKind.Actor;
            var color = actor ? "#ff4d6a" : "#ffe08a";
            P(hit.FromX, hit.FromY, hit.FromZ, out var ax, out var ay);
            P(hit.X, hit.Y, hit.Z, out var bx, out var by);
            frame.Ops.Add(new DrawOp
            {
                Kind = "line",
                Pts = new[] { ax, ay, bx, by },
                Stroke = color,
                Size = 2
            });
            frame.Ops.Add(new DrawOp
            {
                Kind = actor ? "dot" : "ring",
                Pts = actor ? new[] { bx, by, 5 } : new[] { bx, by, 7 },
                Fill = color,
                Stroke = color,
                Size = 2
            });
        }
    }

    static void DrawOrders(RenderFrame frame, ServerMessage.Snapshot snap, ProjectFn P)
    {
        var seen = new HashSet<(int, int, int)>();
        foreach (var member in snap.Crew)
        {
            if (!member.IsAi) continue;
            if (member.OrderLabel is not ("Hold" or "Move" or "Drive" or "Engage")) continue;
            var key = ((int)MathF.Round(member.OrderX * 2), (int)MathF.Round(member.OrderY * 2), (int)MathF.Round(member.OrderZ * 2));
            if (!seen.Add(key)) continue;
            MaybeShadow(frame, member.OrderX, member.OrderY, member.OrderZ, 0.35f, P);
            P(member.OrderX, member.OrderY, member.OrderZ, out var sx, out var sy);
            var color = member.OrderLabel == "Engage" ? "rgba(255,120,96,0.9)" : "rgba(140,210,255,0.9)";
            frame.Ops.Add(new DrawOp
            {
                Kind = "ring",
                Pts = new[] { sx, sy, 8 },
                Stroke = color,
                Fill = color,
                Size = 1.5f
            });
            P(member.OrderX, member.OrderY, member.OrderZ + 0.9f, out var tx, out var ty);
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { tx, ty },
                Fill = color,
                Text = member.OrderLabel,
                Size = 11
            });
        }
    }

    static void DrawHeistSites(RenderFrame frame, ServerMessage.Snapshot snap, ProjectFn P)
    {
        foreach (var site in snap.Heists)
        {
            var color = site.Alarm
                ? "#ff3355"
                : site.Kind switch
                {
                    HeistKind.Vault => "#f0c14a",
                    HeistKind.Terminal => "#5ee0ff",
                    _ => "#ff8a3d"
                };
            MaybeShadow(frame, site.X, site.Y, site.Z, 0.9f, P);
            DrawDiamond(frame, site.X, site.Y, site.Z + 0.4f, color, P);
            P(site.X, site.Y, site.Z + 2.4f, out var lx, out var ly);
            var state = site.State.ToString();
            if (site.State == HeistState.InProgress)
                state = $"{site.Approach} {site.Progress:P0}";
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { lx, ly },
                Fill = color,
                Text = $"{site.Name}  {state}",
                Size = 12
            });

            MaybeShadow(frame, site.QuietX, site.QuietY, site.QuietZ, 0.4f, P);
            P(site.QuietX, site.QuietY, site.QuietZ + 0.2f, out var qx, out var qy);
            frame.Ops.Add(new DrawOp
            {
                Kind = "ring",
                Pts = new[] { qx, qy, 7 },
                Stroke = "#8fd9ff",
                Fill = "#8fd9ff",
                Size = 1.5f
            });
            MaybeShadow(frame, site.LoudX, site.LoudY, site.LoudZ, 0.4f, P);
            P(site.LoudX, site.LoudY, site.LoudZ + 0.2f, out var bx, out var by);
            frame.Ops.Add(new DrawOp
            {
                Kind = "ring",
                Pts = new[] { bx, by, 7 },
                Stroke = "#ff6a4a",
                Fill = "#ff6a4a",
                Size = 1.5f
            });
        }

        foreach (var alarm in snap.Alarms)
        {
            var hot = alarm.Kind == "heist";
            MaybeShadow(frame, alarm.X, alarm.Y, alarm.Z, 1.1f, P);
            P(alarm.X, alarm.Y, alarm.Z + 0.3f, out var ax, out var ay);
            frame.Ops.Add(new DrawOp
            {
                Kind = "ring",
                Pts = new[] { ax, ay, hot ? 18 : 11 },
                Stroke = hot ? "#ff3355" : "#ffd166",
                Fill = hot ? "#ff3355" : "#ffd166",
                Size = 2
            });
            P(alarm.X, alarm.Y, alarm.Z + 2.8f, out var tx, out var ty);
            frame.Ops.Add(new DrawOp
            {
                Kind = "text",
                Pts = new[] { tx, ty },
                Fill = hot ? "#ff8b9a" : "#ffe08a",
                Text = hot ? "ALARM" : "SHOTS",
                Size = 12
            });
        }
    }

    static string QuietLabel(HeistMarker site) => site.QuietZ > 2.5f ? "rooftop hack" : "side access";

    static void DrawInteractPrompt(RenderFrame frame, ServerMessage.Snapshot snap, Guid localId, ProjectFn P)
    {
        var me = snap.Players.FirstOrDefault(p => p.Id == localId);
        if (me is null) return;

        string? text = null;
        var px = me.X;
        var py = me.Y;
        var pz = me.Z;
        var best = float.MaxValue;

        void Consider(float x, float y, float z, string label)
        {
            var dx = x - me.X;
            var dy = y - me.Y;
            var dist = MathF.Sqrt(dx * dx + dy * dy);
            if (dist <= InteractRange && MathF.Abs(z - me.Z) <= InteractZ && dist < best)
            {
                best = dist;
                text = label;
                px = x;
                py = y;
                pz = z;
            }
        }

        foreach (var loot in snap.LootNodes)
        {
            if (loot.Taken) continue;
            Consider(loot.X, loot.Y, loot.Z, "[E] Loot Cache");
        }

        foreach (var crate in snap.DeathCrates)
        {
            var n = crate.Items.Count;
            if (n == 0) continue;
            var heistBag = crate.Items.Any(i => i.StatsJson.Contains("\"heist\":true"));
            var label = heistBag
                ? "[E] Take heist bag"
                : n == 1 ? "[E] Loot Crate (1 item)" : $"[E] Loot Crate ({n} items)";
            Consider(crate.X, crate.Y, crate.Z, label);
        }

        if (text is null)
        {
            foreach (var site in snap.Heists)
            {
                if (site.State is HeistState.Cooldown or HeistState.Looted or HeistState.Breached)
                    continue;
                Consider(site.QuietX, site.QuietY, site.QuietZ,
                    site.State == HeistState.InProgress && site.Approach == "Quiet"
                        ? $"[Hold E] {site.Name} {site.Progress:P0}"
                        : $"[Hold E] Quiet {QuietLabel(site)}");
                Consider(site.LoudX, site.LoudY, site.LoudZ,
                    site.State == HeistState.InProgress && site.Approach == "Loud"
                        ? $"[Hold E] Breach {site.Progress:P0}"
                        : "[Hold E] Loud breach");
            }
        }

        if (text is null)
        {
            if (me.VehicleId is { } seated)
            {
                var car = snap.Vehicles.FirstOrDefault(v => v.Id == seated);
                text = "[E] Exit Vehicle";
                px = car?.X ?? me.X;
                py = car?.Y ?? me.Y;
                pz = car?.Z ?? me.Z;
            }
            else
            {
                foreach (var v in snap.Vehicles)
                {
                    if (v.DriverId is not null || v.Hp <= 0) continue;
                    Consider(v.X, v.Y, v.Z, "[E] Enter Vehicle");
                }
            }
        }

        if (text is null) return;
        P(px, py, pz + 1.6f, out var tx, out var ty);
        frame.Ops.Add(new DrawOp
        {
            Kind = "text",
            Pts = new[] { tx, ty },
            Fill = "#ffe8a3",
            Text = text,
            Size = 13
        });
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

    static void DrawSprite(RenderFrame frame, string? asset, float x, float y, float z,
        float width, float height, ProjectFn P, string anchor = "bottom", float rotation = 0f, float alpha = 1f)
    {
        if (string.IsNullOrEmpty(asset)) return;
        P(x, y, z, out var sx, out var sy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "sprite",
            Pts = new[] { sx, sy, width, height },
            Asset = asset,
            Anchor = anchor,
            Rotation = rotation,
            Alpha = alpha
        });
    }

    static void DrawWorldDecor(RenderFrame frame, MapData map, ProjectFn P)
    {
        var streetAssets = new[]
        {
            "quaternius/street_pack/streetlight_single",
            "quaternius/street_pack/trafficlight",
            "quaternius/street_pack/sign_stop"
        };
        var index = 0;
        foreach (var road in map.Roads)
        {
            var horizontal = road.Width >= road.Depth;
            var length = horizontal ? road.Width : road.Depth;
            var count = Math.Clamp((int)(length / 28f), 1, 5);
            for (var i = 0; i < count; i++)
            {
                var t = (i + 1f) / (count + 1f);
                var x = horizontal ? road.X + road.Width * t : road.X - 0.9f;
                var y = horizontal ? road.Y - 0.9f : road.Y + road.Depth * t;
                DrawSprite(frame, streetAssets[index++ % streetAssets.Length], x, y, 0f,
                    1.7f * Scale, 2.5f * Scale, P, alpha: 0.9f);
            }
        }
        var props = new[]
        {
            "quaternius/survival_pack/trashcan",
            "quaternius/cyberpunk_pack/ac",
            "quaternius/survival_pack/gascan",
            "kit-cidade/mobiliario/banco_praca"
        };
        var propIndex = 0;
        foreach (var o in map.Obstacles.Where(o => o.ZMax < 10f).Take(8))
        {
            DrawSprite(frame, props[propIndex++ % props.Length], o.X + o.Width + 0.5f,
                o.Y + o.Depth + 0.5f, 0f, 1.5f * Scale, 1.4f * Scale, P, alpha: 0.85f);
        }
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
        DrawBox(frame, o.X, o.Y, o.Width, o.Depth, o.ZMin, o.ZMax, top, P, alpha * 0.32f);
        var asset = WorldAssetCatalog.Building(o.Id);
        if (asset is null) return;
        var span = o.Width + o.Depth;
        DrawSprite(frame, asset, o.X + o.Width * 0.5f, o.Y + o.Depth * 0.5f, o.ZMin,
            span * Scale * 0.62f,
            span * Scale * 0.52f + (o.ZMax - o.ZMin) * ZScale * 0.85f,
            P, alpha: alpha);
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

    static void DrawLootGhost(RenderFrame frame, LootNode loot, ProjectFn P)
    {
        var color = LootColor(loot.Tier);
        P(loot.X, loot.Y, 0, out var gx, out var gy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "ring",
            Pts = new[] { gx, gy, 8f },
            Stroke = color,
            Size = 1.5f,
            Alpha = 0.85f
        });
        P(loot.X, loot.Y, loot.Z + 0.6f, out var sx, out var sy);
        frame.Ops.Add(new DrawOp
        {
            Kind = "dot",
            Pts = new[] { sx, sy, 5 },
            Fill = color,
            Alpha = 0.4f
        });
        var secs = MathF.Max(0f, loot.RespawnSecs);
        var clock = secs >= 10f ? $"{MathF.Ceiling(secs):0}s" : $"{secs:0.0}s";
        P(loot.X, loot.Y, loot.Z + 1.5f, out var tx, out var ty);
        frame.Ops.Add(new DrawOp
        {
            Kind = "text",
            Pts = new[] { tx, ty },
            Fill = color,
            Text = $"{loot.Tier}  {clock}",
            Size = 11,
            Alpha = 0.95f
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
