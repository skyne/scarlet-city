// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class NpcState
{
    [JsonPropertyName("id")]
    public Guid Id { get; set; }

    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("z")]
    public float Z { get; set; }

    [JsonPropertyName("yaw")]
    public float Yaw { get; set; }

    [JsonPropertyName("health")]
    public float Health { get; set; }

    [JsonPropertyName("alert")]
    public bool Alert { get; set; }

    [JsonPropertyName("vehicle_id")]
    public Guid? VehicleId { get; set; }

}

