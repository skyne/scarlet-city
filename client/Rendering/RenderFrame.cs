using ScarletCity.Protocol;

namespace ScarletCity.Client.Rendering;

public sealed class RenderFrame
{
    public int Width { get; set; }
    public int Height { get; set; }
    public string Clear { get; set; } = "#140c10";
    public List<DrawOp> Ops { get; set; } = new();
}

public sealed class DrawOp
{
    public string Kind { get; set; } = "poly";
    public float[] Pts { get; set; } = Array.Empty<float>();
    public string Fill { get; set; } = "#333";
    public string Stroke { get; set; } = "transparent";
    public string? Text { get; set; }
    public float Size { get; set; }
    public float Alpha { get; set; } = 1f;
    public string? Asset { get; set; }
    public string? Anchor { get; set; }
    public float Rotation { get; set; }
    public List<ActorDraw>? Actors { get; set; }
}

/// <summary>
/// One skinned character for the WebGL pass. Pts on the parent op are
/// originX, originY, scale, zScale.
/// </summary>
public sealed class ActorDraw
{
    public string Id { get; set; } = "";
    public float X { get; set; }
    public float Y { get; set; }
    public float Z { get; set; }
    public float Yaw { get; set; }
    public float Speed { get; set; }
    public int Variant { get; set; }
    public CharacterAppearance? Appearance { get; set; }
}
