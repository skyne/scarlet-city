using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

/// <summary>
/// Platform-agnostic game connection. Browser uses WebSockets; a Tauri/Electron
/// host can swap this without touching simulation or HUD code.
/// </summary>
public interface IGameTransport : IAsyncDisposable
{
    event Action<ServerMessage>? MessageReceived;
    bool IsConnected { get; }
    Task ConnectAsync(Uri url, CancellationToken cancellationToken = default);
    Task SendAsync(ClientMessage message, CancellationToken cancellationToken = default);
}
