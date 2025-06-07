#include "..\script_component.hpp"

params ["_callsign", "_freq"];

GVAR(commanders) set [_freq, _callsign];
