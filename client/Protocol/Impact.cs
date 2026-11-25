// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v2

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class Impact
{
    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("z")]
    public float Z { get; set; }

    [JsonPropertyName("from_x")]
    public float FromX { get; set; }

    [JsonPropertyName("from_y")]
    public float FromY { get; set; }

    [JsonPropertyName("from_z")]
    public float FromZ { get; set; }

    [JsonPropertyName("hit_kind")]
    public HitKind HitKind { get; set; }

}

