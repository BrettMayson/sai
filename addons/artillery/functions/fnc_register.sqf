#include "..\script_component.hpp"

_this call FUNC(set);

[{
    params ["_args", "_handle"];
    if !(alive (_args#1)) exitWith {
        [_handle] call CBA_fnc_removePerFrameHandler;
    };
    _args call FUNC(set);
}, 15, _this] call CBA_fnc_addPerFrameHandler;
