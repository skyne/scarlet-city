using Microsoft.JSInterop;
using ScarletCity.Client.Rendering;
using ScarletCity.Client.Services;

namespace ScarletCity.Client.Interop;

public sealed class CanvasRenderer : IRenderer
{
    readonly IJSRuntime _js;

    public CanvasRenderer(IJSRuntime js) => _js = js;

    public async Task<int[]> GetSizeAsync() =>
        await _js.InvokeAsync<int[]>("ScarletCanvas.size");

    public async Task DrawAsync(RenderFrame frame) =>
        await _js.InvokeVoidAsync("ScarletCanvas.draw", frame);
}
