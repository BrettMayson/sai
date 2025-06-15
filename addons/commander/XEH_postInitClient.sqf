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
    params ["_data"];
    private _freq = (keys GVAR(commanders)) select ((keys GVAR(commanders)) findIf {
        private _x = GVAR(commanders) get _x;
        _x isEqualTo _data#0
    });
    private _pan = 0.0;
    private _volume = 1.0;
    if (([] call acre_api_fnc_getCurrentRadioList) findIf {
        private _channel = [_x] call acre_api_fnc_getRadioChannel;
        private _f = [[_x] call acre_api_fnc_getBaseRadio, "default", _channel, "frequencyRX"] call acre_api_fnc_getPresetChannelField;
        private _match = _f isEqualTo _freq;
        if (_match) then {
            private _spatial = [_x] call acre_api_fnc_getRadioSpatial;
            if (_spatial == "LEFT") then {
                _pan = -1.0;
            };
            if (_spatial == "RIGHT") then {
                _pan = 1.0;
            };
            _volume = [_x] call acre_api_fnc_getRadioVolume;
        };
        _match
    } == -1) exitWith {};
    ("sai" callExtension ["client:speak", [_data#1, _pan, _volume]]) params ["_ret", "_code"];
}] call CBA_fnc_addEventHandler;

addMissionEventHandler ["ExtensionCallback", {
    params ["_name", "_func", "_data"];
    if (_name != "sai") exitWith {};
    if (_func == "spoke") then {
        private _data = parseSimpleArray _data;
        [QGVAR(spoke), [netId (call CBA_fnc_currentUnit), _data#0, _data#1]] call CBA_fnc_serverEvent;
    };
}];
