#include "script_component.hpp"

addMissionEventHandler ["ExtensionCallback", {
    params ["_name", "_func", "_data"];
    if (_name != "sai") exitWith {};
    if (_func == "artillery:fire") exitWith {
        private _data = parseSimpleArray _data;
        _data params ["_target", "_round", "_quantity", "_unit", "_spread"];
        private _unit = objectFromNetId _unit;
        [QGVAR(fire), [_unit, _target, _spread, _round, _quantity], _unit] call CBA_fnc_targetEvent;
    };
    if (_func == "artillery:eta") exitWith {
        private _data = parseSimpleArray _data;
        _data params ["_id", "_target", "_round", "_unit"];
        private _unit = objectFromNetId _unit;
        private _eta = [_unit, _target, _round] call zen_common_fnc_getArtilleryETA;
        "sai" callExtension ["server:response", [_id, _eta]];
    };
}];
