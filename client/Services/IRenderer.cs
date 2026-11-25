using ScarletCity.Client.Rendering;

namespace ScarletCity.Client.Services;

public interface IRenderer
{
    Task<int[]> GetSizeAsync();
    Task DrawAsync(RenderFrame frame);
}
