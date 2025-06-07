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
    private _freq = (keys GVAR(commanders)) select ((keys GVAR(commanders)) findIf {
        private _x = GVAR(commanders) get _x;
        _x isEqualTo _data#0
    });
    if (([] call acre_api_fnc_getCurrentRadioList) findIf {
        private _channel = [_x] call acre_api_fnc_getRadioChannel;
        private _f = [[_x] call acre_api_fnc_getBaseRadio, "default", _channel, "frequencyRX"] call acre_api_fnc_getPresetChannelField;
        _f isEqualTo _freq
    } == -1) exitWith {};
    ("sai" callExtension [format["client:speak:%1", _func], [_data#1]]) params ["_ret", "_code"];
}] call CBA_fnc_addEventHandler;

addMissionEventHandler ["ExtensionCallback", {
    params ["_name", "_func", "_data"];
    if (_name != "sai") exitWith {};
    if (_func == "spoke") then {
        private _data = parseSimpleArray _data;
        [QGVAR(spoke), [netId (call CBA_fnc_currentUnit), _data#0, _data#1]] call CBA_fnc_serverEvent;
    };
}];
