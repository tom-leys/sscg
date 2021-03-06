!@import sscg sscg;
!@import wlambda;
!@import std std;
!@import c   colors;
!@import gui gui_common;
!@import WID gui_window_ids;

!show = {!(STATE, ent, ent_type) = @;
    gui:window WID:STRUCTURE {
        $[ent.name, $[
            gui:hpanel 700 { $[
                gui:ml_l_vtext 1000 1000 c:SE1_L $[
                    "This is an alient structure.",
                    "You can deploy a reusable mining drone,",
                    "that transfers the mined matter to your ship.",
                    "Go back to the ship with the [F] key.",
                ]
            ] },
            gui:hpanel 300 { $[
                gui:action_button 500 1000 :deploy "Deploy Drone",
                gui:button        500 1000 :depart "Depart",
            ] },
        ] ]
    } ${
        deploy = {!(close) = @;
            std:displayln ent;
            sscg:game.gd_call "ship" :deploy_drone ent;
            close[];
        },
    } { STATE.ship.docked = $f; };
};

!@export show show;
