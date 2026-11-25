// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

public sealed class StashItem
{
    [JsonPropertyName("id")]
    public Guid Id { get; set; }

    [JsonPropertyName("item_type")]
    public string ItemType { get; set; } = string.Empty;

    [JsonPropertyName("item_name")]
    public string ItemName { get; set; } = string.Empty;

    [JsonPropertyName("quantity")]
    public uint Quantity { get; set; }

    [JsonPropertyName("stats_json")]
    public string StatsJson { get; set; } = string.Empty;

}

