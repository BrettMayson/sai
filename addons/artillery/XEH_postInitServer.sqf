#include "script_component.hpp"

addMissionEventHandler ["ExtensionCallback", {
    params ["_name", "_func", "_data"];
    if (_name != "sai") exitWith {};
    if (_func == "artillery:fire") then {
        private _data = parseSimpleArray _data;
        _data params ["_target", "_round", "_quantity", "_unit", "_spread"];
        [objectFromNetId _unit, _target, _spread, _round, _quantity] call zen_common_fnc_fireArtillery;
    };
}];
