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
}
