#include "..\script_component.hpp"

params ["_callsign", "_unit"];

private _id = netId _unit;
private _name = getText (configOf _unit >> "DisplayName");

private _isVLS = _unit call zen_common_fnc_isVLS;

private _cfgMagazines = configFile >> "CfgMagazines";
private _rounds = [];
private _magazines = if (_isVLS) then { magazines _unit } else { getArtilleryAmmo [_unit] };
private _ammo = magazinesAmmo _unit;
{
    private _mag = _x;
    _rounds pushBack [
        _x,
        (getText (_cfgMagazines >> _x >> "displayName")),
        _ammo select (_ammo findIf { _x#0 == _mag}) select 1
    ];
} forEach _magazines;

private _last = _unit getVariable [QGVAR(last), []];
if (_last isEqualTo _rounds) exitWith {};

_unit setVariable [QGVAR(last), _rounds];

"sai" callExtension ["server:commander:artillery:register", [_id, _callsign, _name, _rounds]];
