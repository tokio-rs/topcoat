title = Localization
# The name of the language, in that language.
language = English
intro = This page is translated with Fluent.
greeting = Hello, { $name }!
# Picks one of the variants below based on `$count`. The one marked with `*` is used when
# no other one matches.
items =
    { $count ->
        [one] One item
       *[other] { $count } items
    }
# `.placeholder` is an extra part of the message, used here for the placeholder of the
# search field.
search = Search
    .placeholder = Type to search
