title = Welcome
greeting = Hello, { $user-name }!
items = { $count ->
    [one] One item
   *[other] { $count } items
}
welcome = { greeting } You have { items }.
login-input = Log in
    .placeholder = Your name
    .title = Log in as { $user-name }
only-english = Only in English
type = Type: { $type }
