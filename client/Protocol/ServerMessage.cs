// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v1

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(ServerMessage.Welcome), "Welcome")]
[JsonDerivedType(typeof(ServerMessage.Snapshot), "Snapshot")]
[JsonDerivedType(typeof(ServerMessage.Event), "Event")]
[JsonDerivedType(typeof(ServerMessage.Pong), "Pong")]
public abstract class ServerMessage
{
    public sealed class Welcome : ServerMessage
    {
        [JsonPropertyName("player_id")]
        public Guid PlayerId { get; set; }

        [JsonPropertyName("map")]
        public MapData Map { get; set; } = null!;

    }

    public sealed class Snapshot : ServerMessage
    {
        [JsonPropertyName("tick")]
        public ulong Tick { get; set; }

        [JsonPropertyName("players")]
        public List<PlayerState> Players { get; set; } = new List<PlayerState>();

        [JsonPropertyName("vehicles")]
        public List<VehicleState> Vehicles { get; set; } = new List<VehicleState>();

        [JsonPropertyName("loot_nodes")]
        public List<LootNode> LootNodes { get; set; } = new List<LootNode>();

        [JsonPropertyName("exfil_zones")]
        public List<ExfilZone> ExfilZones { get; set; } = new List<ExfilZone>();

        [JsonPropertyName("projectiles")]
        public List<Projectile> Projectiles { get; set; } = new List<Projectile>();

    }

    public sealed class Event : ServerMessage
    {
        [JsonPropertyName("kind")]
        public string Kind { get; set; } = string.Empty;

        [JsonPropertyName("message")]
        public string Message { get; set; } = string.Empty;

    }

    public sealed class Pong : ServerMessage
    {
        [JsonPropertyName("client_ms")]
        public ulong ClientMs { get; set; }

        [JsonPropertyName("server_tick")]
        public ulong ServerTick { get; set; }

    }

}

