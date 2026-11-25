// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v2

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class PlayerState
{
    [JsonPropertyName("id")]
    public Guid Id { get; set; }

    [JsonPropertyName("player_id")]
    public Guid PlayerId { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("x")]
    public float X { get; set; }

    [JsonPropertyName("y")]
    public float Y { get; set; }

    [JsonPropertyName("z")]
    public float Z { get; set; }

    [JsonPropertyName("yaw")]
    public float Yaw { get; set; }

    [JsonPropertyName("vx")]
    public float Vx { get; set; }

    [JsonPropertyName("vy")]
    public float Vy { get; set; }

    [JsonPropertyName("vz")]
    public float Vz { get; set; }

    [JsonPropertyName("health")]
    public float Health { get; set; }

    [JsonPropertyName("armor")]
    public float Armor { get; set; }

    [JsonPropertyName("backpack")]
    public List<StashItem> Backpack { get; set; } = new List<StashItem>();

    [JsonPropertyName("vehicle_id")]
    public Guid? VehicleId { get; set; }

    [JsonPropertyName("extracting")]
    public bool Extracting { get; set; }

    [JsonPropertyName("extract_progress")]
    public float ExtractProgress { get; set; }

    [JsonPropertyName("last_processed_input_seq")]
    public uint LastProcessedInputSeq { get; set; }

}

