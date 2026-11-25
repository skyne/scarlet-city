using System.Diagnostics;
using ScarletCity.Client.Rendering;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

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
            $"vehicle    {(InVehicle ? "yes" : "on foot")}\n" +
            $"ents       players {Players}  vehicles {Vehicles}  shots {Projectiles}";
    }
}

public sealed class GameSession : IAsyncDisposable
{
    readonly IGameTransport _transport;
    readonly IRenderer _renderer;
    readonly IInputSource _input;
    readonly IsoCamera _camera;
    readonly Stopwatch _clock = Stopwatch.StartNew();

    public Guid PlayerId { get; private set; }
    public MapData? Map { get; private set; }
    public ServerMessage.Snapshot? Snapshot { get; private set; }
    public string Status { get; private set; } = "disconnected";
    public string LastEvent { get; set; } = "";
    public DebugStats Debug { get; } = new();

    uint _seq;
    bool _running;
    long _lastFrameTs;
    double _fpsEma;
    int _snapCount;
    double _snapWindowStart;
    long _lastPingTs;
    const long PingEveryMs = 500;

    public GameSession(IGameTransport transport, IRenderer renderer, IInputSource input, IsoCamera camera)
    {
        _transport = transport;
        _renderer = renderer;
        _input = input;
        _camera = camera;
        _transport.MessageReceived += OnMessage;
    }

    public async Task StartAsync(string wsUrl, string name)
    {
        Status = "connecting";
        await _input.AttachAsync();
        await _transport.ConnectAsync(new Uri(wsUrl));
        await _transport.SendAsync(new ClientMessage.Join { Name = name });
        Status = "connected";
        _running = true;
        _lastFrameTs = _clock.ElapsedMilliseconds;
        _snapWindowStart = _clock.Elapsed.TotalSeconds;
        _ = Loop();
    }

    async Task Loop()
    {
        while (_running)
        {
            try
            {
                await TickOnce();
            }
            catch
            {
                Status = "loop-error";
            }
            await Task.Delay(16);
        }
    }

    async Task TickOnce()
    {
        var now = _clock.ElapsedMilliseconds;
        var dtMs = Math.Max(1, now - _lastFrameTs);
        _lastFrameTs = now;
        var instFps = 1000.0 / dtMs;
        _fpsEma = _fpsEma <= 1 ? instFps : _fpsEma * 0.9 + instFps * 0.1;

        var frame = await _input.PollAsync();
        if (frame.ToggleDebug)
            Debug.Visible = !Debug.Visible;

        var size = await _renderer.GetSizeAsync();
        var w = size.Length > 0 ? size[0] : 1280;
        var h = size.Length > 1 ? size[1] : 720;

        var local = Snapshot?.Players.FirstOrDefault(p => p.Id == PlayerId);
        if (Map != null)
            _camera.Follow(w, h, local, Map);

        var planeZ = local?.Z ?? 0f;
        _camera.AimWorld(frame.PointerX, frame.PointerY, planeZ, out var aimX, out var aimY);
        var originX = local?.X ?? aimX;
        var originY = local?.Y ?? aimY;
        var aimAngle = MathF.Atan2(aimY - originY, aimX - originX);

        var inVehicle = local?.VehicleId != null;
        float moveX = frame.MoveX;
        float moveY = frame.MoveY;
        if (!inVehicle)
            IsoProjection.ScreenWasdToWorld(frame.MoveX, frame.MoveY, out moveX, out moveY);

        if (_transport.IsConnected && PlayerId != Guid.Empty)
        {
            await _transport.SendAsync(new ClientMessage.Input
            {
                Seq = _seq++,
                MoveX = moveX,
                MoveY = moveY,
                AscendDescend = frame.AscendDescend,
                AimAngle = aimAngle,
                Shoot = frame.Shoot,
                Interact = frame.Interact,
                Brake = frame.Brake
            });

            if (now - _lastPingTs >= PingEveryMs)
            {
                _lastPingTs = now;
                await _transport.SendAsync(new ClientMessage.Ping { ClientMs = (ulong)now });
            }
        }

        if (Map != null && Snapshot != null)
        {
            var scene = SceneComposer.Compose(w, h, Map, Snapshot, PlayerId, aimX, aimY, aimAngle);
            await _renderer.DrawAsync(scene);
        }

        Debug.Fps = _fpsEma;
        Debug.FrameMs = dtMs;
        Debug.InputSeq = _seq;
        Debug.Connected = _transport.IsConnected;
        Debug.ServerTick = Snapshot?.Tick ?? 0;
        Debug.Players = Snapshot?.Players.Count ?? 0;
        Debug.Vehicles = Snapshot?.Vehicles.Count ?? 0;
        Debug.Projectiles = Snapshot?.Projectiles.Count ?? 0;
        if (local != null)
        {
            Debug.X = local.X;
            Debug.Y = local.Y;
            Debug.Z = local.Z;
            Debug.YawDeg = aimAngle * 180f / MathF.PI;
            Debug.InVehicle = inVehicle;
        }
    }

    void OnMessage(ServerMessage message)
    {
        switch (message)
        {
            case ServerMessage.Welcome welcome:
                PlayerId = welcome.PlayerId;
                Map = welcome.Map;
                Status = "in-city";
                break;
            case ServerMessage.Snapshot snap:
                Snapshot = snap;
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
            case ServerMessage.Pong pong:
                var rtt = _clock.ElapsedMilliseconds - (long)pong.ClientMs;
                if (rtt >= 0 && rtt < 5000)
                    Debug.PingMs = Debug.PingMs < 0 ? (int)rtt : (int)(Debug.PingMs * 0.7 + rtt * 0.3);
                Debug.ServerTick = pong.ServerTick;
                break;
            case ServerMessage.Event ev:
                LastEvent = $"{ev.Kind}: {ev.Message}";
                break;
        }
    }

    public async ValueTask DisposeAsync()
    {
        _running = false;
        _transport.MessageReceived -= OnMessage;
        await _transport.DisposeAsync();
    }
}
