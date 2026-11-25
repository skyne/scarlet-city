// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v3

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class MapData
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("bounds")]
    public MapBounds Bounds { get; set; } = null!;

    [JsonPropertyName("ground_z")]
    public float GroundZ { get; set; }

    [JsonPropertyName("max_altitude")]
    public float MaxAltitude { get; set; }

    [JsonPropertyName("roads")]
    public List<Road> Roads { get; set; } = new List<Road>();

    [JsonPropertyName("obstacles")]
    public List<Obstacle> Obstacles { get; set; } = new List<Obstacle>();

    [JsonPropertyName("vehicle_spawns")]
    public List<VehicleSpawn> VehicleSpawns { get; set; } = new List<VehicleSpawn>();

    [JsonPropertyName("loot_zones")]
    public List<LootZone> LootZones { get; set; } = new List<LootZone>();

    [JsonPropertyName("exfil_zones")]
    public List<ExfilZone> ExfilZones { get; set; } = new List<ExfilZone>();

}

