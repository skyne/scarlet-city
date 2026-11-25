using ScarletCity.Protocol;

namespace ScarletCity.Client.Rendering;

public static class WorldAssetCatalog
{
    static readonly Dictionary<string, string> Buildings = new(StringComparer.OrdinalIgnoreCase)
    {
        ["warehouse_nw"] = "quaternius/buildings_pack_2/building1_large",
        ["tower_ne"] = "quaternius/buildings_pack_3/6story_stack_mat",
        ["low_shed"] = "kit-cidade/industrial/galpao",
        ["bridge_deck"] = "quaternius/street_pack/street_bridge",
        ["bridge_pylon_w"] = "quaternius/cyberpunk_pack/support",
        ["bridge_pylon_e"] = "quaternius/cyberpunk_pack/support",
        ["rooftop_block_sw"] = "quaternius/buildings_pack_3/4story_mat",
        ["corner_shop"] = "kit-cidade/comercial/loja_esquina",
        ["east_block"] = "quaternius/buildings_pack_2/building3_big",
        ["east_overpass"] = "quaternius/street_pack/street_elevated",
        ["east_overpass_pylon_w"] = "quaternius/cyberpunk_pack/support",
        ["east_overpass_pylon_e"] = "quaternius/cyberpunk_pack/support",
        ["mid_roof"] = "quaternius/buildings_pack_3/2story_balcony_mat",
        ["east_tower"] = "quaternius/buildings_pack_3/6story_stack_mat",
        ["south_block"] = "quaternius/buildings_pack_2/building4",
        ["far_shed"] = "quaternius/buildings_pack_2/house2"
    };

    static readonly string[] Cars =
    {
        "quaternius/cars/car",
        "quaternius/cars/taxi",
        "quaternius/cars/suv",
        "quaternius/cars/sports_car",
        "kit-cidade/veiculos/carro",
        "kit-cidade/veiculos/carro_eletrico"
    };

    static readonly Dictionary<string, string> Weapons = new(StringComparer.OrdinalIgnoreCase)
    {
        ["none"] = "",
        ["pistol"] = "quaternius/guns/pistol",
        ["revolver"] = "quaternius/guns/revolver",
        ["shotgun"] = "quaternius/guns/shotgun",
        ["smg"] = "quaternius/guns/submachine_gun",
        ["rifle"] = "quaternius/guns/assault_rifle",
        ["sniper"] = "quaternius/guns/sniper_rifle",
        ["bullpup"] = "quaternius/guns/bullpup"
    };

    public static string? Building(string id) =>
        Buildings.TryGetValue(id, out var asset) ? asset : null;

    public static string Vehicle(VehicleKind kind, bool law, Guid id)
    {
        if (law) return "quaternius/cars/police_car";
        if (kind == VehicleKind.HoverCraft) return "kit-cidade/veiculos/carro_eletrico";
        var index = Math.Abs(id.GetHashCode()) % Cars.Length;
        return Cars[index];
    }

    public static string? Weapon(string? id) =>
        id != null && Weapons.TryGetValue(id, out var asset) ? asset : null;

    public static string NpcWeapon(string kind) => kind switch
    {
        "Swat" => "quaternius/guns/assault_rifle",
        "Police" => "quaternius/guns/pistol",
        _ => "quaternius/guns/revolver"
    };
}
