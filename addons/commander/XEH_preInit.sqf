#include "script_component.hpp"
ADDON = false;
#include "XEH_PREP.hpp"

[QGVAR(host), "EDITBOX", "Host", "SAI", "http://localhost:8521", 1, {
    params ["_value"];
    "sai" callExtension ["settings:set", ["HOST", _value]];
}] call CBA_fnc_addSetting;

ADDON = true;
