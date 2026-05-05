init python:
    import collections


    class Journal:
        def __init__(self):
            self.entries = collections.defaultdict(list)

        def add(self, day, text):
            normalized = text.strip()
            if normalized:
                self.entries[day].append(normalized)

        def summary(self):
            lines = []
            for day, items in sorted(self.entries.items()):
                lines.append(f"{day}: {', '.join(items)}")
            return "\n".join(lines)

label python_notes:
    python:
        journal = Journal()
        journal.add("day1", "met the courier")
        journal.add("day1", "found the lantern key")
        journal.add("day2", "learned the stage cues")

        if persistent.debug_mode:
            journal.add("debug", "extra instrumentation enabled")

        summary = journal.summary()

    "Notes updated."

    "[summary]"
