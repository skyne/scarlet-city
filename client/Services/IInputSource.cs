namespace ScarletCity.Client.Services;

public sealed class InputFrame
{
    public float MoveX { get; set; }
    public float MoveY { get; set; }
    public float AscendDescend { get; set; }
    public float PointerX { get; set; }
    public float PointerY { get; set; }
    public bool Shoot { get; set; }
    public bool Interact { get; set; }
    public bool Brake { get; set; }
    public bool ToggleDebug { get; set; }
    public int CanvasWidth { get; set; }
    public int CanvasHeight { get; set; }
}

public interface IInputSource
{
    Task AttachAsync();
    Task<InputFrame> PollAsync();
}
