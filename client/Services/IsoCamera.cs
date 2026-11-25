using ScarletCity.Client.Rendering;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

public sealed class IsoCamera
{
    public float OriginX { get; private set; }
    public float OriginY { get; private set; }
    public float Scale => SceneComposer.Scale;
    public float ZScale => SceneComposer.ZScale;

    public void Follow(int width, int height, PlayerState? focus, MapData? map)
    {
        var fx = focus?.X ?? map?.Bounds.Width * 0.5f ?? 40f;
        var fy = focus?.Y ?? map?.Bounds.Depth * 0.5f ?? 40f;
        var fz = focus?.Z ?? 0f;
        IsoProjection.Project(fx, fy, fz, Scale, ZScale, 0, 0, out var psx, out var psy);
        OriginX = width * 0.5f - psx;
        OriginY = height * 0.42f - psy;
    }

    public void AimWorld(float screenX, float screenY, float planeZ, out float x, out float y)
    {
        IsoProjection.Unproject(screenX, screenY, planeZ, Scale, ZScale, OriginX, OriginY, out x, out y);
    }
}
