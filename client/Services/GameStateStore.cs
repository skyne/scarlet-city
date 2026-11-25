using ScarletCity.Client.Simulation;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

public sealed class PendingInput
{
    public uint Seq;
    public float Dt;
    public float MoveX, MoveY, AscendDescend, AimAngle;
    public bool Shoot, Interact, Brake;
}

public sealed class DebugGhosts
{
    public float PredX, PredY, PredZ, PredYaw;
    public float ServerX, ServerY, ServerZ, ServerYaw;
    public float ServerVx, ServerVy, ServerVz;
    public bool HasDelayed;
    public float DelayedX, DelayedY, DelayedZ, DelayedYaw;
    public float Error;
    public int Pending;
    public uint AckSeq;
    public bool InVehicle;
    public float VehicleServerX, VehicleServerY, VehicleServerZ, VehicleServerYaw;
}

/// <summary>
/// Local prediction, server reconciliation, and remote interpolation (~90ms).
/// </summary>
public sealed class GameStateStore
{
    public const double InterpDelayMs = 90;
    const int MaxPending = 128;
    const int MaxSnaps = 8;

    readonly List<PendingInput> _pending = new();
    readonly List<(double TimeMs, ServerMessage.Snapshot Snap)> _snaps = new();
    readonly LocalPhysics.Pose _prevPose = new();
    readonly Dictionary<Guid, LocalPhysics.VehiclePose> _prevVehicles = new();

    /// <summary>0 at a sim step, 1 just before the next. Render lerps previous to current.</summary>
    public float RenderAlpha { get; set; } = 1f;

    public MapData? Map { get; set; }
    public Guid ActorId { get; set; }
    public LocalPhysics.Pose Predicted { get; } = new();
    public bool HasPredicted { get; private set; }
    public PlayerState? LastServerLocal { get; private set; }
    public VehicleState? LastServerVehicle { get; private set; }
    public int PendingCount => _pending.Count;
    public uint LastAckSeq { get; private set; }

    readonly Dictionary<Guid, LocalPhysics.VehiclePose> _predictedVehicles = new();

    public void Reset()
    {
        _pending.Clear();
        _snaps.Clear();
        _predictedVehicles.Clear();
        _prevVehicles.Clear();
        HasPredicted = false;
        ActorId = Guid.Empty;
        LastServerLocal = null;
        LastServerVehicle = null;
        LastAckSeq = 0;
    }

    public void QueueAndPredict(PendingInput input)
    {
        _pending.Add(input);
        if (_pending.Count > MaxPending)
            _pending.RemoveAt(0);
        if (Map == null || !HasPredicted) return;
        RememberPrev();
        Step(Predicted, input, Map);
    }

    public void ApplySnapshot(ServerMessage.Snapshot snap, double nowMs)
    {
        _snaps.Add((nowMs, CloneSnap(snap)));
        while (_snaps.Count > MaxSnaps)
            _snaps.RemoveAt(0);

        if (Map == null || ActorId == Guid.Empty) return;
        var me = snap.Players.FirstOrDefault(p => p.Id == ActorId);
        if (me == null) return;

        Predicted.X = me.X;
        Predicted.Y = me.Y;
        Predicted.Z = me.Z;
        Predicted.Yaw = me.Yaw;
        Predicted.Vx = me.Vx;
        Predicted.Vy = me.Vy;
        Predicted.Vz = me.Vz;
        Predicted.VehicleId = me.VehicleId;
        HasPredicted = true;
        LastServerLocal = ClonePlayer(me);
        LastAckSeq = me.LastProcessedInputSeq;
        LastServerVehicle = me.VehicleId is { } driven
            ? snap.Vehicles.FirstOrDefault(v => v.Id == driven)
            : null;
        if (LastServerVehicle != null)
            LastServerVehicle = CopyVehicleState(LastServerVehicle);

        _predictedVehicles.Clear();
        foreach (var v in snap.Vehicles)
            _predictedVehicles[v.Id] = CopyVehicle(v);

        CopyPose(Predicted, _prevPose);
        CopyVehicleMap(_predictedVehicles, _prevVehicles);

        var ack = me.LastProcessedInputSeq;
        _pending.RemoveAll(p => SeqLte(p.Seq, ack));
        foreach (var inp in _pending)
        {
            RememberPrev();
            Step(Predicted, inp, Map);
        }
    }

    public ServerMessage.Snapshot RenderSnapshot(double nowMs, float aimAngle)
    {
        var interpAt = nowMs - InterpDelayMs;
        var remote = Interpolate(interpAt);
        if (HasPredicted)
        {
            OverlayLocal(remote, aimAngle);
        }
        return remote;
    }

    public PlayerState? LocalActor(ServerMessage.Snapshot snap) =>
        snap.Players.FirstOrDefault(p => p.Id == ActorId);

    /// <summary>
    /// Delayed interpolated pose of the local actor from server snaps (what remotes render).
    /// </summary>
    public PlayerState? DelayedServerLocal(double nowMs)
    {
        if (ActorId == Guid.Empty) return null;
        var delayed = Interpolate(nowMs - InterpDelayMs);
        return delayed.Players.FirstOrDefault(p => p.Id == ActorId);
    }

    public DebugGhosts? BuildGhosts(double nowMs)
    {
        if (!HasPredicted || LastServerLocal == null) return null;
        var delayed = DelayedServerLocal(nowMs);
        var err = Dist(Predicted.X, Predicted.Y, Predicted.Z, LastServerLocal.X, LastServerLocal.Y, LastServerLocal.Z);
        return new DebugGhosts
        {
            PredX = Predicted.X,
            PredY = Predicted.Y,
            PredZ = Predicted.Z,
            PredYaw = Predicted.Yaw,
            ServerX = LastServerLocal.X,
            ServerY = LastServerLocal.Y,
            ServerZ = LastServerLocal.Z,
            ServerYaw = LastServerLocal.Yaw,
            ServerVx = LastServerLocal.Vx,
            ServerVy = LastServerLocal.Vy,
            ServerVz = LastServerLocal.Vz,
            HasDelayed = delayed != null,
            DelayedX = delayed?.X ?? 0,
            DelayedY = delayed?.Y ?? 0,
            DelayedZ = delayed?.Z ?? 0,
            DelayedYaw = delayed?.Yaw ?? 0,
            Error = err,
            Pending = PendingCount,
            AckSeq = LastAckSeq,
            InVehicle = Predicted.VehicleId != null,
            VehicleServerX = LastServerVehicle?.X ?? LastServerLocal.X,
            VehicleServerY = LastServerVehicle?.Y ?? LastServerLocal.Y,
            VehicleServerZ = LastServerVehicle?.Z ?? LastServerLocal.Z,
            VehicleServerYaw = LastServerVehicle?.Yaw ?? LastServerLocal.Yaw
        };
    }

    static float Dist(float x0, float y0, float z0, float x1, float y1, float z1)
    {
        var dx = x0 - x1;
        var dy = y0 - y1;
        var dz = z0 - z1;
        return MathF.Sqrt(dx * dx + dy * dy + dz * dz);
    }

    static VehicleState CopyVehicleState(VehicleState v) => new()
    {
        Id = v.Id, Kind = v.Kind, X = v.X, Y = v.Y, Z = v.Z, Yaw = v.Yaw, Speed = v.Speed, DriverId = v.DriverId
    };

    static bool SeqLte(uint a, uint b) => unchecked(b - a) < 1u << 31;

    void RememberPrev()
    {
        CopyPose(Predicted, _prevPose);
        CopyVehicleMap(_predictedVehicles, _prevVehicles);
    }

    void Step(LocalPhysics.Pose pose, PendingInput input, MapData map)
    {
        var dt = LocalPhysics.FixedDt;
        pose.Yaw = input.AimAngle;
        if (pose.VehicleId is { } vid && _predictedVehicles.TryGetValue(vid, out var veh))
        {
            LocalPhysics.Drive(pose, veh, map, input.MoveX, input.MoveY, input.AscendDescend, input.Brake, dt);
        }
        else
        {
            pose.VehicleId = null;
            LocalPhysics.Walk(pose, map, input.MoveX, input.MoveY, dt);
        }
    }

    void OverlayLocal(ServerMessage.Snapshot snap, float aimAngle)
    {
        var me = snap.Players.FirstOrDefault(p => p.Id == ActorId);
        if (me == null)
        {
            me = new PlayerState { Id = ActorId };
            snap.Players.Add(me);
        }
        var t = Math.Clamp(RenderAlpha, 0f, 1f);
        me.X = Lerp(_prevPose.X, Predicted.X, t);
        me.Y = Lerp(_prevPose.Y, Predicted.Y, t);
        me.Z = Lerp(_prevPose.Z, Predicted.Z, t);
        me.Yaw = aimAngle;
        me.Vx = Predicted.Vx;
        me.Vy = Predicted.Vy;
        me.Vz = Predicted.Vz;
        me.VehicleId = Predicted.VehicleId;
        if (Predicted.VehicleId is { } vid && _predictedVehicles.TryGetValue(vid, out var veh))
        {
            var vs = snap.Vehicles.FirstOrDefault(v => v.Id == vid);
            if (vs != null)
            {
                _prevVehicles.TryGetValue(vid, out var prev);
                prev ??= veh;
                vs.X = Lerp(prev.X, veh.X, t);
                vs.Y = Lerp(prev.Y, veh.Y, t);
                vs.Z = Lerp(prev.Z, veh.Z, t);
                vs.Yaw = LerpAngle(prev.Yaw, veh.Yaw, t);
                vs.Speed = Lerp(prev.Speed, veh.Speed, t);
            }
        }
    }

    ServerMessage.Snapshot Interpolate(double atMs)
    {
        if (_snaps.Count == 0)
            return new ServerMessage.Snapshot();
        if (_snaps.Count == 1 || atMs >= _snaps[^1].TimeMs)
            return CloneSnap(_snaps[^1].Snap);

        var laterIdx = _snaps.FindIndex(s => s.TimeMs >= atMs);
        if (laterIdx <= 0)
            return CloneSnap(_snaps[0].Snap);

        var (t0, a) = _snaps[laterIdx - 1];
        var (t1, b) = _snaps[laterIdx];
        var span = t1 - t0;
        var u = span <= 1e-4 ? 1.0 : Math.Clamp((atMs - t0) / span, 0, 1);
        var t = (float)u;
        var outSnap = CloneSnap(b);
        LerpPlayers(a, b, t, outSnap);
        LerpVehicles(a, b, t, outSnap);
        LerpProjectiles(a, b, t, outSnap);
        return outSnap;
    }

    static void LerpPlayers(ServerMessage.Snapshot a, ServerMessage.Snapshot b, float t, ServerMessage.Snapshot dest)
    {
        var prev = a.Players.ToDictionary(p => p.Id);
        foreach (var p in dest.Players)
        {
            if (!prev.TryGetValue(p.Id, out var o)) continue;
            p.X = Lerp(o.X, p.X, t);
            p.Y = Lerp(o.Y, p.Y, t);
            p.Z = Lerp(o.Z, p.Z, t);
            p.Yaw = LerpAngle(o.Yaw, p.Yaw, t);
        }
    }

    static void LerpVehicles(ServerMessage.Snapshot a, ServerMessage.Snapshot b, float t, ServerMessage.Snapshot dest)
    {
        var prev = a.Vehicles.ToDictionary(v => v.Id);
        foreach (var v in dest.Vehicles)
        {
            if (!prev.TryGetValue(v.Id, out var o)) continue;
            v.X = Lerp(o.X, v.X, t);
            v.Y = Lerp(o.Y, v.Y, t);
            v.Z = Lerp(o.Z, v.Z, t);
            v.Yaw = LerpAngle(o.Yaw, v.Yaw, t);
            v.Speed = Lerp(o.Speed, v.Speed, t);
        }
    }

    static void LerpProjectiles(ServerMessage.Snapshot a, ServerMessage.Snapshot b, float t, ServerMessage.Snapshot dest)
    {
        var prev = a.Projectiles.ToDictionary(p => p.Id);
        foreach (var p in dest.Projectiles)
        {
            if (!prev.TryGetValue(p.Id, out var o)) continue;
            p.X = Lerp(o.X, p.X, t);
            p.Y = Lerp(o.Y, p.Y, t);
            p.Z = Lerp(o.Z, p.Z, t);
        }
    }

    static float Lerp(float a, float b, float t) => a + (b - a) * t;

    static float LerpAngle(float a, float b, float t)
    {
        var diff = b - a;
        while (diff > MathF.PI) diff -= MathF.Tau;
        while (diff < -MathF.PI) diff += MathF.Tau;
        return a + diff * t;
    }

    static void CopyPose(LocalPhysics.Pose src, LocalPhysics.Pose dst)
    {
        dst.X = src.X;
        dst.Y = src.Y;
        dst.Z = src.Z;
        dst.Yaw = src.Yaw;
        dst.Vx = src.Vx;
        dst.Vy = src.Vy;
        dst.Vz = src.Vz;
        dst.VehicleId = src.VehicleId;
    }

    static void CopyVehicleMap(
        Dictionary<Guid, LocalPhysics.VehiclePose> src,
        Dictionary<Guid, LocalPhysics.VehiclePose> dst)
    {
        dst.Clear();
        foreach (var (id, v) in src)
            dst[id] = CloneVehicle(v);
    }

    static LocalPhysics.VehiclePose CloneVehicle(LocalPhysics.VehiclePose v) => new()
    {
        Id = v.Id,
        Kind = v.Kind,
        X = v.X,
        Y = v.Y,
        Z = v.Z,
        Yaw = v.Yaw,
        VelYaw = v.VelYaw,
        Speed = v.Speed,
        DriverId = v.DriverId
    };

    static LocalPhysics.VehiclePose CopyVehicle(VehicleState v) => new()
    {
        Id = v.Id,
        Kind = v.Kind,
        X = v.X,
        Y = v.Y,
        Z = v.Z,
        Yaw = v.Yaw,
        VelYaw = v.Yaw,
        Speed = v.Speed,
        DriverId = v.DriverId
    };

    static ServerMessage.Snapshot CloneSnap(ServerMessage.Snapshot s) => new()
    {
        Tick = s.Tick,
        LastProcessedInputSeq = s.LastProcessedInputSeq,
        Players = s.Players.Select(ClonePlayer).ToList(),
        Vehicles = s.Vehicles.Select(v => new VehicleState
        {
            Id = v.Id, Kind = v.Kind, X = v.X, Y = v.Y, Z = v.Z, Yaw = v.Yaw, Speed = v.Speed, DriverId = v.DriverId
        }).ToList(),
        LootNodes = s.LootNodes.Select(n => new LootNode
        {
            Id = n.Id, X = n.X, Y = n.Y, Z = n.Z, Tier = n.Tier, Taken = n.Taken
        }).ToList(),
        DeathCrates = s.DeathCrates.Select(c => new DeathCrate
        {
            Id = c.Id, X = c.X, Y = c.Y, Z = c.Z, Items = c.Items.ToList()
        }).ToList(),
        ExfilZones = s.ExfilZones.ToList(),
        Projectiles = s.Projectiles.Select(p => new Projectile
        {
            Id = p.Id, X = p.X, Y = p.Y, Z = p.Z, Vx = p.Vx, Vy = p.Vy, Vz = p.Vz, OwnerId = p.OwnerId
        }).ToList()
    };

    static PlayerState ClonePlayer(PlayerState p) => new()
    {
        Id = p.Id,
        PlayerId = p.PlayerId,
        Name = p.Name,
        X = p.X, Y = p.Y, Z = p.Z, Yaw = p.Yaw,
        Vx = p.Vx, Vy = p.Vy, Vz = p.Vz,
        Health = p.Health,
        VehicleId = p.VehicleId,
        Extracting = p.Extracting,
        ExtractProgress = p.ExtractProgress,
        LastProcessedInputSeq = p.LastProcessedInputSeq
    };
}
