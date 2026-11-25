// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v3

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class Projectile
{
    [JsonPropertyName("id")]
    public Guid Id { get; set; }

    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("z")]
    public float Z { get; set; }

    [JsonPropertyName("vx")]
    public float Vx { get; set; }

    [JsonPropertyName("vy")]
    public float Vy { get; set; }

    [JsonPropertyName("vz")]
    public float Vz { get; set; }

    [JsonPropertyName("owner_id")]
    public Guid OwnerId { get; set; }

}

