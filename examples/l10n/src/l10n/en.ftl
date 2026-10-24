title = Localization
# This language's own name, used by the language links.
language = English
intro = This page is translated with Fluent.
greeting = Hello, { $name }!
# Picks a form based on `$count`. `*` marks the default.
items =
    { $count ->
        [one] One item
       *[other] { $count } items
    }
# `DATETIME` formats a date for the locale. `dateStyle: "full"` includes the weekday.
today = Today is { DATETIME($date, dateStyle: "full") }.
# `.placeholder` is an attribute: an extra text that belongs to the message.
search = Search
    .placeholder = Type to search
