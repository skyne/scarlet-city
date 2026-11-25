// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v2

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(ClientMessage.Join), "Join")]
[JsonDerivedType(typeof(ClientMessage.GetProfile), "GetProfile")]
[JsonDerivedType(typeof(ClientMessage.Deploy), "Deploy")]
[JsonDerivedType(typeof(ClientMessage.Input), "Input")]
[JsonDerivedType(typeof(ClientMessage.Ping), "Ping")]
public abstract class ClientMessage
{
    public sealed class Join : ClientMessage
    {
        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

    }

    public sealed class GetProfile : ClientMessage
    {
    }

    public sealed class Deploy : ClientMessage
    {
        [JsonPropertyName("loadout_item_ids")]
        public List<Guid> LoadoutItemIds { get; set; } = new List<Guid>();

    }

    public sealed class Input : ClientMessage
    {
        [JsonPropertyName("seq")]
        public uint Seq { get; set; }

        [JsonPropertyName("dt")]
        public float Dt { get; set; }

        [JsonPropertyName("move_x")]
        public float MoveX { get; set; }

        [JsonPropertyName("move_y")]
        public float MoveY { get; set; }

        [JsonPropertyName("ascend_descend")]
        public float AscendDescend { get; set; }

        [JsonPropertyName("aim_angle")]
        public float AimAngle { get; set; }

        [JsonPropertyName("shoot")]
        public bool Shoot { get; set; }

        [JsonPropertyName("interact")]
        public bool Interact { get; set; }

        [JsonPropertyName("brake")]
        public bool Brake { get; set; }

    }

    public sealed class Ping : ClientMessage
    {
        [JsonPropertyName("client_ms")]
        public ulong ClientMs { get; set; }

    }

}

