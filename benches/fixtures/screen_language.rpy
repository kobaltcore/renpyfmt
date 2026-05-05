screen inventory_panel(items, selected=None):
    tag menu
    modal True
    frame:
        style_prefix "inventory"
        xalign 0.5
        yalign 0.5
        vbox:
            spacing 12
            text "Inventory":
                style "inventory_title"
            if not items:
                text "Nothing collected yet."
            else:
                for item in items:
                    button:
                        action Return(item.name)
                        spacing 8
                        has hbox
                        if item.icon:
                            add item.icon
                        vbox:
                            text item.name
                            if item.description:
                                text item.description:
                                    size 20
            if selected:
                frame:
                    background "#112233"
                    vbox:
                        text selected.name
                        text selected.description
                        textbutton "Use":
                            action Function(use_item, selected)
                        textbutton "Close":
                            action Hide("inventory_panel")
