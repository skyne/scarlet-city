using ScarletCity.Protocol;

namespace ScarletCity.Client.Simulation;

/// <summary>
/// Client copy of server movement/vehicle integration for prediction.
/// Constants must stay aligned with <c>server/src/sim.rs</c>.
/// </summary>
public static class LocalPhysics
{
    public const float FixedDt = 1f / 20f;
    public const float PlayerRadius = 0.45f;
    public const float PlayerHeight = 1.8f;
    public const float PlayerSpeed = 8f;
    public const float CarRadius = 1.1f;
    public const float CarHeight = 1.4f;
    public const float CarAccel = 18f;
    public const float CarMaxSpeed = 22f;
    public const float CarTurn = 2.6f;
    public const float CarGrip = 6f;
    public const float HoverRadius = 1.2f;
    public const float HoverHeight = 1.2f;
    public const float HoverAccel = 14f;
    public const float HoverMaxSpeed = 16f;
    public const float HoverClimb = 7f;
    public const float HoverTurn = 2.2f;

    public sealed class Pose
    {
        public float X, Y, Z, Yaw, Vx, Vy, Vz;
        public Guid? VehicleId;
    }

    public sealed class VehiclePose
    {
        public Guid Id;
        public VehicleKind Kind;
        public float X, Y, Z, Yaw, VelYaw, Speed;
        public Guid? DriverId;
    }

    public static void Walk(Pose p, MapData map, float moveX, float moveY, float dt)
    {
        var mag = MathF.Sqrt(moveX * moveX + moveY * moveY);
        if (mag > 1f)
        {
            moveX /= mag;
            moveY /= mag;
        }
        var ox = p.X;
        var oy = p.Y;
        var nx = ox + moveX * PlayerSpeed * dt;
        var ny = oy + moveY * PlayerSpeed * dt;
        var z = map.GroundZ;
        TryMove(map, ox, oy, nx, ny, PlayerRadius, z, z + PlayerHeight, out var x, out var y);
        Clamp(map, PlayerRadius, ref x, ref y);
        p.Vx = (x - ox) / dt;
        p.Vy = (y - oy) / dt;
        p.Vz = (z - p.Z) / dt;
        p.X = x;
        p.Y = y;
        p.Z = z;
    }

    public static void Drive(Pose p, VehiclePose v, MapData map, float moveX, float moveY,
        float ascend, bool brake, float dt)
    {
        float radius, height, accel, maxSpeed, turn, grip;
        if (v.Kind == VehicleKind.GroundCar)
        {
            radius = CarRadius; height = CarHeight; accel = CarAccel;
            maxSpeed = CarMaxSpeed; turn = CarTurn; grip = CarGrip;
        }
        else
        {
            radius = HoverRadius; height = HoverHeight; accel = HoverAccel;
            maxSpeed = HoverMaxSpeed; turn = HoverTurn; grip = 10f;
        }

        var speed = v.Speed + moveY * accel * dt;
        if (brake) speed *= 0.82f;
        speed = Math.Clamp(speed, -maxSpeed * 0.4f, maxSpeed);
        var yaw = v.Yaw + moveX * turn * dt * MathF.Max(MathF.Abs(speed) / maxSpeed, 0.25f);
        var velYaw = LerpAngle(v.VelYaw, yaw, MathF.Min(grip * dt, 1f));
        var ox = v.X;
        var oy = v.Y;
        var oz = v.Z;
        var nx = ox + MathF.Cos(velYaw) * speed * dt;
        var ny = oy + MathF.Sin(velYaw) * speed * dt;
        var z = v.Kind == VehicleKind.GroundCar
            ? map.GroundZ
            : Math.Clamp(v.Z + Math.Clamp(ascend, -1f, 1f) * HoverClimb * dt, map.GroundZ, map.MaxAltitude);
        TryMove(map, ox, oy, nx, ny, radius, z, z + height, out var x, out var y);
        Clamp(map, radius, ref x, ref y);
        if (Blocked(map, x, y, radius, z, z + height))
            z = oz;
        v.X = x; v.Y = y; v.Z = z; v.Yaw = yaw; v.VelYaw = velYaw; v.Speed = speed;
        p.X = x; p.Y = y; p.Z = z; p.Yaw = yaw;
        p.Vx = (x - ox) / dt;
        p.Vy = (y - oy) / dt;
        p.Vz = (z - oz) / dt;
    }

    static void TryMove(MapData map, float x, float y, float nx, float ny, float radius,
        float z0, float z1, out float ox, out float oy)
    {
        ox = nx;
        oy = ny;
        if (Blocked(map, ox, oy, radius, z0, z1))
        {
            if (!Blocked(map, nx, y, radius, z0, z1))
            {
                ox = nx;
                oy = y;
            }
            else if (!Blocked(map, x, ny, radius, z0, z1))
            {
                ox = x;
                oy = ny;
            }
            else
            {
                ox = x;
                oy = y;
            }
        }
    }

    static void Clamp(MapData map, float radius, ref float x, ref float y)
    {
        var b = map.Bounds;
        x = Math.Clamp(x, b.X + radius, b.X + b.Width - radius);
        y = Math.Clamp(y, b.Y + radius, b.Y + b.Depth - radius);
    }

    static bool Blocked(MapData map, float x, float y, float radius, float z0, float z1)
    {
        foreach (var o in map.Obstacles)
        {
            if (z0 >= o.ZMax || o.ZMin >= z1) continue;
            var nearestX = Math.Clamp(x, o.X, o.X + o.Width);
            var nearestY = Math.Clamp(y, o.Y, o.Y + o.Depth);
            var dx = x - nearestX;
            var dy = y - nearestY;
            if (dx * dx + dy * dy < radius * radius)
                return true;
        }
        return false;
    }

    static float LerpAngle(float from, float to, float t)
    {
        var diff = to - from;
        while (diff > MathF.PI) diff -= MathF.Tau;
        while (diff < -MathF.PI) diff += MathF.Tau;
        return from + diff * t;
    }
}
