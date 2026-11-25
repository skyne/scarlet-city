// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class HeistMarker
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("kind")]
    public HeistKind Kind { get; set; }

    [JsonPropertyName("state")]
    public HeistState State { get; set; }

    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("z")]
    public float Z { get; set; }

    [JsonPropertyName("quiet_x")]
    public float QuietX { get; set; }

    [JsonPropertyName("quiet_y")]
    public float QuietY { get; set; }

    [JsonPropertyName("quiet_z")]
    public float QuietZ { get; set; }

    [JsonPropertyName("loud_x")]
    public float LoudX { get; set; }

    [JsonPropertyName("loud_y")]
    public float LoudY { get; set; }

    [JsonPropertyName("loud_z")]
    public float LoudZ { get; set; }

    [JsonPropertyName("progress")]
    public float Progress { get; set; }

    [JsonPropertyName("approach")]
    public string Approach { get; set; } = string.Empty;

    [JsonPropertyName("alarm")]
    public bool Alarm { get; set; }

    [JsonPropertyName("owner_squad")]
    public Guid? OwnerSquad { get; set; }

}

