using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using ScarletCity.Client;
using ScarletCity.Client.Interop;
using ScarletCity.Client.Services;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

builder.Services.AddScoped(sp => new HttpClient { BaseAddress = new Uri(builder.HostEnvironment.BaseAddress) });
builder.Services.AddScoped<IGameTransport, WebSocketTransport>();
builder.Services.AddScoped<IRenderer, CanvasRenderer>();
builder.Services.AddScoped<IInputSource, BrowserInput>();
builder.Services.AddScoped<IsoCamera>();
builder.Services.AddScoped<GameStateStore>();
builder.Services.AddScoped<GameSession>();

await builder.Build().RunAsync();
