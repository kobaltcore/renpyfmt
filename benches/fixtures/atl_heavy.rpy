image logo pulse:
    "gui/logo.png"
    alpha 0.0
    linear 0.3 alpha 1.0
    parallel:
        ease 1.2 yoffset -10
        ease 1.2 yoffset 0
        repeat
    parallel:
        ease 0.6 zoom 1.02
        ease 0.6 zoom 1.0
        repeat

label atl_showcase:
    scene bg stage

    show logo pulse:
        xalign 0.5
        yalign 0.4
        ease 0.5 rotate 2
        ease 0.5 rotate -2
        repeat

    with dissolve

    show eileen happy at center:
        pause 0.2
        block:
            ease 0.5 xoffset 12
            ease 0.5 xoffset 0
        repeat

    "The sign refuses to stay still."
