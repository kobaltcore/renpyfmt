"""renpy
label generated_entry:
    "This fixture exercises _ren.py logical line extraction."
    python:
        score = 0
        for step in range(5):
            score += step
    "Score: [score]"
"""


def helper(value):
    return value * 2


class Utility:
    def compute(self, items):
        total = 0
        for item in items:
            total += helper(item)
        return total
