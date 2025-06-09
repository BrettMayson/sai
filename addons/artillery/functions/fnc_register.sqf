#include "..\script_component.hpp"

params ["_callsign", "_unit"];

if (_unit getVariable [QGVAR(callsign), ""] isNotEqualTo "") exitWith { false };
_unit setVariable [QGVAR(callsign), _callsign, true];

_this call FUNC(set);

_this set [2, netId _unit];

[{
    params ["_args", "_handle"];
    if !(alive (_args#1)) exitWith {
        [_handle] call CBA_fnc_removePerFrameHandler;
        "sai" callExtension ["server:commander:artillery:remove", [_args#2, _callsign]];
    };
    _args call FUNC(set);
}, 15, _this] call CBA_fnc_addPerFrameHandler;

true
