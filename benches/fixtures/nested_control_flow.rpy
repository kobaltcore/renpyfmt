label branchy_start:
    $ mood = "steady"

    if visited_bridge:
        menu:
            "Ask about the bridge":
                if weather == "rain":
                    "The river is loud enough to swallow the conversation."

                elif weather == "fog":
                    "Everything near the water sounds farther away than it is."

                else:
                    "The bridge creaks, but it has not failed us yet."

            "Change the subject":
                "We talk about the bakery instead."

    elif visited_square:
        while patience > 0:
            $ patience -= 1

            if patience == 2:
                "The square feels smaller every time we circle it."

            elif patience == 1:
                "Even the clock tower looks impatient."

            else:
                "At last, we agree to leave."

    else:
        if has_map:
            call map_review
        else:
            jump lost_without_map

label map_review:
    "The map is more annotation than paper at this point."

    return

label lost_without_map:
    "We make progress only by pretending certainty."

    return
