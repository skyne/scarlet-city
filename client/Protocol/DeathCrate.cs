// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class DeathCrate
{
    [JsonPropertyName("id")]
    public Guid Id { get; set; }

    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("z")]
    public float Z { get; set; }

    [JsonPropertyName("items")]
    public List<StashItem> Items { get; set; } = new List<StashItem>();

}

