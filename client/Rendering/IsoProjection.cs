namespace ScarletCity.Client.Rendering;

/// <summary>
/// 2:1 isometric projection with altitude. Screen Y subtracts world Z so flyers
/// sit above their ground drop-shadow.
/// </summary>
public static class IsoProjection
{
    public static void Project(
        float x,
        float y,
        float z,
        float scale,
        float zScale,
        float originX,
        float originY,
        out float screenX,
        out float screenY)
    {
        screenX = (x - y) * scale + originX;
        screenY = (x + y) * 0.5f * scale - z * zScale + originY;
    }

    /// <summary>
    /// Inverse of <see cref="Project"/> onto a constant-Z plane (usually ground z = 0).
    /// </summary>
    public static void Unproject(
        float screenX,
        float screenY,
        float planeZ,
        float scale,
        float zScale,
        float originX,
        float originY,
        out float x,
        out float y)
    {
        var a = (screenX - originX) / scale;
        var b = 2f * (screenY - originY + planeZ * zScale) / scale;
        x = (a + b) * 0.5f;
        y = (b - a) * 0.5f;
    }

    /// <summary>
    /// Convert screen-space WASD (W = up the monitor, D = right) into world
    /// <c>move_x/move_y</c> so walking matches the isometric camera.
    /// </summary>
    public static void ScreenWasdToWorld(float screenRight, float screenUp, out float worldX, out float worldY)
    {
        // Screen up  → world (-1, -1); screen right → world (+1, -1)
        worldX = -screenUp + screenRight;
        worldY = -screenUp - screenRight;
        var mag = MathF.Sqrt(worldX * worldX + worldY * worldY);
        if (mag > 1e-4f)
        {
            worldX /= mag;
            worldY /= mag;
        }
    }
}
