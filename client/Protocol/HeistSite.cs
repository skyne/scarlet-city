// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class HeistSite
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("kind")]
    public HeistKind Kind { get; set; }

    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("width")]
    public float Width { get; set; }

    [JsonPropertyName("depth")]
    public float Depth { get; set; }

    [JsonPropertyName("z_min")]
    public float ZMin { get; set; }

    [JsonPropertyName("z_max")]
    public float ZMax { get; set; }

    [JsonPropertyName("quiet_x")]
    public float QuietX { get; set; }

    [JsonPropertyName("quiet_y")]
    public float QuietY { get; set; }

    [JsonPropertyName("quiet_z")]
    public float QuietZ { get; set; }

    [JsonPropertyName("quiet_secs")]
    public float QuietSecs { get; set; }

    [JsonPropertyName("loud_x")]
    public float LoudX { get; set; }

    [JsonPropertyName("loud_y")]
    public float LoudY { get; set; }

    [JsonPropertyName("loud_z")]
    public float LoudZ { get; set; }

    [JsonPropertyName("loud_secs")]
    public float LoudSecs { get; set; }

    [JsonPropertyName("loot_tier")]
    public LootTier LootTier { get; set; }

}

