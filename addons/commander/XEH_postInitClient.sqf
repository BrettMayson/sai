#include "script_component.hpp"

GVAR(commanders) = createHashMap;
GVAR(radio) = "";

["acre_startedSpeaking", {
    params ["_unit", "_onRadio", "_radio", "_type"];
    if (!_onRadio) exitWith {};
    if (_unit != (call CBA_fnc_currentUnit)) exitWith {};
    GVAR(radio) = _radio;
    "sai" callExtension ["client:start", []];
}] call CBA_fnc_addEventHandler;

["acre_stoppedSpeaking", {
    params ["_unit", "_onRadio"];
    if (!_onRadio) exitWith {};
    if (_unit != (call CBA_fnc_currentUnit)) exitWith {};
    private _channel = [GVAR(radio)] call acre_api_fnc_getRadioChannel;
    private _freq = [[GVAR(radio)] call acre_api_fnc_getBaseRadio, "default", _channel, "frequencyRX"] call acre_api_fnc_getPresetChannelField;
    private _callsign = GVAR(commanders) getOrDefault [_freq, ""];
    if (_callsign isEqualTo "") exitWith {};
    "sai" callExtension ["client:stop", [_callsign]];
}] call CBA_fnc_addEventHandler;

[QGVAR(speak), {
    params ["_func", "_data"];
    ("sai" callExtension [format["client:speak:%1", _func], [_data]]) params ["_ret", "_code"];
}] call CBA_fnc_addEventHandler;

addMissionEventHandler ["ExtensionCallback", {
    params ["_name", "_func", "_data"];
    if (_name != "sai") exitWith {};
    if (_func == "spoke") then {
        private _data = parseSimpleArray _data;
        [QGVAR(spoke), [netId (call CBA_fnc_currentUnit), _data#0, _data#1]] call CBA_fnc_serverEvent;
    };
}];
