using System.Diagnostics;
using ScarletCity.Client.Rendering;
using ScarletCity.Client.Simulation;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

public enum SessionPhase
{
    Menu,
    Connecting,
    Hideout,
    Raid,
    Summary
}

public sealed class DebugStats
{
    public bool Visible { get; set; }
    public double Fps { get; set; }
    public double FrameMs { get; set; }
    public int PingMs { get; set; } = -1;
    public ulong ServerTick { get; set; }
    public double SnapshotHz { get; set; }
    public uint InputSeq { get; set; }
    public bool Connected { get; set; }
    public int Players { get; set; }
    public int Vehicles { get; set; }
    public int Projectiles { get; set; }
    public float X { get; set; }
    public float Y { get; set; }
    public float Z { get; set; }
    public float YawDeg { get; set; }
    public bool InVehicle { get; set; }
    public float PredX { get; set; }
    public float PredY { get; set; }
    public float PredZ { get; set; }
    public float ServerX { get; set; }
    public float ServerY { get; set; }
    public float ServerZ { get; set; }
    public float PredError { get; set; }
    public int Pending { get; set; }
    public uint AckSeq { get; set; }

    public string Text()
    {
        var ping = PingMs < 0 ? "…" : $"{PingMs} ms";
        return
            $"Scarlet debug  (F3)\n" +
            $"fps        {Fps:0.0}   frame {FrameMs:0.0} ms\n" +
            $"ping       {ping}\n" +
            $"tick       {ServerTick}   snap {SnapshotHz:0.0} Hz  (sim 20 Hz)\n" +
            $"input seq  {InputSeq}   ws {(Connected ? "up" : "down")}\n" +
            $"pos        {X:0.00}, {Y:0.00}, {Z:0.00}   yaw {YawDeg:0}°\n" +
            $"pred       {PredX:0.00}, {PredY:0.00}, {PredZ:0.00}\n" +
            $"server     {ServerX:0.00}, {ServerY:0.00}, {ServerZ:0.00}\n" +
            $"err        {PredError:0.03} m   pending {Pending}  ack {AckSeq}\n" +
            $"ghosts     gold pred  cyan server  purple interp\n" +
            $"loot       hollow marker + tier timer while respawning\n" +
            $"vehicle    {(InVehicle ? "yes" : "on foot")}\n" +
            $"ents       players {Players}  vehicles {Vehicles}  shots {Projectiles}";
    }
}

public sealed class ProfileStats
{
    public bool Visible { get; set; }
    public double PeriodMs { get; private set; }
    public double WorkMs { get; private set; }
    public double DelayMs { get; private set; }
    public double InputMs { get; private set; }
    public double SizeMs { get; private set; }
    public double LogicMs { get; private set; }
    public double SendMs { get; private set; }
    public double ComposeMs { get; private set; }
    public double DrawMs { get; private set; }
    public double OtherMs { get; private set; }
    public int DrawOps { get; private set; }
    public int JsCalls { get; private set; }
    public double PeakPeriodMs { get; private set; }
    public double PeakDrawMs { get; private set; }
    public double PeakComposeMs { get; private set; }
    public double PeakSendMs { get; private set; }
    public double CapSettingMs { get; set; }

    public void Push(in FrameProbe probe, double workMs, double delayMs, double periodMs)
    {
        WorkMs = Smooth(WorkMs, workMs);
        DelayMs = Smooth(DelayMs, delayMs);
        PeriodMs = Smooth(PeriodMs, periodMs);
        InputMs = Smooth(InputMs, probe.InputMs);
        SizeMs = Smooth(SizeMs, probe.SizeMs);
        LogicMs = Smooth(LogicMs, probe.LogicMs);
        SendMs = Smooth(SendMs, probe.SendMs);
        ComposeMs = Smooth(ComposeMs, probe.ComposeMs);
        DrawMs = Smooth(DrawMs, probe.DrawMs);
        var named = probe.InputMs + probe.SizeMs + probe.LogicMs + probe.SendMs + probe.ComposeMs + probe.DrawMs;
        OtherMs = Smooth(OtherMs, Math.Max(0, workMs - named));
        DrawOps = probe.DrawOps;
        JsCalls = probe.JsCalls;

        if (periodMs > PeakPeriodMs) PeakPeriodMs = periodMs;
        if (probe.DrawMs > PeakDrawMs) PeakDrawMs = probe.DrawMs;
        if (probe.ComposeMs > PeakComposeMs) PeakComposeMs = probe.ComposeMs;
        if (probe.SendMs > PeakSendMs) PeakSendMs = probe.SendMs;
    }

    public void DecayPeaks()
    {
        PeakPeriodMs *= 0.92;
        PeakDrawMs *= 0.92;
        PeakComposeMs *= 0.92;
        PeakSendMs *= 0.92;
    }

    public string Text()
    {
        var fps = PeriodMs > 0 ? 1000.0 / PeriodMs : 0;
        return
            "Scarlet profile  (F4)\n" +
            "smoothed; peaks decay\n" +
            $"period     {PeriodMs,6:0.00} ms   {fps,5:0.0} fps\n" +
            $"work       {Line(WorkMs)}\n" +
            $"cap        {Line(DelayMs)}   {(CapSettingMs <= 0 ? "off" : $"{CapSettingMs:0} ms")}\n" +
            $"input      {Line(InputMs)}   js\n" +
            $"size       {Line(SizeMs)}   js\n" +
            $"logic      {Line(LogicMs)}\n" +
            $"send       {Line(SendMs)}   ws\n" +
            $"compose    {Line(ComposeMs)}   ops {DrawOps}\n" +
            $"draw       {Line(DrawMs)}   js + json\n" +
            $"other      {Line(OtherMs)}\n" +
            $"js calls   {JsCalls} / frame\n" +
            $"peak       period {PeakPeriodMs:0.0}  draw {PeakDrawMs:0.00}  compose {PeakComposeMs:0.00}  send {PeakSendMs:0.00}";
    }

    string Line(double ms)
    {
        var pct = PeriodMs > 0 ? ms / PeriodMs * 100.0 : 0;
        return $"{ms,6:0.00} ms  {pct,4:0}%";
    }

    static double Smooth(double ema, double sample) =>
        ema <= 0 ? sample : ema * 0.85 + sample * 0.15;
}

public struct FrameProbe
{
    public double InputMs;
    public double SizeMs;
    public double LogicMs;
    public double SendMs;
    public double ComposeMs;
    public double DrawMs;
    public int DrawOps;
    public int JsCalls;
}

public sealed class GameSession : IAsyncDisposable
{
    readonly IGameTransport _transport;
    readonly IRenderer _renderer;
    readonly IInputSource _input;
    readonly IsoCamera _camera;
    readonly GameStateStore _store;
    readonly Stopwatch _clock = Stopwatch.StartNew();

    public Guid PlayerId { get; private set; }
    public Guid ActorId { get; private set; }
    public MapData? Map { get; private set; }
    public ServerMessage.Snapshot? Snapshot { get; private set; }
    public string Status { get; private set; } = "disconnected";
    public string LastEvent { get; set; } = "";
    public DebugStats Debug { get; } = new();
    public ProfileStats Profile { get; } = new();
    public SessionPhase Phase { get; private set; } = SessionPhase.Menu;
    public int Cash { get; private set; }
    public List<StashItem> Stash { get; private set; } = new();
    public List<StashItem> DeployedLoadout { get; private set; } = new();
    public List<CrewMember> Roster { get; private set; } = new();
    public List<Guid> SelectedCrewIds { get; private set; } = new();
    public string JoinSquad { get; set; } = "";
    public Guid? OrderTargetId { get; private set; }
    public Guid SquadId { get; private set; }
    public string PlayerName { get; private set; } = "";
    public ServerMessage.RaidEnded? LastRaid { get; private set; }
    public Guid? SelectedWeaponId { get; set; }
    public Guid? SelectedMedId { get; set; }

    public string CrewCountLabel => SelectedCrewIds.Count switch
    {
        1 => "Solo",
        2 => "Duo",
        3 => "Trio",
        4 => "Squad",
        _ => "Pick a crew"
    };

    /// <summary>Target frame time in milliseconds. 0 runs uncapped; set this for a later fps or vsync cap.</summary>
    public double FrameCapMs { get; set; }

    uint _seq;
    bool _running;
    long _simClock;
    double _simAccum;
    bool _latchShoot;
    bool _latchInteract;
    bool _rosterReady;
    int _orderCycle;
    double _fpsEma;
    int _snapCount;
    double _snapWindowStart;
    long _lastPingTs;
    long _lastPeakDecay;
    FrameProbe _probe;
    const long PingEveryMs = 500;

    public GameSession(
        IGameTransport transport,
        IRenderer renderer,
        IInputSource input,
        IsoCamera camera,
        GameStateStore store)
    {
        _transport = transport;
        _renderer = renderer;
        _input = input;
        _camera = camera;
        _store = store;
        _transport.MessageReceived += OnMessage;
    }

    public async Task StartAsync(string wsUrl, string name)
    {
        PlayerName = name;
        Phase = SessionPhase.Connecting;
        Status = "connecting";
        await _input.AttachAsync();
        await _transport.ConnectAsync(new Uri(wsUrl));
        await _transport.SendAsync(new ClientMessage.Join { Name = name });
        Status = "connected";
        _running = true;
        _simClock = Stopwatch.GetTimestamp();
        _simAccum = 0;
        _snapWindowStart = _clock.Elapsed.TotalSeconds;
        _ = Loop();
    }

    public async Task DeployAsync()
    {
        if (SelectedCrewIds.Count is < 1 or > 4) return;
        var ids = new List<Guid>();
        if (SelectedWeaponId is { } w) ids.Add(w);
        if (SelectedMedId is { } m) ids.Add(m);
        Guid? join = Guid.TryParse(JoinSquad.Trim(), out var parsed) ? parsed : null;
        await _transport.SendAsync(new ClientMessage.Deploy
        {
            LoadoutItemIds = ids,
            CrewIds = SelectedCrewIds.ToList(),
            JoinSquadId = join
        });
    }

    public void ToggleCrew(Guid id)
    {
        if (SelectedCrewIds.Contains(id))
        {
            if (SelectedCrewIds.Count <= 1) return;
            SelectedCrewIds.Remove(id);
            return;
        }
        if (SelectedCrewIds.Count >= 4) return;
        SelectedCrewIds.Add(id);
    }

    public void SelectOrderTarget(Guid actorId)
    {
        var member = Snapshot?.Crew.FirstOrDefault(c => c.ActorId == actorId);
        if (member == null || !member.IsAi || actorId == ActorId)
        {
            OrderTargetId = null;
            return;
        }
        OrderTargetId = OrderTargetId == actorId ? null : actorId;
    }

    public void DismissSummary()
    {
        LastRaid = null;
        if (Phase == SessionPhase.Summary)
            Phase = SessionPhase.Hideout;
    }

    async Task Loop()
    {
        while (_running)
        {
            var start = Stopwatch.GetTimestamp();
            try
            {
                await TickOnce();
            }
            catch
            {
                Status = "loop-error";
            }
            var afterWork = Stopwatch.GetTimestamp();
            var workMs = MsBetween(start, afterWork);
            var delayMs = 0.0;
            if (FrameCapMs > 0)
            {
                var spareMs = FrameCapMs - workMs;
                if (spareMs >= 1)
                {
                    var delayStart = Stopwatch.GetTimestamp();
                    await Task.Delay((int)Math.Min(spareMs, 250));
                    delayMs = MsBetween(delayStart, Stopwatch.GetTimestamp());
                }
            }
            // Blazor WASM has one thread. With no browser timer, this loop's
            // continuations run forever and clicks, drawing, and the socket never get a turn.
            // Delay(1) only yields; it is not the frame cap.
            if (delayMs == 0)
            {
                var delayStart = Stopwatch.GetTimestamp();
                await Task.Delay(1);
                delayMs = MsBetween(delayStart, Stopwatch.GetTimestamp());
            }
            var periodMs = workMs + delayMs;
            var instFps = 1000.0 / Math.Max(periodMs, 0.001);
            _fpsEma = _fpsEma <= 1 ? instFps : _fpsEma * 0.9 + instFps * 0.1;
            Debug.Fps = _fpsEma;
            Debug.FrameMs = periodMs;
            Profile.CapSettingMs = FrameCapMs;
            Profile.Push(_probe, workMs, delayMs, periodMs);
            var now = _clock.ElapsedMilliseconds;
            if (now - _lastPeakDecay >= 1000)
            {
                _lastPeakDecay = now;
                Profile.DecayPeaks();
            }
        }
    }

    static double MsBetween(long a, long b) =>
        (b - a) * 1000.0 / Stopwatch.Frequency;

    async Task TickOnce()
    {
        var now = _clock.ElapsedMilliseconds;
        var simNow = Stopwatch.GetTimestamp();
        var frameSec = Math.Clamp(MsBetween(_simClock, simNow) / 1000.0, 0, 0.25);
        _simClock = simNow;

        _probe = default;
        var mark = Stopwatch.GetTimestamp();
        var frame = await _input.PollAsync();
        _probe.InputMs = MsBetween(mark, Stopwatch.GetTimestamp());
        _probe.JsCalls++;
        if (frame.ToggleDebug)
            Debug.Visible = !Debug.Visible;
        if (frame.ToggleProfile)
            Profile.Visible = !Profile.Visible;

        mark = Stopwatch.GetTimestamp();
        var size = await _renderer.GetSizeAsync();
        _probe.SizeMs = MsBetween(mark, Stopwatch.GetTimestamp());
        _probe.JsCalls++;
        var w = size.Length > 0 ? size[0] : 1280;
        var h = size.Length > 1 ? size[1] : 720;

        mark = Stopwatch.GetTimestamp();
        var inRaid = Phase == SessionPhase.Raid && ActorId != Guid.Empty;
        var renderSnap = inRaid ? _store.RenderSnapshot(now, 0) : Snapshot;
        var local = renderSnap != null ? _store.LocalActor(renderSnap) : null;
        if (local == null && Snapshot != null)
            local = Snapshot.Players.FirstOrDefault(p => p.Id == ActorId);

        float aimX = 0f, aimY = 0f, aimAngle = 0f;
        void AimAtPointer(PlayerState? body)
        {
            if (Map == null) return;
            if (body != null && ActorId != Guid.Empty)
                _camera.Track(ActorId, body.X, body.Y, body.Z, now);
            else
                _camera.Follow(w, h, body, Map);
            _camera.ApplyOrigin(w, h);
            var planeZ = body?.Z ?? _camera.FocusZ;
            _camera.AimWorld(frame.PointerX, frame.PointerY, planeZ, out aimX, out aimY);
            var originX = body?.X ?? aimX;
            var originY = body?.Y ?? aimY;
            aimAngle = MathF.Atan2(aimY - originY, aimX - originX);
        }

        AimAtPointer(local);

        var inVehicle = local?.VehicleId != null;
        float moveX = frame.MoveX;
        float moveY = frame.MoveY;
        if (!inVehicle)
            IsoProjection.ScreenWasdToWorld(frame.MoveX, frame.MoveY, out moveX, out moveY);
        _probe.LogicMs = MsBetween(mark, Stopwatch.GetTimestamp());

        mark = Stopwatch.GetTimestamp();
        if (inRaid)
        {
            _simAccum += frameSec;
            _latchShoot |= frame.Shoot;
            _latchInteract |= frame.Interact;
        }
        else
        {
            _simAccum = 0;
            _latchShoot = false;
            _latchInteract = false;
        }

        if (inRaid && _simAccum > LocalPhysics.FixedDt &&
            (!_transport.IsConnected || PlayerId == Guid.Empty))
            _simAccum = LocalPhysics.FixedDt;

        if (_transport.IsConnected && PlayerId != Guid.Empty)
        {
            if (inRaid && _simAccum >= LocalPhysics.FixedDt)
            {
                _simAccum -= LocalPhysics.FixedDt;
                if (_simAccum > LocalPhysics.FixedDt)
                    _simAccum = LocalPhysics.FixedDt;
                var pending = new PendingInput
                {
                    Seq = _seq++,
                    Dt = LocalPhysics.FixedDt,
                    MoveX = moveX,
                    MoveY = moveY,
                    AscendDescend = frame.AscendDescend,
                    AimAngle = aimAngle,
                    Shoot = _latchShoot,
                    Interact = _latchInteract,
                    Brake = frame.Brake
                };
                _latchShoot = frame.Shoot;
                _latchInteract = false;
                await _transport.SendAsync(new ClientMessage.Input
                {
                    Seq = pending.Seq,
                    Dt = pending.Dt,
                    MoveX = pending.MoveX,
                    MoveY = pending.MoveY,
                    AscendDescend = pending.AscendDescend,
                    AimAngle = pending.AimAngle,
                    Shoot = pending.Shoot,
                    Interact = pending.Interact,
                    Brake = pending.Brake
                });
                _store.QueueAndPredict(pending);
            }

            if (inRaid)
            {
                if (frame.SwapSlot is >= 1 and <= 4)
                    await TrySwapSlot(frame.SwapSlot);
                if (frame.SwapNext)
                    await TrySwapNext();
                if (frame.OrderPing)
                {
                    var order = (_orderCycle % 3) switch
                    {
                        0 => OrderType.Hold,
                        1 => OrderType.Follow,
                        _ => OrderType.Engage
                    };
                    _orderCycle++;
                    await SendOrder(order, aimX, aimY, local?.Z ?? 0f);
                }
                if (frame.OrderMove)
                    await SendOrder(OrderType.MoveOrDriveTo, aimX, aimY, local?.Z ?? 0f);
            }

            if (now - _lastPingTs >= PingEveryMs)
            {
                _lastPingTs = now;
                await _transport.SendAsync(new ClientMessage.Ping { ClientMs = (ulong)now });
            }
        }
        _probe.SendMs = MsBetween(mark, Stopwatch.GetTimestamp());

        if (inRaid)
            _store.RenderAlpha = (float)Math.Clamp(_simAccum / LocalPhysics.FixedDt, 0, 1);

        if (inRaid && Map != null)
        {
            mark = Stopwatch.GetTimestamp();
            renderSnap = _store.RenderSnapshot(now, aimAngle);
            var drawn = _store.LocalActor(renderSnap);
            AimAtPointer(drawn ?? local);
            var ghosts = Debug.Visible ? _store.BuildGhosts(now) : null;
            var scene = SceneComposer.Compose(
                w, h, Map, renderSnap, ActorId, aimX, aimY, aimAngle,
                _camera.FocusX, _camera.FocusY, _camera.FocusZ, ghosts, Debug.Visible);
            _probe.ComposeMs = MsBetween(mark, Stopwatch.GetTimestamp());
            _probe.DrawOps = scene.Ops.Count;
            mark = Stopwatch.GetTimestamp();
            await _renderer.DrawAsync(scene);
            _probe.DrawMs = MsBetween(mark, Stopwatch.GetTimestamp());
            _probe.JsCalls++;
        }

        Debug.InputSeq = _seq;
        Debug.Connected = _transport.IsConnected;
        Debug.ServerTick = Snapshot?.Tick ?? 0;
        Debug.Players = Snapshot?.Players.Count ?? 0;
        Debug.Vehicles = Snapshot?.Vehicles.Count ?? 0;
        Debug.Projectiles = Snapshot?.Projectiles.Count ?? 0;
        if (local != null)
        {
            Debug.X = inRaid ? _store.Predicted.X : local.X;
            Debug.Y = inRaid ? _store.Predicted.Y : local.Y;
            Debug.Z = inRaid ? _store.Predicted.Z : local.Z;
            Debug.YawDeg = aimAngle * 180f / MathF.PI;
            Debug.InVehicle = inVehicle;
        }
        if (inRaid && _store.HasPredicted)
        {
            Debug.PredX = _store.Predicted.X;
            Debug.PredY = _store.Predicted.Y;
            Debug.PredZ = _store.Predicted.Z;
            var srv = _store.LastServerLocal;
            if (srv != null)
            {
                Debug.ServerX = srv.X;
                Debug.ServerY = srv.Y;
                Debug.ServerZ = srv.Z;
                var dx = Debug.PredX - srv.X;
                var dy = Debug.PredY - srv.Y;
                var dz = Debug.PredZ - srv.Z;
                Debug.PredError = MathF.Sqrt(dx * dx + dy * dy + dz * dz);
            }
            Debug.Pending = _store.PendingCount;
            Debug.AckSeq = _store.LastAckSeq;
        }
    }

    void OnMessage(ServerMessage message)
    {
        switch (message)
        {
            case ServerMessage.Welcome welcome:
                PlayerId = welcome.PlayerId;
                ActorId = welcome.ActorId ?? Guid.Empty;
                Map = welcome.Map;
                _store.Map = welcome.Map;
                Status = "hideout";
                if (Phase == SessionPhase.Connecting || Phase == SessionPhase.Menu)
                    Phase = SessionPhase.Hideout;
                break;
            case ServerMessage.ProfileState profile:
                Cash = profile.Cash;
                Stash = profile.Stash;
                DeployedLoadout = profile.DeployedLoadout;
                ApplyRoster(profile.Crew);
                if (profile.SquadId is { } sid && sid != Guid.Empty)
                    SquadId = sid;
                if (profile.ActorId is { } aid && aid != Guid.Empty)
                {
                    AdoptActor(aid);
                    Phase = SessionPhase.Raid;
                    Status = "in-city";
                }
                else if (Phase == SessionPhase.Raid)
                {
                    ActorId = Guid.Empty;
                    _store.Reset();
                    _camera.Reset();
                }
                SelectedWeaponId ??= Stash.FirstOrDefault(i => i.ItemType.Equals("Weapon", StringComparison.OrdinalIgnoreCase))?.Id;
                SelectedMedId ??= Stash.FirstOrDefault(i => i.ItemType.Equals("Med", StringComparison.OrdinalIgnoreCase))?.Id;
                break;
            case ServerMessage.Snapshot snap:
                Snapshot = snap;
                if (snap.SquadId is { } snapSquad && snapSquad != Guid.Empty)
                    SquadId = snapSquad;
                if (snap.ControlledActorId is { } controlled && controlled != Guid.Empty)
                    AdoptActor(controlled);
                _store.ApplySnapshot(snap, _clock.ElapsedMilliseconds);
                _snapCount++;
                var t = _clock.Elapsed.TotalSeconds;
                var window = t - _snapWindowStart;
                if (window >= 0.5)
                {
                    Debug.SnapshotHz = _snapCount / window;
                    _snapCount = 0;
                    _snapWindowStart = t;
                }
                break;
            case ServerMessage.RaidEnded ended:
                LastRaid = ended;
                Phase = SessionPhase.Summary;
                Status = ended.Outcome;
                ActorId = Guid.Empty;
                OrderTargetId = null;
                _store.Reset();
                _store.Map = Map;
                _camera.Reset();
                Snapshot = null;
                break;
            case ServerMessage.Pong pong:
                var rtt = _clock.ElapsedMilliseconds - (long)pong.ClientMs;
                if (rtt >= 0 && rtt < 5000)
                    Debug.PingMs = Debug.PingMs < 0 ? (int)rtt : (int)(Debug.PingMs * 0.7 + rtt * 0.3);
                Debug.ServerTick = pong.ServerTick;
                break;
            case ServerMessage.Event ev:
                LastEvent = $"{ev.Kind}: {ev.Message}";
                if (ev.Kind == "deploy" && Guid.TryParse(ev.Message, out var deployedId))
                {
                    AdoptActor(deployedId);
                    _store.Map = Map;
                    Phase = SessionPhase.Raid;
                    Status = "in-city";
                }
                if (ev.Kind == "swap" && Guid.TryParse(ev.Message, out var swappedId))
                    AdoptActor(swappedId);
                break;
        }
    }

    void ApplyRoster(List<CrewMember> crew)
    {
        Roster = crew;
        var ids = crew.Select(c => c.ActorId).ToHashSet();
        if (!_rosterReady)
        {
            SelectedCrewIds = crew.Select(c => c.ActorId).ToList();
            _rosterReady = crew.Count > 0;
            return;
        }
        SelectedCrewIds = SelectedCrewIds.Where(ids.Contains).ToList();
        if (SelectedCrewIds.Count == 0 && crew.Count > 0)
            SelectedCrewIds.Add(crew[0].ActorId);
    }

    void AdoptActor(Guid actorId)
    {
        if (actorId == Guid.Empty || actorId == ActorId) return;
        ActorId = actorId;
        _store.Anchor(actorId);
        OrderTargetId = null;
    }

    async Task TrySwapSlot(int slot)
    {
        var crew = Snapshot?.Crew;
        if (crew == null || slot > crew.Count) return;
        var member = crew[slot - 1];
        if (member.ActorId == ActorId || member.Hp <= 0) return;
        if (!member.IsAi) return;
        await _transport.SendAsync(new ClientMessage.SwapActor { TargetActorId = member.ActorId });
    }

    async Task TrySwapNext()
    {
        var crew = Snapshot?.Crew;
        if (crew == null || crew.Count == 0) return;
        var idx = crew.FindIndex(c => c.ActorId == ActorId);
        for (var n = 1; n <= crew.Count; n++)
        {
            var member = crew[(idx + n + crew.Count) % crew.Count];
            if (member.Hp <= 0 || member.ActorId == ActorId) continue;
            if (!member.IsAi) continue;
            await _transport.SendAsync(new ClientMessage.SwapActor { TargetActorId = member.ActorId });
            return;
        }
    }

    async Task SendOrder(OrderType order, float x, float y, float z)
    {
        await _transport.SendAsync(new ClientMessage.SquadOrder
        {
            OrderType = order,
            TargetActorId = OrderTargetId,
            X = x,
            Y = y,
            Z = z
        });
    }

    public async ValueTask DisposeAsync()
    {
        _running = false;
        _transport.MessageReceived -= OnMessage;
        await _transport.DisposeAsync();
    }
}
