# Ein Betrag in US-Dollar, etwa 1234.5. Das geschützte Leerzeichen hält Betrag
# und Währung in einer Zeile.
money = { NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2) }{ "\u00A0" }$
# Ein Betrag in einem Formularfeld, etwa 1234,50. Ohne W\u00E4hrungssymbol und
# Tausendertrennzeichen, damit die App ihn wieder einlesen kann.
money_input = { NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2, useGrouping: "false") }

role_admin = Admin
role_staff = Mitarbeiter
role_viewer = Betrachter
