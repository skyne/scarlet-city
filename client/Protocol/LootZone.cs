// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v2

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class LootZone
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

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

    [JsonPropertyName("tier")]
    public LootTier Tier { get; set; }

}

