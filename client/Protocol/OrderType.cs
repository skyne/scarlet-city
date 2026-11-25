// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol scarlet.city v7

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;


namespace ScarletCity.Protocol;

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum OrderType
{
    Follow,
    Hold,
    MoveOrDriveTo,
    Engage,
}

