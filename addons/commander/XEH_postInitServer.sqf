#include "script_component.hpp"

("sai" callExtension ["server:start", []]) params ["_ret", "_code"];
if (_code != 0) then {
    ERROR_2("Failed to start server extension (%1): %2",_code,_ret);
} else {
    INFO("sai: server extension started");
};

[QGVAR(spoke), {
    params ["_id", "_callsign", "_text"];
    diag_log format ["sai: spoke %1: %2", _id, _text];
    "sai" callExtension ["server:spoke", [_callsign, _text]];
}] call CBA_fnc_addEventHandler;

addMissionEventHandler ["ExtensionCallback", {
    params ["_name", "_func", "_data"];
    if (_name != "sai") exitWith {};
    if (_func == "speak:local") then {
        [QGVAR(speak), ["local", parseSimpleArray _data]] call CBA_fnc_globalEvent;
    };
    if (_func == "speak:openai") then {
        [QGVAR(speak), ["openai", parseSimpleArray _data]] call CBA_fnc_globalEvent;
    };
}];
