#include "script_component.hpp"

[QGVAR(fire), {
    _this call zen_common_fnc_fireArtillery
}] call CBA_fnc_addEventHandler;
