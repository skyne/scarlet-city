using Microsoft.JSInterop;
using ScarletCity.Client.Services;

namespace ScarletCity.Client.Interop;

public sealed class BrowserInput : IInputSource
{
    readonly IJSRuntime _js;

    public BrowserInput(IJSRuntime js) => _js = js;

    public Task AttachAsync() => _js.InvokeVoidAsync("ScarletInput.attach").AsTask();

    public Task<InputFrame> PollAsync() => _js.InvokeAsync<InputFrame>("ScarletInput.poll").AsTask();
}
