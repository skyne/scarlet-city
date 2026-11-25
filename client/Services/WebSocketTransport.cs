using System.Net.WebSockets;
using System.Text;
using System.Text.Json;
using ScarletCity.Protocol;

namespace ScarletCity.Client.Services;

public sealed class WebSocketTransport : IGameTransport
{
    ClientWebSocket? _socket;
    CancellationTokenSource? _run;

    public event Action<ServerMessage>? MessageReceived;
    public bool IsConnected => _socket?.State == WebSocketState.Open;

    public async Task ConnectAsync(Uri url, CancellationToken cancellationToken = default)
    {
        await DisposeAsync();
        _socket = new ClientWebSocket();
        _run = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        await _socket.ConnectAsync(url, _run.Token);
        _ = Task.Run(() => ReceiveLoop(_run.Token));
    }

    public async Task SendAsync(ClientMessage message, CancellationToken cancellationToken = default)
    {
        if (_socket is null || _socket.State != WebSocketState.Open)
            return;
        var json = JsonSerializer.Serialize(message, ProtocolJson.Options);
        var bytes = Encoding.UTF8.GetBytes(json);
        await _socket.SendAsync(bytes, WebSocketMessageType.Text, true, cancellationToken);
    }

    async Task ReceiveLoop(CancellationToken ct)
    {
        var buffer = new byte[1 << 16];
        var acc = new MemoryStream();
        try
        {
            while (!ct.IsCancellationRequested && _socket is { State: WebSocketState.Open })
            {
                var result = await _socket.ReceiveAsync(buffer, ct);
                if (result.MessageType == WebSocketMessageType.Close)
                    break;
                acc.Write(buffer, 0, result.Count);
                if (!result.EndOfMessage) continue;
                var json = Encoding.UTF8.GetString(acc.ToArray());
                acc.SetLength(0);
                var msg = JsonSerializer.Deserialize<ServerMessage>(json, ProtocolJson.Options);
                if (msg != null)
                    MessageReceived?.Invoke(msg);
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (WebSocketException)
        {
        }
    }

    public async ValueTask DisposeAsync()
    {
        _run?.Cancel();
        if (_socket != null)
        {
            try
            {
                if (_socket.State == WebSocketState.Open)
                    await _socket.CloseAsync(WebSocketCloseStatus.NormalClosure, "bye", CancellationToken.None);
            }
            catch
            {
                // ignored
            }
            _socket.Dispose();
            _socket = null;
        }
        _run?.Dispose();
        _run = null;
    }
}
