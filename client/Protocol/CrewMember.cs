// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class CrewMember
{
    [JsonPropertyName("actor_id")]
    public Guid ActorId { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("role")]
    public string Role { get; set; } = string.Empty;

    [JsonPropertyName("hp")]
    public float Hp { get; set; }

    [JsonPropertyName("max_hp")]
    public float MaxHp { get; set; }

    [JsonPropertyName("is_ai")]
    public bool IsAi { get; set; }

    [JsonPropertyName("order_label")]
    public string OrderLabel { get; set; } = string.Empty;

    [JsonPropertyName("order_x")]
    public float OrderX { get; set; }

    [JsonPropertyName("order_y")]
    public float OrderY { get; set; }

    [JsonPropertyName("order_z")]
    public float OrderZ { get; set; }

    [JsonPropertyName("appearance")]
    public CharacterAppearance Appearance { get; set; } = null!;

}

