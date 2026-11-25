using ScarletCity.Client.Rendering;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

public sealed class IsoCamera
{
    public const double SwapBlendMs = 250;

    public float OriginX { get; private set; }
    public float OriginY { get; private set; }
    public float FocusX { get; private set; }
    public float FocusY { get; private set; }
    public float FocusZ { get; private set; }
    public float Scale => SceneComposer.Scale;
    public float ZScale => SceneComposer.ZScale;

    Guid _actorId;
    bool _has;
    bool _blending;
    double _blendStart;
    float _fromX, _fromY, _fromZ;

    public void Reset()
    {
        _has = false;
        _blending = false;
        _actorId = Guid.Empty;
    }

    /// <summary>
    /// Follow the controlled actor. A change of actor eases the camera over 250ms.
    /// </summary>
    public void Track(Guid actorId, float x, float y, float z, double nowMs)
    {
        if (!_has || actorId == Guid.Empty)
        {
            _has = actorId != Guid.Empty;
            _actorId = actorId;
            _blending = false;
            FocusX = x;
            FocusY = y;
            FocusZ = z;
            return;
        }
        if (actorId != _actorId)
        {
            _actorId = actorId;
            _fromX = FocusX;
            _fromY = FocusY;
            _fromZ = FocusZ;
            _blendStart = nowMs;
            _blending = true;
        }
        if (_blending)
        {
            var u = (nowMs - _blendStart) / SwapBlendMs;
            if (u >= 1)
            {
                _blending = false;
                FocusX = x;
                FocusY = y;
                FocusZ = z;
            }
            else
            {
                var s = (float)(u * u * (3 - 2 * u));
                FocusX = _fromX + (x - _fromX) * s;
                FocusY = _fromY + (y - _fromY) * s;
                FocusZ = _fromZ + (z - _fromZ) * s;
            }
        }
        else
        {
            FocusX = x;
            FocusY = y;
            FocusZ = z;
        }
    }

    public void Follow(int width, int height, PlayerState? focus, MapData? map)
    {
        if (focus != null)
        {
            FocusX = focus.X;
            FocusY = focus.Y;
            FocusZ = focus.Z;
        }
        else if (!_has)
        {
            FocusX = map?.Bounds.Width * 0.5f ?? 40f;
            FocusY = map?.Bounds.Depth * 0.5f ?? 40f;
            FocusZ = 0f;
        }
        ApplyOrigin(width, height);
    }

    public void ApplyOrigin(int width, int height)
    {
        IsoProjection.Project(FocusX, FocusY, FocusZ, Scale, ZScale, 0, 0, out var psx, out var psy);
        OriginX = width * 0.5f - psx;
        OriginY = height * 0.42f - psy;
    }

    public void AimWorld(float screenX, float screenY, float planeZ, out float x, out float y)
    {
        IsoProjection.Unproject(screenX, screenY, planeZ, Scale, ZScale, OriginX, OriginY, out x, out y);
    }
}
